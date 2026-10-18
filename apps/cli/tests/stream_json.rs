use std::fs;
use std::io::{BufRead, BufReader, Write};
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
    assert_eq!(
        events[0],
        serde_json::json!({"version": 1, "event": "ready"})
    );
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

#[test]
fn stream_json_scan_honors_hidden_file_filter_request() {
    let root = tempfile::tempdir().expect("create scan root");
    fs::write(root.path().join("visible.txt"), "visible")
        .expect("create visible file");
    fs::write(root.path().join(".hidden.txt"), "hidden")
        .expect("create hidden file");
    let events = run_stream(&[
        serde_json::json!({
            "version": 1,
            "id": "scan-filtered",
            "command": "scan",
            "root": root.path().to_string_lossy(),
            "include_hidden": false
        }),
        serde_json::json!({"version":1,"id":"exit","command":"shutdown"}),
    ]);

    assert!(events.iter().any(|event| {
        event["id"] == "scan-filtered"
            && event["event"] == "scan_complete"
            && event["files"] == 1
    }));
}

#[test]
fn resources_keep_existing_fields_and_add_bounded_process_rows() {
    let events = run_stream(&[
        serde_json::json!({
            "version": 1,
            "id": "resources-1",
            "command": "resources"
        }),
        serde_json::json!({
            "version": 1,
            "id": "resources-2",
            "command": "resources"
        }),
        serde_json::json!({
            "version": 1,
            "id": "exit",
            "command": "shutdown"
        }),
    ]);
    let resources: Vec<_> = events
        .iter()
        .filter(|event| event["event"] == "resources")
        .collect();

    assert_eq!(resources.len(), 2);
    assert!(resources.iter().all(|event| {
        event.get("cpu_percent").is_some()
            && event.get("memory_used_bytes").is_some()
            && event.get("memory_total_bytes").is_some()
            && event.get("memory_processes_available").is_some()
    }));
    assert_eq!(resources[0]["cpu_processes_available"], false);
    assert!(resources[0]["cpu_processes"].is_null());
    assert_eq!(resources[1]["cpu_processes_available"], true);
    let processes = resources[1]["cpu_processes"]
        .as_array()
        .expect("warmed CPU process list");
    assert!(processes.len() <= 20);
    assert!(processes.iter().all(|process| {
        process["pid"].is_u64()
            && process["name"].is_string()
            && process["cpu_percent"].is_number()
    }));
    for event in &resources {
        let memory_processes = event["memory_processes"]
            .as_array()
            .expect("memory process list");
        assert!(memory_processes.len() <= 20);
        assert!(memory_processes.iter().all(|process| {
            process["pid"].is_u64()
                && process["name"].is_string()
                && process["memory_bytes"].as_u64().is_some()
        }));
        assert!(memory_processes.windows(2).all(|pair| {
            let prior_bytes = pair[0]["memory_bytes"].as_u64().unwrap_or(0);
            let next_bytes = pair[1]["memory_bytes"].as_u64().unwrap_or(0);
            let prior_pid = pair[0]["pid"].as_u64().unwrap_or(u64::MAX);
            let next_pid = pair[1]["pid"].as_u64().unwrap_or(u64::MAX);
            prior_bytes > next_bytes
                || (prior_bytes == next_bytes && prior_pid <= next_pid)
        }));
    }
}

