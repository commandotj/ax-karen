use std::fs;
use std::io::Write as _;
use std::process::{Command, Stdio};

use serde_json::Value;

#[test]
fn stream_json_emits_ready_then_shutdown_event() {
    let events = run_stream(&[serde_json::json!({
        "version": 1,
        "id": "exit",
        "command": "shutdown"
    })]);

    assert_eq!(events.len(), 2);
    assert_eq!(events[0], serde_json::json!({"version": 1, "event": "ready"}));
    assert_eq!(
        events[1],
        serde_json::json!({"version": 1, "id": "exit", "event": "shutdown"})
    );
}

#[test]
fn stream_json_scan_reports_real_file_count() {
    let root = tempfile::tempdir().expect("create scan root");
    fs::write(root.path().join("note.txt"), "hello").expect("create file");
    let events = run_stream(&[
        serde_json::json!({
            "version": 1,
            "id": "scan-1",
            "command": "scan",
            "root": root.path().to_string_lossy()
        }),
        serde_json::json!({
            "version": 1,
            "id": "exit",
            "command": "shutdown"
        }),
    ]);

    assert!(events.iter().any(|event| {
        event["id"] == "scan-1"
            && event["event"] == "scan_complete"
            && event["files"] == 1
    }));
}

fn run_stream(requests: &[Value]) -> Vec<Value> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_karen"))
        .arg("--stream-json")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start Karen CLI");

    let mut stdin = child.stdin.take().expect("CLI stdin");
    for request in requests {
        serde_json::to_writer(&mut stdin, request).expect("write request");
        stdin.write_all(b"\n").expect("finish request line");
    }
    drop(stdin);
    let output = child.wait_with_output().expect("wait for Karen CLI");

    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events: Vec<Value> = String::from_utf8(output.stdout)
        .expect("JSON stream is UTF-8")
        .lines()
        .map(|line| serde_json::from_str(line).expect("each line is JSON"))
        .collect();
    events
}
