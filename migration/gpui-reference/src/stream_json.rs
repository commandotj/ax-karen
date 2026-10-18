use std::io::{self, BufRead, Write};

use anyhow::Result;
use monitor_disk::scan::{ScanHandle, ScanOptions};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use serde_json::{Value, json};

pub fn run() -> Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_stream(stdin.lock(), stdout.lock())
}

fn run_stream(mut input: impl BufRead, mut output: impl Write) -> Result<()> {
    write_event(&mut output, json!({"version": 1, "event": "ready"}))?;

    let mut line = String::new();
    loop {
        line.clear();
        if input.read_line(&mut line)? == 0 {
            break;
        }
        if line.trim().is_empty() {
            continue;
        }

        let shutdown = handle_request(&line, &mut output)?;
        if shutdown {
            break;
        }
    }
    Ok(())
}

fn handle_request(line: &str, output: &mut impl Write) -> Result<bool> {
    let request: Value = match serde_json::from_str(line) {
        Ok(request) => request,
        Err(_) => {
            write_event(output, error_event(None, "invalid_json"))?;
            return Ok(false);
        }
    };
    let id = request.get("id").cloned();
    if request["version"] != 1 {
        write_event(output, error_event(id, "unsupported_version"))?;
        return Ok(false);
    }
    if !id.as_ref().is_some_and(Value::is_string) {
        write_event(output, error_event(None, "invalid_request"))?;
        return Ok(false);
    }
    if request["command"] == "shutdown" {
        write_event(output, json!({"version": 1, "id": id, "event": "shutdown"}))?;
        return Ok(true);
    }
    if request["command"] == "scan" {
        scan_request(id, request.get("root"), output)?;
        return Ok(false);
    }
    write_event(output, error_event(id, "unknown_command"))?;
    Ok(false)
}

fn scan_request(id: Option<Value>, root: Option<&Value>, output: &mut impl Write) -> Result<()> {
    let Some(root) = root.and_then(Value::as_str) else {
        write_event(output, error_event(id, "invalid_root"))?;
        return Ok(());
    };
    let scan = ScanHandle::spawn(PathBuf::from(root), ScanOptions::default());
    let mut previous = (0, 0, 0, 0);
    loop {
        if let Some(result) = scan.poll() {
            match result {
                Ok(tree) => write_event(output, json!({
                    "version": 1,
                    "id": id,
                    "event": "scan_complete",
                    "files": tree.files,
                    "dirs": tree.dirs,
                    "bytes": tree.bytes
                }))?,
                Err(error) => write_event(output, json!({
                    "version": 1,
                    "id": id,
                    "event": "error",
                    "code": "scan_failed",
                    "message": error.to_string()
                }))?,
            }
            return Ok(());
        }
        let snapshot = scan.progress.snapshot();
        let current = (snapshot.files, snapshot.dirs, snapshot.bytes, snapshot.errors);
        if current != previous {
            write_event(output, json!({
                "version": 1,
                "id": id,
                "event": "scan_progress",
                "files": snapshot.files,
                "dirs": snapshot.dirs,
                "bytes": snapshot.bytes,
                "errors": snapshot.errors
            }))?;
            previous = current;
        }
        thread::sleep(Duration::from_millis(25));
    }
}

fn error_event(id: Option<Value>, code: &str) -> Value {
    json!({"version": 1, "id": id, "event": "error", "code": code})
}

fn write_event(output: &mut impl Write, event: Value) -> Result<()> {
    serde_json::to_writer(&mut *output, &event)?;
    output.write_all(b"\n")?;
    output.flush()?;
    Ok(())
}