#[test]
fn cached_tree_navigation_marks_and_review_stay_in_cli_without_rescan() {
    let root = tempfile::tempdir().expect("create tree root");
    let folder = root.path().join("large-folder");
    fs::create_dir(&folder).expect("create nested directory");
    fs::write(folder.join("blob.bin"), vec![7_u8; 4096]).expect("write file");
    let root_string = root.path().to_string_lossy().into_owned();
    let file_string = folder.join("blob.bin").to_string_lossy().into_owned();
    let mut child = Command::new(env!("CARGO_BIN_EXE_karen"))
        .arg("--stream-json")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start Karen CLI");
    let mut input = child.stdin.take().expect("CLI stdin");
    let stdout = child.stdout.take().expect("CLI stdout");
    let mut output = BufReader::new(stdout);
    let mut next_event = || {
        let mut line = String::new();
        output.read_line(&mut line).expect("read CLI event");
        serde_json::from_str::<Value>(&line).expect("parse CLI event")
    };
    assert_eq!(next_event()["event"], "ready");
    send_request(
        &mut input,
        serde_json::json!({"id":"scan","command":"scan","root":root_string}),
    );
    let mut scan_complete = false;
    while !scan_complete {
        scan_complete = next_event()["event"] == "scan_complete";
    }
    send_request(
        &mut input,
        serde_json::json!({"id":"view","command":"tree_view","root":root_string,"path":root_string,"width":800,"height":500}),
    );
    let view = read_event_for(&mut next_event, "view");
    assert_eq!(view["event"], "tree_view");
    assert_eq!(view["tiles"][0]["name"], "large-folder");
    assert!(view["tiles"][0]["header"]["height"].as_f64().unwrap_or(0.0) > 0.0);
    assert!(view["tiles"][0]["x"].as_f64().unwrap_or(-1.0) >= 0.0);
    assert!(view["tiles"][0]["y"].as_f64().unwrap_or(-1.0) >= 0.0);
    assert!(view["tiles"][0]["width"].as_f64().unwrap_or(0.0) <= 800.0);
    assert!(view["tiles"][0]["height"].as_f64().unwrap_or(0.0) <= 500.0);
    assert_eq!(view["tiles"][0]["files"], 1);
    assert_eq!(view["tiles"][0]["dirs"], 0);
    assert!(view["tiles"][0]["direct_bytes"].as_u64().is_some());
    assert_eq!(view["tiles"][0]["parent_bytes"], view["bytes"]);
    let nested_path = folder.to_string_lossy().into_owned();
    send_request(
        &mut input,
        serde_json::json!({
            "id":"select-folder","command":"tree_view","root":root_string,
            "path":nested_path,"open":false,"metric":"files","depth":1
        }),
    );
    let selected = read_event_for(&mut next_event, "select-folder");
    assert_eq!(selected["selection"]["path"], nested_path);
    assert_eq!(selected["breadcrumbs"].as_array().map(Vec::len), Some(1));
    assert_eq!(selected["metric"], "files");
    assert_eq!(selected["depth"], 1);
    assert_eq!(selected["tiles"][0]["name"], "large-folder");
    send_request(
        &mut input,
        serde_json::json!({"id":"nested","command":"tree_view","root":root_string,"path":nested_path}),
    );
    let nested = read_event_for(&mut next_event, "nested");
    assert_eq!(nested["event"], "tree_view");
    assert_eq!(nested["breadcrumbs"].as_array().map(Vec::len), Some(2));
    send_request(
        &mut input,
        serde_json::json!({"id":"return","command":"tree_view","root":root_string,"path":root_string}),
    );
    assert_eq!(
        read_event_for(&mut next_event, "return")["event"],
        "tree_view"
    );
    send_request(
        &mut input,
        serde_json::json!({"id":"mark","command":"toggle_mark","root":root_string,"path":file_string}),
    );
    assert_eq!(
        read_event_for(&mut next_event, "mark")["event"],
        "mark_changed"
    );
    send_request(
        &mut input,
        serde_json::json!({"id":"review","command":"review","root":root_string}),
    );
    let review = read_event_for(&mut next_event, "review");
    assert_eq!(review["event"], "review");
    assert_eq!(review["targets"], 1);
    assert!(review["reclaim_bytes"].as_u64().is_some());
    send_request(
        &mut input,
        serde_json::json!({"id":"exit","command":"shutdown"}),
    );
    assert_eq!(read_event_for(&mut next_event, "exit")["event"], "shutdown");
    drop(input);
    let result = child.wait_with_output().expect("wait for Karen CLI");
    assert!(result.status.success());
}

fn send_request(input: &mut impl Write, request: Value) {
    let mut request = request;
    request["version"] = serde_json::json!(1);
    serde_json::to_writer(&mut *input, &request).expect("write request");
    input.write_all(b"\n").expect("finish request line");
    input.flush().expect("flush request");
}

fn read_event_for(next_event: &mut impl FnMut() -> Value, id: &str) -> Value {
    loop {
        let event = next_event();
        if event["id"] == id {
            return event;
        }
    }
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
