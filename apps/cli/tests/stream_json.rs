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

#[test]
fn scan_power_preset_reports_workers_and_completion_stats() {
    let root = tempfile::tempdir().expect("create scan root");
    fs::write(root.path().join("note.txt"), "hello").expect("create file");
    let events = run_stream(&[
        serde_json::json!({
            "version": 1, "id": "scan", "command": "scan",
            "root": root.path().to_string_lossy(), "power": "miser"
        }),
        serde_json::json!({"version":1,"id":"exit","command":"shutdown"}),
    ]);
    let started = events
        .iter()
        .find(|event| event["event"] == "scan_started")
        .expect("scan_started");
    let workers = started["workers"].as_u64().expect("workers count");
    assert!((1..=2).contains(&workers));
    let complete = events
        .iter()
        .find(|event| event["event"] == "scan_complete")
        .expect("scan_complete");
    assert_eq!(complete["unreadable"], 0);
    assert!(complete["elapsed_ms"].as_u64().is_some());
}

#[test]
fn breadcrumbs_carry_largest_first_siblings_and_selection_details() {
    let root = tempfile::tempdir().expect("create tree root");
    let big = root.path().join("big");
    fs::create_dir(&big).expect("create big");
    fs::write(big.join("blob.bin"), vec![1_u8; 64 * 1024]).expect("write");
    let small = root.path().join("small");
    fs::create_dir(&small).expect("create small");
    fs::write(small.join("a.txt"), "a").expect("write");
    let root_string = root.path().to_string_lossy().into_owned();
    let small_string = small.to_string_lossy().into_owned();
    let events = run_stream_after_scan(
        &root_string,
        &[serde_json::json!({
            "id":"view","command":"tree_view","root":root_string,
            "path":small_string
        })],
    );
    let view = events
        .iter()
        .find(|event| event["id"] == "view")
        .expect("tree_view");
    let crumbs = view["breadcrumbs"].as_array().expect("breadcrumbs");
    assert_eq!(crumbs.len(), 2);
    assert_eq!(crumbs[0]["siblings"], serde_json::json!([]));
    let siblings = crumbs[1]["siblings"].as_array().expect("siblings");
    let names: Vec<_> = siblings.iter().map(|item| &item["name"]).collect();
    assert_eq!(names, ["big", "small"]);
    assert!(siblings.iter().all(|item| {
        item["path"].is_string()
            && item["bytes"].is_u64()
            && item["is_dir"] == true
            && item["category"].is_string()
    }));
    assert_eq!(view["unreadable"], 0);
    let selection = &view["selection"];
    assert_eq!(selection["dirs"], 0);
    assert!(selection["modified"].is_i64() || selection["modified"].is_null());
    let share = selection["share"].as_f64().expect("share");
    assert!(share > 0.0 && share < 0.5);
    assert!(selection.get("reason").is_some());
}

