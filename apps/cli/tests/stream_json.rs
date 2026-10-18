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
        assert!(event["battery"]["present"].is_boolean());
        assert!(event["network"]["total_rx_bytes"].is_u64());
        assert!(event["network"]["total_tx_bytes"].is_u64());
        let interfaces = event["network"]["interfaces"]
            .as_array()
            .expect("network interfaces list");
        assert!(interfaces.len() <= 20);
    }
    assert!(resources[0]["network"]["rx_bytes_per_sec"].is_null());
    assert!(resources[0]["network"]["tx_bytes_per_sec"].is_null());
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
        .env("USERPROFILE", home.path())
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

/// One CLI process with its own temporary `HOME`, so settings, exports and
/// removal guards never see the real home directory.
struct Session {
    child: std::process::Child,
    input: std::process::ChildStdin,
    output: BufReader<std::process::ChildStdout>,
    _home: tempfile::TempDir,
}

impl Session {
    fn start() -> Self {
        let home = tempfile::tempdir().expect("temp home");
        let mut child = Command::new(env!("CARGO_BIN_EXE_karen"))
            .arg("--stream-json")
            .env("HOME", home.path())
            .env("USERPROFILE", home.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("start Karen CLI");
        let input = child.stdin.take().expect("CLI stdin");
        let output = BufReader::new(child.stdout.take().expect("CLI stdout"));
        let mut session = Self {
            child,
            input,
            output,
            _home: home,
        };
        assert_eq!(session.event()["event"], "ready");
        session
    }

    fn event(&mut self) -> Value {
        let mut line = String::new();
        self.output.read_line(&mut line).expect("read CLI event");
        serde_json::from_str(&line).expect("parse CLI event")
    }

    /// Sends `request` and returns the next event carrying its id; events in
    /// between are dropped.
    fn reply(&mut self, request: Value) -> Value {
        let id = request["id"].as_str().expect("request id").to_owned();
        send_request(&mut self.input, request);
        loop {
            let event = self.event();
            if event["id"] == id.as_str() {
                return event;
            }
        }
    }

    /// Waits for the scan with `id` to finish and returns its last event.
    fn scan_result(&mut self, id: &str) -> Value {
        loop {
            let event = self.event();
            let done =
                event["event"] == "scan_complete" || event["event"] == "error";
            if event["id"] == id && done {
                return event;
            }
        }
    }

    fn scan(&mut self, root: &str) -> Value {
        let started = self.reply(
            serde_json::json!({"id":"scan","command":"scan","root":root}),
        );
        assert_eq!(started["event"], "scan_started");
        self.scan_result("scan")
    }

    fn until(&mut self, event_name: &str) -> Value {
        loop {
            let event = self.event();
            if event["event"] == event_name {
                return event;
            }
        }
    }

    fn finish(mut self) {
        assert_eq!(
            self.reply(serde_json::json!({"id":"exit","command":"shutdown"}))["event"],
            "shutdown"
        );
        drop(self.input);
        assert!(self.child.wait().expect("wait for Karen CLI").success());
    }
}

/// Writes raw bytes to stdin, closes it without `shutdown` and collects
/// every event until the CLI exits on its own.
fn run_raw(input: &[u8]) -> Vec<Value> {
    let home = tempfile::tempdir().expect("temp home");
    let mut child = Command::new(env!("CARGO_BIN_EXE_karen"))
        .arg("--stream-json")
        .env("HOME", home.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start Karen CLI");
    let mut stdin = child.stdin.take().expect("CLI stdin");
    stdin.write_all(input).expect("write input");
    drop(stdin);
    let output = child.wait_with_output().expect("wait for Karen CLI");
    assert!(output.status.success());
    String::from_utf8(output.stdout)
        .expect("JSON stream is UTF-8")
        .lines()
        .map(|line| serde_json::from_str(line).expect("each line is JSON"))
        .collect()
}

#[test]
fn malformed_requests_get_error_codes_and_closed_input_ends_the_stream() {
    let mut input = Vec::new();
    input.extend_from_slice(b"not json\n\n   \n");
    input.extend_from_slice(
        b"{\"version\":2,\"id\":\"old\",\"command\":\"shutdown\"}\n",
    );
    input.extend_from_slice(b"{\"version\":1,\"command\":\"shutdown\"}\n");
    input.extend_from_slice(
        b"{\"version\":1,\"id\":7,\"command\":\"shutdown\"}\n",
    );
    input.extend_from_slice(b"\xff\xfe\n");
    input.extend_from_slice(
        b"{\"version\":1,\"id\":\"what\",\"command\":\"fly\"}\n",
    );
    input.extend_from_slice(b"{\"version\":1,\"id\":\"none\"}\n");
    let events = run_raw(&input);
    let codes: Vec<_> = events
        .iter()
        .skip(1)
        .map(|event| (event["id"].clone(), event["code"].clone()))
        .collect();
    let json = |id: Value, code: &str| (id, Value::from(code));
    assert_eq!(
        codes,
        [
            json(Value::Null, "invalid_json"),
            json(Value::from("old"), "unsupported_version"),
            json(Value::Null, "invalid_request"),
            json(Value::Null, "invalid_request"),
            json(Value::Null, "stdin_failed"),
            json(Value::from("what"), "unknown_command"),
            json(Value::from("none"), "unknown_command"),
        ]
    );
    assert!(events[5]["message"].is_string());
}

#[test]
fn requests_without_roots_paths_or_a_cached_scan_are_rejected() {
    let mut session = Session::start();
    let missing = "/karen-test/not-scanned";
    for (request, code) in [
        (serde_json::json!({"command":"scan"}), "invalid_root"),
        (
            serde_json::json!({"command":"tree_view","path":"/"}),
            "invalid_root",
        ),
        (
            serde_json::json!({"command":"tree_view","root":"/"}),
            "invalid_path",
        ),
        (
            serde_json::json!({"command":"toggle_mark","root":"/"}),
            "invalid_path",
        ),
        (
            serde_json::json!({"command":"toggle_mark","root":missing,"path":missing}),
            "tree_cache_miss",
        ),
        (serde_json::json!({"command":"clear_marks"}), "invalid_root"),
        (serde_json::json!({"command":"review"}), "invalid_root"),
        (
            serde_json::json!({"command":"export_marks"}),
            "invalid_root",
        ),
        (serde_json::json!({"command":"reveal"}), "invalid_path"),
        (
            serde_json::json!({"command":"confirm_remove","mode":"permanent"}),
            "review_required",
        ),
    ] {
        let mut request = request;
        request["id"] = Value::from(code);
        let event = session.reply(request.clone());
        assert_eq!(event["code"], code, "{request}");
    }
    let miss = session.reply(serde_json::json!({
        "id":"miss","command":"tree_view","root":missing,"path":missing
    }));
    assert_eq!(miss["event"], "tree_cache_miss");
    assert_eq!(miss["root"], missing);
    let cancel = session
        .reply(serde_json::json!({"id":"cancel","command":"cancel_scan"}));
    assert_eq!(cancel["event"], "scan_cancel_requested");
    assert_eq!(cancel["accepted"], false);
    assert_eq!(
        session
            .reply(serde_json::json!({"id":"undo","command":"cancel_review"}))
            ["event"],
        "review_cancelled"
    );
    let failed = session.reply(serde_json::json!({
        "id":"bad-scan","command":"scan","root":"/karen-test/does-not-exist"
    }));
    assert_eq!(failed["event"], "scan_started");
    let failed = session.scan_result("bad-scan");
    assert_eq!(failed["code"], "scan_failed");
    assert!(failed["message"].is_string());
    session.finish();
}

#[test]
fn volumes_lists_the_boot_disk() {
    let mut session = Session::start();
    let volumes =
        session.reply(serde_json::json!({"id":"vol","command":"volumes"}));
    assert_eq!(volumes["event"], "volumes");
    let list = volumes["volumes"].as_array().expect("volume list");
    let boot = list
        .iter()
        .find(|volume| {
            if cfg!(windows) {
                volume["mount_point"]
                    .as_str()
                    .is_some_and(|m| m.starts_with("C:") || m.starts_with("c:"))
            } else {
                volume["mount_point"] == "/"
            }
        })
        .expect("boot volume");
    assert!(boot["total_bytes"].as_u64().unwrap_or(0) > 0);
    assert!(boot.get("device").is_some());
    assert!(boot["free_bytes"].is_u64() && boot["available_bytes"].is_u64());
    session.finish();
}

#[test]
fn settings_without_any_home_directory_report_settings_invalid() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_karen"))
        .arg("--stream-json")
        .env_remove("HOME")
        .env_remove("USERPROFILE")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start Karen CLI");
    let mut input = child.stdin.take().expect("CLI stdin");
    send_request(
        &mut input,
        serde_json::json!({"id":"get","command":"get_settings"}),
    );
    send_request(
        &mut input,
        serde_json::json!({"id":"set","command":"set_settings","power":"miser"}),
    );
    drop(input);
    let output = child.wait_with_output().expect("wait for Karen CLI");
    let events: Vec<Value> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| serde_json::from_str(line).expect("JSON line"))
        .collect();
    for id in ["get", "set"] {
        let event = events
            .iter()
            .find(|event| event["id"] == id)
            .expect("settings reply");
        assert_eq!(event["code"], "settings_invalid");
        assert!(
            event["message"]
                .as_str()
                .is_some_and(|message| message.contains("no home directory"))
        );
    }
}

