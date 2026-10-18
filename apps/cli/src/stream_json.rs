use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::Duration;

use anyhow::Result;
use monitor_core::MonitorCore;
use monitor_disk::removal::{RemovalEvent, RemovalHandle, RemovalMode};
use monitor_disk::scan::ScanOptions;
use monitor_disk::tree::{Metric, Node};
use monitor_disk::treemap::{self, LayoutOptions, Rect, TileKind};
use serde_json::{Value, json};

#[derive(Clone, Copy)]
struct Viewport {
    width: f32,
    height: f32,
}

pub fn run() -> Result<()> {
    run_stream(io::stdout().lock())
}

fn run_stream(mut output: impl Write) -> Result<()> {
    let (sender, requests) = mpsc::channel();
    thread::spawn(move || {
        for line in io::stdin().lock().lines() {
            let request = line.map_err(|error| error.to_string());
            if sender.send(request).is_err() {
                break;
            }
        }
    });

    write_event(&mut output, &json!({"version": 1, "event": "ready"}))?;
    let mut monitor = MonitorCore::new();
    let mut tree_cache = HashMap::new();
    let mut finding_cache = HashMap::new();
    let mut pending_removal = None;
    let mut removal = None;
    let mut active_scan_id: Option<Value> = None;
    let mut active_scan_root: Option<PathBuf> = None;
    let mut last_progress = None;
    let mut last_preview_revision = 0;
    let mut input_closed = false;
    let mut shutdown_requested = false;

    loop {
        if !shutdown_requested {
            match requests.recv_timeout(Duration::from_millis(25)) {
                Ok(Ok(line)) if !line.trim().is_empty() => {
                    shutdown_requested = handle_request(
                        &line,
                        &mut output,
                        &mut monitor,
                        &tree_cache,
                        &finding_cache,
                        &mut pending_removal,
                        &mut removal,
                        &mut active_scan_id,
                        &mut active_scan_root,
                        &mut last_progress,
                        &mut last_preview_revision,
                    )?;
                }
                Ok(Ok(_)) | Err(RecvTimeoutError::Timeout) => {}
                Ok(Err(message)) => write_event(
                    &mut output,
                    &json!({"version": 1, "event": "error", "code": "stdin_failed", "message": message}),
                )?,
                Err(RecvTimeoutError::Disconnected) => input_closed = true,
            }
        }

        report_scan(
            &mut output,
            &mut monitor,
            &mut tree_cache,
            &mut finding_cache,
            &mut active_scan_id,
            &mut active_scan_root,
            &mut last_progress,
            &mut last_preview_revision,
        )?;
        report_removal(&mut output, &mut removal)?;
        if (shutdown_requested || input_closed) && active_scan_id.is_none() {
            break;
        }
    }
    Ok(())
}

