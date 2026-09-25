//! Runs the real helper binary against a real pipe, without UAC. Not elevated, the helper can
//! not open the volume, so it must say hello with our token and then send a clean error.
//! If the test runs elevated it reads the MFT (read only) and we just check records arrive.

use std::process::Command;
use std::time::Duration;

use fazasanj_platform::{is_elevated, random_hex, PipeServer};
use fazasanj_scan::wire::{Frame, FrameDecoder, ERR_OPEN_VOLUME, VERSION};

#[test]
fn helper_handshake_over_pipe() {
    let name = format!("fazasanj-{}", random_hex(16).expect("rand"));
    let token = random_hex(32).expect("rand");
    let server = PipeServer::create(&name, false).expect("pipe");
    let mut child = Command::new(env!("CARGO_BIN_EXE_fast-scan-helper"))
        .args(["--volume", "C:", "--pipe", &name, "--token", &token])
        .spawn()
        .expect("spawn helper");

    server.wait_for_client(Duration::from_secs(10), &|| false).expect("helper connects");
    let mut dec = FrameDecoder::default();
    let mut buf = vec![0u8; 64 * 1024];
    let mut frames = Vec::new();
    'read: loop {
        let n = server.read(&mut buf, Duration::from_secs(30), &|| false).expect("read");
        if n == 0 {
            break;
        }
        dec.push(&buf[..n]);
        while let Some(f) = dec.next_frame().expect("frame") {
            let stop = matches!(f, Frame::Records { .. } | Frame::Error { .. } | Frame::Done { .. });
            frames.push(f);
            if stop {
                break 'read;
            }
        }
    }
    drop(server);
    let status = child.wait().expect("wait");

    assert_eq!(frames[0], Frame::Hello { version: VERSION, token });
    if is_elevated() {
        assert!(matches!(frames.last(), Some(Frame::Records { .. } | Frame::Done { .. })));
    } else {
        assert!(matches!(frames.last(), Some(Frame::Error { code: ERR_OPEN_VOLUME, .. })), "{frames:?}");
        assert_eq!(status.code(), Some(4));
    }
}

#[test]
fn helper_rejects_bad_pipe_name() {
    let out = Command::new(env!("CARGO_BIN_EXE_fast-scan-helper"))
        .args(["--volume", "C:", "--pipe", r"..\..\x", "--token", "00112233445566778899aabbccddeeff"])
        .output()
        .expect("run helper");
    assert_eq!(out.status.code(), Some(2));
}