#[test]
fn tree_view_and_marks_reject_bad_paths_and_clear_marks_resets() {
    let root = tempfile::tempdir().expect("create tree root");
    let file = root.path().join("blob.bin");
    fs::write(&file, vec![7_u8; 4096]).expect("write file");
    let root_string = root.path().to_string_lossy().into_owned();
    let file_string = file.to_string_lossy().into_owned();
    let escape = root.path().join("..").to_string_lossy().into_owned();
    let mut session = Session::start();
    assert_eq!(session.scan(&root_string)["event"], "scan_complete");

    for (id, path) in
        [("outside", "/karen-test/elsewhere"), ("escape", &escape)]
    {
        let view = session.reply(serde_json::json!({
            "id":id,"command":"tree_view","root":root_string,"path":path
        }));
        assert_eq!(view["code"], "tree_path_not_found", "{id}");
    }
    let missing = session.reply(serde_json::json!({
        "id":"missing","command":"toggle_mark","root":root_string,
        "path":root.path().join("nope").to_string_lossy()
    }));
    assert_eq!(missing["code"], "tree_path_not_found");
    let root_mark = session.reply(serde_json::json!({
        "id":"root","command":"toggle_mark","root":root_string,"path":root_string
    }));
    assert_eq!(root_mark["code"], "mark_rejected");
    assert_eq!(root_mark["message"], "RootCannotBeMarked");

    let mark = session.reply(serde_json::json!({
        "id":"mark","command":"toggle_mark","root":root_string,"path":file_string
    }));
    assert_eq!(mark["marks"].as_array().map(Vec::len), Some(1));
    let format = session.reply(serde_json::json!({
        "id":"format","command":"export_marks","root":root_string,"format":"csv"
    }));
    assert_eq!(format["code"], "invalid_format");
    // The temporary home has no Downloads folder to save into.
    let unsaved = session.reply(serde_json::json!({
        "id":"unsaved","command":"export_marks","root":root_string,
        "format":"paths","save":true
    }));
    assert_eq!(unsaved["code"], "export_failed");
    assert!(unsaved["message"].is_string());
    let cleared = session.reply(serde_json::json!({
        "id":"clear","command":"clear_marks","root":root_string
    }));
    assert_eq!(cleared["event"], "marks_changed");
    assert_eq!(cleared["marks"], serde_json::json!([]));
    let empty = session.reply(serde_json::json!({
        "id":"empty","command":"export_marks","root":root_string,"format":"paths"
    }));
    assert_eq!(empty["code"], "no_marks");
    assert!(file.exists());
    session.finish();
}

