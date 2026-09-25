//! Reads the `$MFT` file from a volume in big sequential chunks and hands out records.

use fazasanj_platform::Extent;

/// Anything we can read raw bytes from at an offset (the volume, or a test image).
pub trait VolumeRead {
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, String>;
}

#[derive(Debug, Clone)]
pub struct MftLayout {
    pub cluster_size: u64,
    pub record_size: usize,
    /// Bytes of `$MFT` that hold real records.
    pub valid_len: u64,
    /// Extents of `$MFT`, in VCN order.
    pub extents: Vec<Extent>,
}

/// About 4 MB per read. Always a multiple of the cluster size and the record size.
fn chunk_size(layout: &MftLayout) -> u64 {
    let unit = layout.cluster_size.max(layout.record_size as u64).max(512);
    (4 << 20) / unit * unit
}

/// Calls `on_record(record_number, bytes)` for every record, in order. `progress(done, total)`
/// is called after each chunk and can return false to stop.
pub fn read_mft(
    vol: &impl VolumeRead,
    layout: &MftLayout,
    progress: &mut impl FnMut(u64, u64) -> bool,
    on_record: &mut impl FnMut(u64, &mut [u8]),
) -> Result<(), String> {
    let rs = layout.record_size;
    if rs == 0 || layout.cluster_size == 0 {
        return Err("bad volume geometry".into());
    }
    let total_records = layout.valid_len / rs as u64;
    let chunk = chunk_size(layout);
    let mut buf = vec![0u8; chunk as usize];
    // Bytes of a record that started at the end of the previous extent.
    let mut carry: Vec<u8> = Vec::new();
    let mut next_rec = 0u64;
    let mut extents = layout.extents.clone();
    extents.sort_by_key(|e| e.vcn);

    for e in &extents {
        let ext_start = e.vcn * layout.cluster_size;
        if ext_start >= layout.valid_len {
            break;
        }
        let ext_len = (e.clusters * layout.cluster_size).min(layout.valid_len - ext_start);
        let mut done = 0u64;
        while done < ext_len {
            let n = chunk.min(ext_len - done) as usize;
            let data = &mut buf[..n];
            match e.lcn {
                Some(lcn) => {
                    let off = lcn * layout.cluster_size + done;
                    let got = vol.read_at(off, data)?;
                    if got < n {
                        data[got..].fill(0);
                    }
                }
                // A hole: nothing on disk, records here are unused.
                None => data.fill(0),
            }
            done += n as u64;

            let mut slice: &mut [u8] = data;
            if !carry.is_empty() {
                let need = rs - carry.len();
                let take = need.min(slice.len());
                carry.extend_from_slice(&slice[..take]);
                slice = &mut slice[take..];
                if carry.len() == rs {
                    on_record(next_rec, &mut carry);
                    next_rec += 1;
                    carry.clear();
                }
            }
            let whole = slice.len() / rs * rs;
            let (records, rest) = slice.split_at_mut(whole);
            for r in records.chunks_exact_mut(rs) {
                on_record(next_rec, r);
                next_rec += 1;
            }
            carry.extend_from_slice(rest);
            if !progress(next_rec, total_records) {
                return Err("stopped".into());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Image(Vec<u8>);

    impl VolumeRead for Image {
        fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, String> {
            let start = offset as usize;
            let end = (start + buf.len()).min(self.0.len());
            let src = self.0.get(start..end).ok_or("out of range")?;
            buf[..src.len()].copy_from_slice(src);
            Ok(src.len())
        }
    }

    #[test]
    fn records_across_fragmented_extents() {
        // 512 byte clusters, 1024 byte records, so records straddle extents.
        let cluster = 512u64;
        let mut img = vec![0u8; 64 * 512];
        // $MFT has 6 clusters (3 records): clusters 10, then 20..22, then 40..41.
        let mut write_cluster = |lcn: usize, tag: u8| img[lcn * 512..lcn * 512 + 512].fill(tag);
        for (i, lcn) in [10, 20, 21, 22, 40, 41].into_iter().enumerate() {
            write_cluster(lcn, i as u8 + 1);
        }
        let layout = MftLayout {
            cluster_size: cluster,
            record_size: 1024,
            valid_len: 6 * 512,
            extents: vec![
                Extent { vcn: 1, lcn: Some(20), clusters: 3 },
                Extent { vcn: 0, lcn: Some(10), clusters: 1 },
                Extent { vcn: 4, lcn: Some(40), clusters: 2 },
            ],
        };
        let mut seen = Vec::new();
        let mut last = (0, 0);
        read_mft(
            &Image(img),
            &layout,
            &mut |d, t| {
                last = (d, t);
                true
            },
            &mut |n, r| seen.push((n, r[0], r[1023])),
        )
        .expect("read");
        assert_eq!(seen, vec![(0, 1, 2), (1, 3, 4), (2, 5, 6)]);
        assert_eq!(last, (3, 3));
    }

    #[test]
    fn stops_when_asked() {
        let layout = MftLayout {
            cluster_size: 4096,
            record_size: 1024,
            valid_len: 4096,
            extents: vec![Extent { vcn: 0, lcn: None, clusters: 1 }],
        };
        let r = read_mft(&Image(vec![]), &layout, &mut |_, _| false, &mut |_, _| {});
        assert!(r.is_err());
    }
}