#[allow(
    clippy::too_many_arguments,
    reason = "the stream loop owns protocol state for one request"
)]
fn handle_request(
    line: &str,
    output: &mut impl Write,
    monitor: &mut MonitorCore,
    tree_cache: &HashMap<PathBuf, Node>,
    finding_cache: &HashMap<PathBuf, Vec<Value>>,
    pending_removal: &mut Option<monitor_disk::removal::Plan>,
    removal: &mut Option<RemovalHandle>,
    active_scan_id: &mut Option<Value>,
    active_scan_root: &mut Option<PathBuf>,
    last_progress: &mut Option<(u64, u64, u64, u64)>,
    last_preview_revision: &mut u64,
) -> Result<bool> {
    let request: Value = if let Ok(request) = serde_json::from_str(line) {
        request
    } else {
        write_event(output, &error_event(None, "invalid_json"))?;
        return Ok(false);
    };
    let id = request.get("id");
    if request["version"] != 1 {
        write_event(output, &error_event(id, "unsupported_version"))?;
        return Ok(false);
    }
    let Some(id) = id.filter(|value| value.is_string()) else {
        write_event(output, &error_event(None, "invalid_request"))?;
        return Ok(false);
    };
    match request["command"].as_str() {
        Some("shutdown") => {
            write_event(
                output,
                &json!({"version": 1, "id": id, "event": "shutdown"}),
            )?;
            Ok(true)
        }
        Some("scan") => {
            *pending_removal = None;
            let Some(root) = request.get("root").and_then(Value::as_str) else {
                write_event(output, &error_event(Some(id), "invalid_root"))?;
                return Ok(false);
            };
            let mut options = ScanOptions::default();
            if let Some(include_hidden) = request["include_hidden"].as_bool() {
                options.include_hidden = include_hidden;
            }
            if let Some(apparent_size) = request["apparent_size"].as_bool() {
                options.apparent_size = apparent_size;
            }
            if !monitor.start_scan(PathBuf::from(root), options) {
                write_event(output, &error_event(Some(id), "scan_busy"))?;
                return Ok(false);
            }
            *active_scan_id = Some(id.clone());
            *active_scan_root = Some(PathBuf::from(root));
            *last_progress = None;
            *last_preview_revision = 0;
            write_event(
                output,
                &json!({"version": 1, "id": id, "event": "scan_started"}),
            )?;
            Ok(false)
        }
        Some("cancel_scan") => {
            let cancelled = monitor.cancel_scan();
            write_event(
                output,
                &json!({
                    "version": 1,
                    "id": id,
                    "event": "scan_cancel_requested",
                    "accepted": cancelled
                }),
            )?;
            Ok(false)
        }
        Some("tree_view") => {
            let Some(root) = request.get("root").and_then(Value::as_str) else {
                write_event(output, &error_event(Some(id), "invalid_root"))?;
                return Ok(false);
            };
            let Some(path) = request.get("path").and_then(Value::as_str) else {
                write_event(output, &error_event(Some(id), "invalid_path"))?;
                return Ok(false);
            };
            let root = PathBuf::from(root);
            let path = PathBuf::from(path);
            let Some(tree) = tree_cache.get(&root) else {
                write_event(
                    output,
                    &json!({"version": 1, "id": id, "event": "tree_cache_miss", "root": root, "path": path}),
                )?;
                return Ok(false);
            };
            let Some(crumbs) = crumbs_for_any_path(&root, tree, &path) else {
                write_event(
                    output,
                    &error_event(Some(id), "tree_path_not_found"),
                )?;
                return Ok(false);
            };
            let Some(node) = node_at(tree, &crumbs) else {
                write_event(
                    output,
                    &error_event(Some(id), "tree_path_not_found"),
                )?;
                return Ok(false);
            };
            let open = request["open"].as_bool().unwrap_or(true);
            let metric = if request["metric"] == "files" {
                Metric::Files
            } else {
                Metric::Bytes
            };
            let depth =
                request["depth"].as_u64().unwrap_or(4).clamp(1, 6) as u32;
            let width = request["width"]
                .as_f64()
                .unwrap_or(1200.0)
                .clamp(320.0, 8192.0) as f32;
            let height = request["height"]
                .as_f64()
                .unwrap_or(680.0)
                .clamp(240.0, 8192.0) as f32;
            let (view_crumbs, view_node) = if open && node.is_dir() {
                (crumbs.as_slice(), node)
            } else {
                let parent_crumbs =
                    crumbs.get(..crumbs.len().saturating_sub(1)).unwrap_or(&[]);
                let Some(parent) = node_at(tree, parent_crumbs) else {
                    write_event(
                        output,
                        &error_event(Some(id), "tree_path_not_found"),
                    )?;
                    return Ok(false);
                };
                (parent_crumbs, parent)
            };
            write_event(
                output,
                &tree_view_event(
                    id,
                    &root,
                    &path,
                    tree,
                    view_crumbs,
                    view_node,
                    node,
                    monitor.disk_marks().items(&root),
                    finding_cache.get(&root).map_or(&[], Vec::as_slice),
                    metric,
                    depth,
                    width,
                    height,
                ),
            )?;
            Ok(false)
        }
        Some("toggle_mark") => {
            *pending_removal = None;
            let (Some(root), Some(path)) =
                (request["root"].as_str(), request["path"].as_str())
            else {
                write_event(output, &error_event(Some(id), "invalid_path"))?;
                return Ok(false);
            };
            let root = PathBuf::from(root);
            let path = PathBuf::from(path);
            let Some(tree) = tree_cache.get(&root) else {
                write_event(output, &error_event(Some(id), "tree_cache_miss"))?;
                return Ok(false);
            };
            if crumbs_for_any_path(&root, tree, &path).is_none() {
                write_event(
                    output,
                    &error_event(Some(id), "tree_path_not_found"),
                )?;
                return Ok(false);
            }
            let result = monitor.disk_marks_mut().toggle(&root, tree, &path);
            match result {
                Ok(marked) => write_event(
                    output,
                    &json!({"version":1,"id":id,"event":"mark_changed","root":root,"path":path,"marked":marked,"marks":mark_summaries(monitor.disk_marks().items(&root))}),
                )?,
                Err(error) => write_event(
                    output,
                    &json!({"version":1,"id":id,"event":"error","code":"mark_rejected","message":format!("{error:?}")}),
                )?,
            }
            Ok(false)
        }
        Some("clear_marks") => {
            *pending_removal = None;
            let Some(root) = request["root"].as_str() else {
                write_event(output, &error_event(Some(id), "invalid_root"))?;
                return Ok(false);
            };
            let root = PathBuf::from(root);
            monitor.disk_marks_mut().clear(&root);
            write_event(
                output,
                &json!({"version":1,"id":id,"event":"marks_changed","root":root,"marks":[]}),
            )?;
            Ok(false)
        }
        Some("review") => {
            let Some(root) = request["root"].as_str() else {
                write_event(output, &error_event(Some(id), "invalid_root"))?;
                return Ok(false);
            };
            let root = PathBuf::from(root);
            let plan = monitor.disk_marks().plan(&root);
            *pending_removal = Some(plan.clone());
            write_event(
                output,
                &json!({"version":1,"id":id,"event":"review","root":root,"marks":mark_summaries(monitor.disk_marks().items(&root)),"targets":plan.targets.len(),"bytes":plan.bytes(),"reclaim_bytes":plan.reclaim(),"foreign_bytes":plan.foreign(),"blocked":plan.blocked.iter().map(|item| json!({"path":item.path,"reason":item.reason})).collect::<Vec<_>>()}),
            )?;
            Ok(false)
        }
        Some("cancel_review") => {
            *pending_removal = None;
            write_event(
                output,
                &json!({"version":1,"id":id,"event":"review_cancelled"}),
            )?;
            Ok(false)
        }
        Some("confirm_remove") => {
            let Some(plan) = pending_removal.take() else {
                write_event(output, &error_event(Some(id), "review_required"))?;
                return Ok(false);
            };
            if removal.is_some() {
                *pending_removal = Some(plan);
                write_event(output, &error_event(Some(id), "removal_busy"))?;
                return Ok(false);
            }
            let mode = match request["mode"].as_str() {
                Some("trash") => RemovalMode::Trash,
                Some("permanent") => RemovalMode::Permanent,
                _ => {
                    *pending_removal = Some(plan);
                    write_event(
                        output,
                        &error_event(Some(id), "invalid_removal_mode"),
                    )?;
                    return Ok(false);
                }
            };
            *removal = Some(monitor_disk::removal::spawn(plan, mode));
            write_event(
                output,
                &json!({"version":1,"id":id,"event":"removal_started","mode":mode.label()}),
            )?;
            Ok(false)
        }
        Some("resources") => {
            let sample = monitor.sample_resources();
            write_event(
                output,
                &json!({
                    "version": 1,
                    "id": id,
                    "event": "resources",
                    "cpu_percent": sample.cpu_percent,
                    "cpu_processes_available": sample.cpu_processes.is_some(),
                    "cpu_processes": sample.cpu_processes.map(|processes| {
                        processes
                            .into_iter()
                            .map(|process| json!({
                                "pid": process.pid,
                                "name": process.name,
                                "cpu_percent": process.cpu_percent
                            }))
                            .collect::<Vec<_>>()
                    }),
                    "memory_used_bytes": sample.memory_used_bytes,
                    "memory_total_bytes": sample.memory_total_bytes,
                    "memory_processes_available": sample.memory_processes.is_some(),
                    "memory_processes": sample.memory_processes.map(|processes| {
                        processes
                            .into_iter()
                            .map(|process| json!({
                                "pid": process.pid,
                                "name": process.name,
                                "memory_bytes": process.memory_bytes
                            }))
                            .collect::<Vec<_>>()
                    })
                }),
            )?;
            Ok(false)
        }
        Some("volumes") => {
            let volumes: Vec<Value> = monitor
                .sample_volumes()
                .into_iter()
                .map(|volume| {
                    json!({
                        "mount_point": volume.mount_point,
                        "device": volume.device,
                        "total_bytes": volume.total_bytes,
                        "free_bytes": volume.free_bytes,
                        "available_bytes": volume.available_bytes
                    })
                })
                .collect();
            write_event(
                output,
                &json!({"version": 1, "id": id, "event": "volumes", "volumes": volumes}),
            )?;
            Ok(false)
        }
        _ => {
            write_event(output, &error_event(Some(id), "unknown_command"))?;
            Ok(false)
        }
    }
}