#[test]
fn an_empty_root_reports_zero_share() {
    let root = tempfile::tempdir().expect("create tree root");
    let root_string = root.path().to_string_lossy().into_owned();
    let mut session = Session::start();
    assert_eq!(session.scan(&root_string)["bytes"], 0);
    let view = session.reply(serde_json::json!({
        "id":"view","command":"tree_view","root":root_string,"path":root_string
    }));
    assert_eq!(view["selection"]["share"], 0.0);
    assert_eq!(view["tiles"], serde_json::json!([]));
    session.finish();
}

#[test]
fn permanent_removal_inside_a_tempdir_streams_progress_items_and_completion() {
    let root = tempfile::tempdir().expect("create tree root");
    let doomed = root.path().join("doomed");
    fs::create_dir(&doomed).expect("create doomed dir");
    fs::write(doomed.join("a.bin"), vec![1_u8; 4096]).expect("write file");
    let keep = root.path().join("keep.txt");
    fs::write(&keep, "keep").expect("write keep file");
    let root_string = root.path().to_string_lossy().into_owned();
    let doomed_string = doomed.to_string_lossy().into_owned();
    let mut session = Session::start();
    session.scan(&root_string);
    session.reply(serde_json::json!({
        "id":"mark","command":"toggle_mark","root":root_string,"path":doomed_string
    }));
    assert_eq!(
        session.reply(serde_json::json!({"id":"review","command":"review","root":root_string}))["targets"],
        1
    );
    let mode = session.reply(serde_json::json!({
        "id":"mode","command":"confirm_remove","mode":"shred"
    }));
    assert_eq!(mode["code"], "invalid_removal_mode");
    // The rejected mode keeps the reviewed plan, so a valid one can follow.
    let started = session.reply(serde_json::json!({
        "id":"go","command":"confirm_remove","mode":"permanent"
    }));
    assert_eq!(started["event"], "removal_started");
    assert_eq!(started["mode"], "Delete permanently");
    let progress = session.until("removal_progress");
    assert_eq!(
        (progress["total"].as_u64(), progress["completed"].as_u64()),
        (Some(1), Some(0))
    );
    let item = session.until("removal_item");
    assert_eq!(item["path"], doomed_string);
    assert_eq!(item["ok"], true);
    assert!(item["error"].is_null());
    let complete = session.until("removal_complete");
    assert_eq!(complete["removed"], 1);
    assert_eq!(complete["failed"], 0);
    assert!(!doomed.exists());
    assert!(keep.exists());
    session.finish();
}