#[test]
fn export_marks_returns_paths_and_prompt_and_saves_to_downloads() {
    let root = tempfile::tempdir().expect("create tree root");
    let file = root.path().join("blob.bin");
    fs::write(&file, vec![7_u8; 4096]).expect("write file");
    let home = tempfile::tempdir().expect("create home");
    fs::create_dir(home.path().join("Downloads")).expect("create Downloads");
    let root_string = root.path().to_string_lossy().into_owned();
    let file_string = file.to_string_lossy().into_owned();
    let mut child = Command::new(env!("CARGO_BIN_EXE_karen"))
        .arg("--stream-json")
        .env("HOME", home.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start Karen CLI");
    let mut input = child.stdin.take().expect("CLI stdin");
    let mut output = BufReader::new(child.stdout.take().expect("CLI stdout"));
    let mut next_event = || {
        let mut line = String::new();
        output.read_line(&mut line).expect("read CLI event");
        serde_json::from_str::<Value>(&line).expect("parse CLI event")
    };
    send_request(
        &mut input,
        serde_json::json!({"id":"scan","command":"scan","root":root_string}),
    );
    while read_event_for(&mut next_event, "scan")["event"] != "scan_complete" {}
    send_request(
        &mut input,
        serde_json::json!({"id":"empty","command":"export_marks","root":root_string,"format":"paths","save":false}),
    );
    assert_eq!(read_event_for(&mut next_event, "empty")["code"], "no_marks");
    send_request(
        &mut input,
        serde_json::json!({"id":"mark","command":"toggle_mark","root":root_string,"path":file_string}),
    );
    assert_eq!(read_event_for(&mut next_event, "mark")["marked"], true);
    send_request(
        &mut input,
        serde_json::json!({"id":"paths","command":"export_marks","root":root_string,"format":"paths","save":false}),
    );
    let paths = read_event_for(&mut next_event, "paths");
    assert_eq!(paths["event"], "marks_export");
    assert_eq!(paths["format"], "paths");
    assert_eq!(paths["text"], format!("{file_string}\n"));
    assert!(paths["saved_path"].is_null());
    send_request(
        &mut input,
        serde_json::json!({"id":"prompt","command":"export_marks","root":root_string,"format":"prompt","save":true}),
    );
    let prompt = read_event_for(&mut next_event, "prompt");
    assert_eq!(prompt["format"], "prompt");
    let text = prompt["text"].as_str().expect("prompt text");
    assert!(text.contains(&format!("- {file_string}")));
    let saved = prompt["saved_path"].as_str().expect("saved path");
    let saved = std::path::Path::new(saved);
    assert!(saved.starts_with(home.path().join("Downloads")));
    let name = saved.file_name().and_then(|n| n.to_str()).unwrap_or("");
    assert!(name.starts_with("karen-marks-"));
    assert_eq!(saved.extension().and_then(|ext| ext.to_str()), Some("md"));
    assert_eq!(fs::read_to_string(saved).expect("read saved"), text);
    send_request(
        &mut input,
        serde_json::json!({"id":"exit","command":"shutdown"}),
    );
    assert_eq!(read_event_for(&mut next_event, "exit")["event"], "shutdown");
    drop(input);
    assert!(child.wait().expect("wait for Karen CLI").success());
}

#[test]
fn reveal_rejects_paths_outside_scanned_roots() {
    let root = tempfile::tempdir().expect("create tree root");
    let outside = tempfile::tempdir().expect("create outside dir");
    let root_string = root.path().to_string_lossy().into_owned();
    let escape = root.path().join("..").join("etc");
    let events = run_stream_after_scan(
        &root_string,
        &[
            serde_json::json!({"id":"out","command":"reveal","path":outside.path().to_string_lossy()}),
            serde_json::json!({"id":"escape","command":"reveal","path":escape.to_string_lossy()}),
            serde_json::json!({"id":"relative","command":"reveal","path":"relative/path"}),
        ],
    );
    for id in ["out", "escape", "relative"] {
        let event = events
            .iter()
            .find(|event| event["id"] == id)
            .expect("reveal response");
        assert_eq!(event["code"], "reveal_rejected", "{id}");
    }
}

/// Scans `root` to completion, then sends `requests` and shuts down.
fn run_stream_after_scan(root: &str, requests: &[Value]) -> Vec<Value> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_karen"))
        .arg("--stream-json")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start Karen CLI");
    let mut input = child.stdin.take().expect("CLI stdin");
    let mut output = BufReader::new(child.stdout.take().expect("CLI stdout"));
    let mut next_event = || {
        let mut line = String::new();
        output.read_line(&mut line).expect("read CLI event");
        serde_json::from_str::<Value>(&line).expect("parse CLI event")
    };
    send_request(
        &mut input,
        serde_json::json!({"id":"scan","command":"scan","root":root}),
    );
    while read_event_for(&mut next_event, "scan")["event"] != "scan_complete" {}
    let mut events = Vec::new();
    for request in requests {
        let id = request["id"].as_str().expect("request id").to_owned();
        send_request(&mut input, request.clone());
        events.push(read_event_for(&mut next_event, &id));
    }
    send_request(
        &mut input,
        serde_json::json!({"id":"exit","command":"shutdown"}),
    );
    read_event_for(&mut next_event, "exit");
    drop(input);
    assert!(child.wait().expect("wait for Karen CLI").success());
    events
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

#[test]
fn large_tree_view_stays_bounded_instead_of_serializing_every_node() {
    let root = tempfile::tempdir().expect("create tree root");
    let wide = root.path().join("wide");
    fs::create_dir(&wide).expect("create wide directory");
    for index in 0..3000 {
        fs::write(wide.join(format!("file-{index:04}.dat")), [1_u8; 8])
            .expect("write file");
    }
    for dir in 0..20 {
        let nested = root.path().join(format!("dir-{dir:02}"));
        fs::create_dir(&nested).expect("create nested directory");
        for index in 0..50 {
            fs::write(nested.join(format!("f{index}.dat")), [1_u8; 8])
                .expect("write file");
        }
    }
    let root_string = root.path().to_string_lossy().into_owned();

    let events = run_stream_after_scan(
        &root_string,
        &[serde_json::json!({
            "id": "view", "command": "tree_view", "root": root_string,
            "path": root_string, "width": 1600, "height": 1000, "depth": 6
        })],
    );

    let view = &events[0];
    let tiles = view["tiles"].as_array().expect("tiles array");
    assert_eq!(view["files"], 4000);
    // Each directory keeps at most 96 children plus one Others tile, so the
    // view grows with directories shown, never with the 4,000 files scanned.
    let directories =
        tiles.iter().filter(|tile| tile["is_dir"] == true).count();
    assert!(
        tiles.len() <= 97 * (directories + 1),
        "{} tiles for {directories} directories",
        tiles.len()
    );
    assert!(tiles.len() < 2000, "view carried {} tiles", tiles.len());
    assert!(
        tiles.iter().any(|tile| tile["path"].is_null()),
        "the long tail must merge into an Others tile"
    );
    assert!(
        view.to_string().len() < 600_000,
        "event was {} bytes",
        view.to_string().len()
    );
}

#[test]
fn settings_persist_to_home_config_and_a_broken_file_is_never_overwritten() {
    let home = tempfile::tempdir().expect("temp home");
    let config = home.path().join(".karen").join("config.toml");
    let request = |request: Value| {
        let mut child = Command::new(env!("CARGO_BIN_EXE_karen"))
            .arg("--stream-json")
            .env("HOME", home.path())
            .env_remove("USERPROFILE")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("start Karen CLI");
        let mut input = child.stdin.take().expect("CLI stdin");
        let mut output =
            BufReader::new(child.stdout.take().expect("CLI stdout"));
        let mut next_event = || {
            let mut line = String::new();
            output.read_line(&mut line).expect("read CLI event");
            serde_json::from_str::<Value>(&line).expect("parse CLI event")
        };
        send_request(&mut input, request);
        let event = read_event_for(&mut next_event, "settings-request");
        send_request(
            &mut input,
            serde_json::json!({"id":"exit","command":"shutdown"}),
        );
        read_event_for(&mut next_event, "exit");
        drop(input);
        assert!(child.wait().expect("wait for Karen CLI").success());
        event
    };

    let defaults = request(
        serde_json::json!({"id":"settings-request","command":"get_settings"}),
    );
    assert_eq!(
        (defaults["power"].as_str(), defaults["ui_zoom"].as_f64()),
        (Some("balanced"), Some(1.0))
    );
    assert!(
        !config.exists(),
        "reading settings must not create the file"
    );

    let saved = request(serde_json::json!({
        "id":"settings-request","command":"set_settings","power":"miser","ui_zoom":1.3
    }));
    assert_eq!(saved["power"], "miser");
    let text = fs::read_to_string(&config).expect("config written");
    assert!(text.contains("miser"), "{text}");

    fs::write(&config, "power = [broken").expect("hand edit");
    let failed = request(serde_json::json!({
        "id":"settings-request","command":"set_settings","power":"aggressive"
    }));
    assert_eq!(failed["code"], "settings_invalid");
    assert_eq!(
        fs::read_to_string(&config).expect("config kept"),
        "power = [broken"
    );
}