fn report_removal(
    output: &mut impl Write,
    removal: &mut Option<RemovalHandle>,
) -> Result<()> {
    let Some(handle) = removal.as_ref() else {
        return Ok(());
    };
    let mut finished = false;
    while let Some(event) = handle.poll() {
        let value = match event {
            RemovalEvent::Start { total } => {
                json!({"version":1,"event":"removal_progress","total":total,"completed":0})
            }
            RemovalEvent::Item {
                path,
                bytes,
                outcome,
            } => {
                json!({"version":1,"event":"removal_item","path":path,"bytes":bytes,"ok":outcome.is_ok(),"error":outcome.err()})
            }
            RemovalEvent::Done {
                removed,
                bytes,
                failed,
            } => {
                json!({"version":1,"event":"removal_complete","removed":removed,"bytes":bytes,"failed":failed})
            }
        };
        let done = value["event"] == "removal_complete";
        write_event(output, &value)?;
        if done {
            finished = true;
            break;
        }
    }
    if finished {
        *removal = None;
    }
    Ok(())
}

#[allow(
    clippy::too_many_arguments,
    reason = "scan event state is owned by the stream loop"
)]
fn report_scan(
    output: &mut impl Write,
    monitor: &mut MonitorCore,
    tree_cache: &mut HashMap<PathBuf, Node>,
    finding_cache: &mut HashMap<PathBuf, Vec<Value>>,
    active_scan_id: &mut Option<Value>,
    active_scan_root: &mut Option<PathBuf>,
    last_progress: &mut Option<(u64, u64, u64, u64)>,
    last_preview_revision: &mut u64,
) -> Result<()> {
    let Some(id) = active_scan_id.as_ref() else {
        return Ok(());
    };
    if let Some(result) = monitor.poll_scan() {
        let event = match result {
            Ok(tree) => {
                let root = active_scan_root
                    .take()
                    .unwrap_or_else(|| PathBuf::from("."));
                let event = scan_complete_event(id, &root, &tree);
                monitor.disk_marks_mut().refresh(&root, &tree);
                finding_cache
                    .insert(root.clone(), cached_findings(&root, &tree));
                tree_cache.insert(root, tree);
                event
            }
            Err(error) => json!({
                "version": 1,
                "id": id,
                "event": "error",
                "code": "scan_failed",
                "message": error.to_string()
            }),
        };
        write_event(output, &event)?;
        *active_scan_id = None;
        *active_scan_root = None;
        *last_progress = None;
        *last_preview_revision = 0;
        return Ok(());
    }
    let Some(snapshot) = monitor.scan_progress() else {
        return Ok(());
    };
    let progress = (
        snapshot.files,
        snapshot.dirs,
        snapshot.bytes,
        snapshot.errors,
    );
    if *last_progress == Some(progress) {
        if snapshot.preview_revision == *last_preview_revision {
            return Ok(());
        }
    } else {
        write_event(
            output,
            &json!({
                "version": 1,
                "id": id,
                "event": "scan_progress",
                "files": snapshot.files,
                "dirs": snapshot.dirs,
                "bytes": snapshot.bytes,
                "errors": snapshot.errors
            }),
        )?;
        *last_progress = Some(progress);
    }
    if snapshot.preview_revision != *last_preview_revision
        && let Some(root) = active_scan_root.as_ref()
    {
        write_event(output, &scan_preview_event(id, root, &snapshot))?;
        *last_preview_revision = snapshot.preview_revision;
    }
    Ok(())
}