/// Enough files that scanning and removing take longer than the next
/// request does to arrive.
fn crowded_root() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("create tree root");
    for dir in 0..8 {
        let nested = root.path().join(format!("dir-{dir}"));
        fs::create_dir(&nested).expect("create nested directory");
        for index in 0..600 {
            fs::write(nested.join(format!("f{index}")), [1_u8; 8])
                .expect("write file");
        }
    }
    root
}

#[test]
fn busy_scan_and_removal_are_refused_and_cancel_is_accepted() {
    let root = crowded_root();
    let root_string = root.path().to_string_lossy().into_owned();
    let mut session = Session::start();
    send_request(
        &mut session.input,
        serde_json::json!({"id":"scan","command":"scan","root":root_string}),
    );
    send_request(
        &mut session.input,
        serde_json::json!({"id":"again","command":"scan","root":root_string}),
    );
    send_request(
        &mut session.input,
        serde_json::json!({"id":"cancel","command":"cancel_scan"}),
    );
    let mut busy = None;
    let mut cancel = None;
    while busy.is_none() || cancel.is_none() {
        let event = session.event();
        if event["id"] == "again" {
            busy = Some(event);
        } else if event["id"] == "cancel" {
            cancel = Some(event);
        }
    }
    assert_eq!(busy.expect("busy reply")["code"], "scan_busy");
    assert_eq!(cancel.expect("cancel reply")["accepted"], true);
    session.scan_result("scan");

    assert_eq!(session.scan(&root_string)["event"], "scan_complete");
    for dir in 0..8 {
        let path = root.path().join(format!("dir-{dir}"));
        session.reply(serde_json::json!({
            "id":"mark","command":"toggle_mark","root":root_string,
            "path":path.to_string_lossy()
        }));
    }
    for request in [
        serde_json::json!({"id":"review","command":"review","root":root_string}),
        serde_json::json!({"id":"go","command":"confirm_remove","mode":"permanent"}),
        serde_json::json!({"id":"review-2","command":"review","root":root_string}),
        serde_json::json!({"id":"busy","command":"confirm_remove","mode":"permanent"}),
    ] {
        send_request(&mut session.input, request);
    }
    let busy = loop {
        let event = session.event();
        if event["id"] == "busy" {
            break event;
        }
    };
    assert_eq!(busy["code"], "removal_busy");
    session.until("removal_complete");
    session.finish();
}

#[test]
fn power_presets_map_to_worker_counts() {
    let root = tempfile::tempdir().expect("create scan root");
    let root_string = root.path().to_string_lossy().into_owned();
    let cpus =
        std::thread::available_parallelism().map_or(1, usize::from) as u64;
    let mut session = Session::start();
    for (power, workers) in [("aggressive", 8.min(cpus)), ("drain", cpus)] {
        let started = session.reply(serde_json::json!({
            "id":power,"command":"scan","root":root_string,"power":power
        }));
        assert_eq!(started["workers"], workers, "{power}");
        session.scan_result(power);
    }
    session.finish();
}

