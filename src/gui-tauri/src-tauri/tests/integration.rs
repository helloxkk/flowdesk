// End-to-end integration test: write a barrier-format config directly,
// spawn the real barriers binary, verify it starts listening, then shut
// it down. This validates the config format our Rust serializer emits by
// exercising the C++ core's actual parser.
//
// Skipped automatically if the barriers binary isn't present, so `cargo
// test` still passes in bare environments (CI without a C++ build).
//
// Copyright (C) 2026 helloxkk (FlowDesk)
// Licensed under GPLv2.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn barriers_binary() -> Option<PathBuf> {
    let candidates: [&str; 3] = [
        "../../../build/bin/barriers",
        "/Applications/Barrier.app/Contents/MacOS/barriers",
        "barriers",
    ];
    for c in candidates {
        let p = PathBuf::from(c);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

/// A config string exactly like what config.rs::to_barrier_config emits
/// for a two-screen side-by-side layout. Keeping it inline here lets the
/// test run without depending on the crate's private API, while still
/// validating the exact wire format the C++ core must parse.
const SAMPLE_CONFIG: &str = "\
# FlowDesk server configuration
section: screens
    server:
    client:
end

section: links
    server:
        right = client
    client:
        left = server
end

section: options
    screenSaverSync = true
    win32KeepForeground = true
    clipboardSharing = true
end
";

#[test]
fn barriers_starts_with_generated_config() {
    let bin = match barriers_binary() {
        Some(b) => b,
        None => {
            eprintln!("skipping: barriers binary not found");
            return;
        }
    };
    eprintln!("using barriers: {}", bin.display());

    // Write to a temp file in the OS temp dir.
    let mut tmp_path = std::env::temp_dir();
    tmp_path.push(format!("flowdesk-it-{}.conf", std::process::id()));
    let mut f = std::fs::File::create(&tmp_path).expect("create temp");
    f.write_all(SAMPLE_CONFIG.as_bytes()).expect("write");
    eprintln!("config at: {}", tmp_path.display());

    // Pick a non-default port so we don't collide with a running Barrier.
    let port = 24999u16;
    let address = format!("127.0.0.1:{port}");

    let mut child = Command::new(&bin)
        .args([
            "-f",
            "--no-tray",
            "--debug",
            "INFO",
            "--name",
            "server",
            "-c",
            tmp_path.to_str().unwrap(),
            "--address",
            &address,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn barriers");

    let stdout = child.stdout.take().expect("stdout");
    use std::io::{BufRead, BufReader};
    let lines = BufReader::new(stdout).lines();

    let (tx, rx) = std::sync::mpsc::channel::<String>();
    std::thread::spawn(move || {
        for line in lines.flatten() {
            if tx.send(line).is_err() {
                break;
            }
        }
    });

    let deadline = Instant::now() + Duration::from_secs(10);
    let mut saw_started = false;
    while Instant::now() < deadline {
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(line) => {
                eprintln!("barriers> {}", line);
                if line.contains("started server") {
                    saw_started = true;
                    break;
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            Err(_) => continue,
        }
    }

    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_file(&tmp_path);

    assert!(
        saw_started,
        "barriers did not report 'started server' within 10s; config format may be wrong"
    );
}