fn scan_complete_event(
    id: &Value,
    root: &std::path::Path,
    tree: &Node,
) -> Value {
    json!({
        "version": 1,
        "id": id,
        "event": "scan_complete",
        "root": root.display().to_string(),
        "files": tree.files,
        "dirs": tree.dirs,
        "bytes": tree.bytes,
        "tiles": tree_tiles(root, tree)
    })
}

#[allow(
    clippy::too_many_arguments,
    reason = "event is assembled from one cached view and its selection context"
)]
fn tree_view_event(
    id: &Value,
    root: &Path,
    path: &Path,
    tree: &Node,
    crumbs: &[usize],
    view_node: &Node,
    selected_node: &Node,
    marks: &[monitor_disk::removal::Target],
    worth_a_look: &[Value],
    metric: Metric,
    depth: u32,
    width: f32,
    height: f32,
) -> Value {
    let selected_crumbs =
        crumbs_for_any_path(root, tree, path).unwrap_or_default();
    let sibling_parent_crumbs = selected_crumbs
        .get(..selected_crumbs.len().saturating_sub(1))
        .unwrap_or(&[]);
    let siblings = node_at(tree, sibling_parent_crumbs)
        .map(|parent| {
            parent
                .children
                .iter()
                .map(|child| {
                    json!({
                        "name": child.name,
                        "path": path_for_child(root, tree, sibling_parent_crumbs, child),
                        "bytes": child.bytes,
                        "is_dir": child.is_dir()
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    json!({
        "version": 1,
        "id": id,
        "event": "tree_view",
        "root": root.display().to_string(),
        "path": path.display().to_string(),
        "files": view_node.files,
        "dirs": view_node.dirs,
        "bytes": view_node.bytes,
        "selection": {"name": selected_node.name, "path": path, "category": selected_node.category.label(), "bytes": selected_node.bytes, "files": selected_node.files, "is_dir": selected_node.is_dir(), "marked": marks.iter().any(|mark| mark.path == path)},
        "siblings": siblings,
        "marks": mark_summaries(marks),
        "worth_a_look": worth_a_look,
        "breadcrumbs": breadcrumbs(root, tree, crumbs),
        "metric": metric.label(),
        "depth": depth,
        "tiles": tree_tiles_from_view(
            root,
            tree,
            view_node,
            crumbs,
            metric,
            depth,
            Viewport { width, height },
        )
    })
}

fn cached_findings(root: &Path, tree: &Node) -> Vec<Value> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_secs()).unwrap_or(i64::MAX)
        });
    monitor_disk::insights::worth_a_look(tree, now, 5)
        .into_iter()
        .map(|candidate| {
            let path = path_at(root, tree, &candidate.crumbs);
            json!({"path":path,"bytes":candidate.bytes,"reason":finding_reason(&candidate.finding)})
        })
        .collect()
}

fn finding_reason(finding: &monitor_disk::insights::Finding) -> String {
    match finding {
        monitor_disk::insights::Finding::Reclaimable(reason) => {
            reason.label().to_owned()
        }
        monitor_disk::insights::Finding::Worktrees { count, oldest_days } => {
            format!("{count} worktrees · oldest {oldest_days} days")
        }
        monitor_disk::insights::Finding::StaleExperiments { count } => {
            format!("{count} stale experiments")
        }
    }
}

fn mark_summaries(marks: &[monitor_disk::removal::Target]) -> Vec<Value> {
    marks.iter().map(|mark| json!({"path":mark.path,"bytes":mark.bytes,"is_dir":mark.is_dir,"hidden":mark.hidden})).collect()
}

fn path_for_child(
    root: &Path,
    tree: &Node,
    parent: &[usize],
    child: &Node,
) -> String {
    let mut path = PathBuf::from(path_at(root, tree, parent));
    path.push(&*child.name);
    path.display().to_string()
}

fn crumbs_for_any_path(
    root: &Path,
    tree: &Node,
    path: &Path,
) -> Option<Vec<usize>> {
    let relative = path.strip_prefix(root).ok()?;
    let mut node = tree;
    let mut crumbs = Vec::new();
    for component in relative.components() {
        let Component::Normal(name) = component else {
            return None;
        };
        let index = node.children.iter().position(|child| {
            std::ffi::OsStr::new(child.name.as_ref()) == name
        })?;
        node = node.children.get(index)?;
        crumbs.push(index);
    }
    Some(crumbs)
}

fn scan_preview_event(
    id: &Value,
    root: &std::path::Path,
    snapshot: &monitor_disk::scan::ScanSnapshot,
) -> Value {
    let mut tree = Node::directory(
        root.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_else(|| root.to_str().unwrap_or("Volume")),
    );
    for entry in &snapshot.preview {
        let kind = if entry.kind.is_dir() {
            monitor_disk::tree::NodeKind::Directory
        } else {
            entry.kind
        };
        let mut node = Node::entry(entry.name.as_str(), kind, entry.bytes);
        node.files = entry.files;
        node.dirs = entry.dirs;
        node.category = entry.category;
        tree.bytes = tree.bytes.saturating_add(entry.bytes);
        tree.files = tree.files.saturating_add(entry.files);
        tree.dirs = tree.dirs.saturating_add(entry.dirs);
        tree.children.push(node);
    }
    tree.children.sort_unstable_by(|left, right| {
        right
            .bytes
            .cmp(&left.bytes)
            .then_with(|| left.name.cmp(&right.name))
    });
    json!({
        "version": 1,
        "id": id,
        "event": "scan_preview",
        "root": root.display().to_string(),
        "files": snapshot.files,
        "dirs": snapshot.dirs,
        "bytes": snapshot.bytes,
        "provisional_bytes": tree.bytes,
        "provisional": true,
        "tiles": tree_tiles(root, &tree)
    })
}

fn tree_tiles(root: &std::path::Path, tree: &Node) -> Vec<Value> {
    tree_tiles_from_view(
        root,
        tree,
        tree,
        &[],
        Metric::Bytes,
        4,
        Viewport {
            width: 1200.0,
            height: 680.0,
        },
    )
}

fn tree_tiles_from_view(
    root: &Path,
    tree: &Node,
    view: &Node,
    view_crumbs: &[usize],
    metric: Metric,
    depth: u32,
    viewport: Viewport,
) -> Vec<Value> {
    let area = Rect::new(0.0, 0.0, viewport.width, viewport.height);
    let tiles = treemap::layout(
        view,
        view_crumbs,
        area,
        metric,
        &LayoutOptions {
            max_depth: depth,
            ..LayoutOptions::default()
        },
    );
    let tiles: Vec<Value> = tiles
        .into_iter()
        .map(|tile| {
            let (
                name,
                category,
                bytes,
                files,
                modified,
                is_dir,
                path,
                reclaimable,
                unreadable,
            ) = match &tile.kind {
                TileKind::Node { crumbs } => node_at(tree, crumbs).map_or_else(
                    || {
                        (
                            "Unknown item".to_owned(),
                            "Other",
                            None,
                            None,
                            None,
                            false,
                            None,
                            false,
                            false,
                        )
                    },
                    |node| {
                        (
                            node.name.to_string(),
                            node.category.label(),
                            Some(node.bytes),
                            Some(node.files),
                            Some(node.modified),
                            node.is_dir(),
                            Some(path_at(root, tree, crumbs)),
                            node.reclaim.is_some(),
                            node.read_error,
                        )
                    },
                ),
                TileKind::Others { count, .. } => (
                    format!("Other items ({count})"),
                    "Other",
                    None,
                    None,
                    None,
                    false,
                    None,
                    false,
                    false,
                ),
            };
            let (dirs, direct_bytes, parent_bytes) = match &tile.kind {
                TileKind::Node { crumbs } => {
                    node_at(tree, crumbs).map_or((None, None, None), |node| {
                        let parent = crumbs
                            .get(..crumbs.len().saturating_sub(1))
                            .unwrap_or(&[]);
                        let parent_bytes = node_at(tree, parent)
                            .map_or(0, |parent| parent.bytes);
                        (
                            Some(
                                node.dirs
                                    .saturating_sub(u32::from(node.is_dir())),
                            ),
                            Some(node.own_bytes()),
                            Some(parent_bytes),
                        )
                    })
                }
                TileKind::Others { .. } => (None, None, None),
            };
            json!({
                "name": name,
                "path": path,
                "category": category,
                "bytes": bytes,
                "files": files,
                "dirs": dirs,
                "direct_bytes": direct_bytes,
                "parent_bytes": parent_bytes,
                "modified": modified,
                "is_dir": is_dir,
                "reclaimable": reclaimable,
                "unreadable": unreadable,
                "depth": tile.depth,
                "has_header": tile.header.is_some(),
                "header": tile.header.map(|header| json!({
                    "x": header.x, "y": header.y,
                    "width": header.w, "height": header.h
                })),
                "x": tile.rect.x,
                "y": tile.rect.y,
                "width": tile.rect.w,
                "height": tile.rect.h
            })
        })
        .collect();
    tiles
}

fn node_at<'a>(root: &'a Node, crumbs: &[usize]) -> Option<&'a Node> {
    crumbs
        .iter()
        .try_fold(root, |node, index| node.children.get(*index))
}