/// Sparse files measured by apparent size reach the 64 MiB finding floor
/// without writing to disk.
fn sparse(path: &std::path::Path) {
    fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
    fs::File::create(path)
        .and_then(|file| file.set_len(100 * 1024 * 1024))
        .expect("sparse file");
}

#[test]
fn worth_a_look_names_reclaimable_worktree_and_stale_experiment_findings() {
    let root = tempfile::tempdir().expect("create tree root");
    sparse(&root.path().join("tmp").join("cache.bin"));
    sparse(&root.path().join("worktrees").join("feature").join("blob"));
    let old = root.path().join("tries").join("old-idea");
    sparse(&old.join("blob"));
    let long_ago =
        std::time::SystemTime::now() - std::time::Duration::from_hours(90 * 24);
    // A directory's age follows its newest entry; age both on Unix.
    #[cfg(unix)]
    for path in [old.join("blob"), old.clone()] {
        fs::File::open(&path)
            .and_then(|entry| entry.set_modified(long_ago))
            .expect("age experiment");
    }
    #[cfg(not(unix))]
    {
        fs::OpenOptions::new()
            .write(true)
            .open(old.join("blob"))
            .and_then(|entry| entry.set_modified(long_ago))
            .expect("age experiment");
    }
    let root_string = root.path().to_string_lossy().into_owned();
    let tmp_string = root.path().join("tmp").to_string_lossy().into_owned();
    let mut session = Session::start();
    send_request(
        &mut session.input,
        serde_json::json!({"id":"scan","command":"scan","root":root_string,"apparent_size":true}),
    );
    assert_eq!(session.scan_result("scan")["event"], "scan_complete");
    let view = session.reply(serde_json::json!({
        "id":"view","command":"tree_view","root":root_string,"path":tmp_string
    }));
    let findings = view["worth_a_look"].as_array().expect("findings");
    let reasons: Vec<_> = findings
        .iter()
        .filter_map(|item| item["reason"].as_str())
        .collect();
    assert_eq!(findings.len(), 3, "{reasons:?}");
    assert!(
        reasons
            .iter()
            .any(|reason| reason.starts_with("1 worktrees · oldest "))
    );
    assert!(reasons.contains(&"1 stale experiments"));
    let tmp = findings
        .iter()
        .find(|item| item["path"] == tmp_string.as_str())
        .expect("tmp finding");
    assert_eq!(view["selection"]["reason"], tmp["reason"]);
    session.finish();
}

#[test]
fn a_large_scan_streams_provisional_previews_before_completion() {
    let root = tempfile::tempdir().expect("create tree root");
    for index in 0..4 {
        fs::write(root.path().join(format!("top-{index}")), [1_u8; 8])
            .expect("write file");
    }
    for dir in 0..30 {
        let nested = root.path().join(format!("dir-{dir:02}"));
        fs::create_dir(&nested).expect("create nested directory");
        for index in 0..(if dir == 0 { 10_000 } else { 50 }) {
            fs::write(nested.join(format!("f{index}")), [1_u8; 8])
                .expect("write file");
        }
    }
    let root_string = root.path().to_string_lossy().into_owned();
    let mut session = Session::start();
    send_request(
        &mut session.input,
        serde_json::json!({"id":"scan","command":"scan","root":root_string,"power":"miser"}),
    );
    // Every request is followed by a scan poll, so a stream of no-op
    // requests samples the scan densely instead of every 25 ms.
    let pings: usize = 3000;
    let mut previews = Vec::new();
    std::thread::scope(|scope| {
        let input = &mut session.input;
        scope.spawn(move || {
            for _ in 0..pings {
                send_request(
                    input,
                    serde_json::json!({"id":"ping","command":"cancel_review"}),
                );
            }
        });
        let (mut pongs, mut complete) = (0, false);
        while pongs < pings || !complete {
            let mut line = String::new();
            session.output.read_line(&mut line).expect("read CLI event");
            let event: Value = serde_json::from_str(&line).expect("event");
            pongs += usize::from(event["id"] == "ping");
            complete |= event["event"] == "scan_complete";
            if event["event"] == "scan_preview" {
                previews.push(event);
            }
        }
    });
    let preview = previews.last().expect("at least one preview");
    assert_eq!(preview["provisional"], true);
    assert_eq!(preview["root"], root_string);
    assert!(previews.iter().any(|preview| {
        preview["tiles"].as_array().is_some_and(|tiles| {
            tiles.iter().any(|tile| tile["is_dir"] == true)
        })
    }));
    session.finish();
}
