//! End to end without a real volume: synthetic MFT image, helper streaming, wire decoding, and
//! the app side ingest building the same tree the app would show.

use fazasanj_platform::Extent;
use fazasanj_scan::ingest::Ingest;

use crate::mft::{MftLayout, VolumeRead};
use crate::record::test_util::RecordBuilder;
use crate::record::ATTR_ATTRIBUTE_LIST;
use crate::stream::stream_mft;
use crate::wire::{encode_done, encode_hello};
// Decode with the app's copy of the wire module, so this also proves both sides agree.
use fazasanj_scan::wire::{records, Frame, FrameDecoder};

const CLUSTER: u64 = 4096;
const RECORD: usize = 1024;
const MFT_LCN: u64 = 2;
const IN_USE: u16 = 1;
const DIR: u16 = 3;

struct Image(Vec<u8>);

impl VolumeRead for Image {
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, String> {
        let s = offset as usize;
        let e = (s + buf.len()).min(self.0.len());
        let src = self.0.get(s..e).ok_or("out of range")?;
        buf[..src.len()].copy_from_slice(src);
        Ok(src.len())
    }
}

fn put(img: &mut [u8], rec: usize, bytes: &[u8]) {
    let off = MFT_LCN as usize * CLUSTER as usize + rec * RECORD;
    img[off..off + RECORD].copy_from_slice(bytes);
}

fn image() -> (Image, MftLayout) {
    let records = 64;
    let mut img = vec![0u8; (MFT_LCN as usize + 16) * CLUSTER as usize];
    let ft = 116_444_736_000_000_000i64 + 10_000 * 1_000;
    put(&mut img, 0, &RecordBuilder::new(RECORD, IN_USE, 0).std_info(ft, 6).file_name(5, "$MFT", 3).nonresident_data(0, 1 << 26, None, 0).build());
    put(&mut img, 5, &RecordBuilder::new(RECORD, DIR, 0).std_info(ft, 0x16).file_name(5, ".", 3).build());
    put(&mut img, 11, &RecordBuilder::new(RECORD, DIR, 0).std_info(ft, 0x16).file_name(5, "$Extend", 3).build());
    put(&mut img, 16, &RecordBuilder::new(RECORD, DIR, 0).std_info(ft, 0x10).file_name(5, "Windows", 3).build());
    put(&mut img, 17, &RecordBuilder::new(RECORD, DIR, 0).std_info(ft, 0x10).file_name(16, "System32", 3).build());
    put(&mut img, 18, &RecordBuilder::new(RECORD, DIR, 0).std_info(ft, 0x10).file_name(16, "WinSxS", 3).build());
    put(&mut img, 19, &RecordBuilder::new(RECORD, IN_USE, 0).std_info(ft, 0x20).file_name(11, "$UsnJrnl", 0).nonresident_data(0, 1 << 30, None, 0).build());
    put(
        &mut img,
        20,
        &RecordBuilder::new(RECORD, IN_USE, 0)
            .std_info(ft, 0x20)
            .file_name(18, "kernel.dll", 0)
            .file_name(17, "kernel.dll", 3)
            .nonresident_data(0, 8192, None, 0)
            .build(),
    );
    put(
        &mut img,
        21,
        &RecordBuilder::new(RECORD, IN_USE, 0)
            .std_info(ft, 0x20)
            .resident(ATTR_ATTRIBUTE_LIST, 0, &[0u8; 64])
            .file_name(5, "huge.iso", 1)
            .build(),
    );
    put(&mut img, 22, &RecordBuilder::new(RECORD, IN_USE, 21).nonresident_data(0, 5 << 30, None, 0).build());
    put(&mut img, 23, &RecordBuilder::new(RECORD, 0, 0).std_info(ft, 0x20).file_name(5, "deleted.txt", 1).build());
    let mut torn = RecordBuilder::new(RECORD, IN_USE, 0).std_info(ft, 0x20).file_name(5, "torn.bin", 1).build();
    torn[510] ^= 0xFF;
    put(&mut img, 24, &torn);
    put(
        &mut img,
        25,
        &RecordBuilder::new(RECORD, IN_USE, 0)
            .std_info(ft, 0x20 | 0x0040_0000)
            .file_name(5, "cloud.docx", 1)
            .nonresident_data(0, 4096, None, 0)
            .build(),
    );
    let layout = MftLayout {
        cluster_size: CLUSTER,
        record_size: RECORD,
        valid_len: (records * RECORD) as u64,
        extents: vec![Extent { vcn: 0, lcn: Some(MFT_LCN), clusters: 16 }],
    };
    (Image(img), layout)
}

#[test]
fn image_to_tree() {
    let (img, layout) = image();
    let mut stream = Vec::new();
    encode_hello(&mut stream, "tok");
    let sent = stream_mft(&img, &layout, &mut stream).expect("stream");
    encode_done(&mut stream, sent);
    // root, Windows, System32, WinSxS, 2 kernel.dll links, huge.iso, cloud.docx
    assert_eq!(sent, 8);

    let mut d = FrameDecoder::default();
    d.push(&stream);
    let mut ingest = Ingest::new();
    let mut done = false;
    while let Some(f) = d.next_frame().expect("frame") {
        match f {
            Frame::Records { count, payload } => {
                for r in records(count, &payload) {
                    ingest.add(&r.expect("record"));
                }
            }
            Frame::Done { records } => {
                assert_eq!(records, 8);
                done = true;
            }
            _ => {}
        }
    }
    assert!(done);
    let t = ingest.finish(&[], &[], r"C:\".into()).expect("tree");
    let s = t.summary();
    assert_eq!(s.total_bytes, 8192 + (5 << 30));
    assert_eq!(s.files, 4);
    assert_eq!(s.dirs, 3);
    assert_eq!(s.hardlink_dups, 1);
    assert_eq!(s.cloud_only, 1);
    assert!(t.find_by_path(r"C:\$MFT").is_none());
    assert!(t.find_by_path(r"C:\torn.bin").is_none());
    assert!(t.find_by_path(r"C:\deleted.txt").is_none());
    let sys32 = t.find_by_path(r"C:\Windows\System32").expect("system32");
    assert_eq!(t.node(sys32).map(|n| n.total_size), Some(8192));
    let iso = t.find_by_path(r"C:\huge.iso").expect("iso");
    assert_eq!(t.node_info(iso).map(|i| (i.size, i.modified)), Some((5 << 30, Some(1_000))));
}