fn path_at(root: &std::path::Path, tree: &Node, crumbs: &[usize]) -> String {
    let mut path = root.to_path_buf();
    let mut node = tree;
    for index in crumbs {
        let Some(child) = node.children.get(*index) else {
            break;
        };
        path.push(&*child.name);
        node = child;
    }
    path.display().to_string()
}

fn breadcrumbs(root: &Path, tree: &Node, crumbs: &[usize]) -> Vec<Value> {
    let mut path = root.to_path_buf();
    let mut node = tree;
    let mut result = vec![json!({
        "path": path.display().to_string(),
        "label": root.file_name().and_then(|name| name.to_str()).unwrap_or_else(|| root.to_str().unwrap_or("Volume"))
    })];
    for &index in crumbs {
        let Some(child) = node.children.get(index) else {
            break;
        };
        path.push(&*child.name);
        node = child;
        result.push(json!({
            "path": path.display().to_string(),
            "label": child.name.as_ref()
        }));
    }
    result
}

fn error_event(id: Option<&Value>, code: &str) -> Value {
    json!({"version": 1, "id": id, "event": "error", "code": code})
}

fn write_event(output: &mut impl Write, event: &Value) -> Result<()> {
    serde_json::to_writer(&mut *output, event)?;
    output.write_all(b"\n")?;
    output.flush()?;
    Ok(())
}
