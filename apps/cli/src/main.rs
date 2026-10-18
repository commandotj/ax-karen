//! Karen command-line scanner.

mod stream_json;

use std::path::Path;

use anyhow::{Context as _, Result};
use monitor_core::MonitorCore;
use monitor_disk::scan::ScanOptions;
use std::thread;
use std::time::Duration;

const USAGE: &str =
    "Karen — system monitor\n\nusage: karen [PATH] | karen --stream-json\n";

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.as_slice() == ["--stream-json"] {
        return stream_json::run();
    }
    if args.is_empty() {
        return scan(Path::new("."));
    }
    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        print!("{USAGE}");
        return Ok(());
    }
    if args.len() != 1 {
        anyhow::bail!("{USAGE}");
    }
    scan(Path::new(&args[0]))
}

fn scan(root: &Path) -> Result<()> {
    let mut monitor = MonitorCore::new();
    monitor.start_scan(root.to_path_buf(), ScanOptions::default());
    loop {
        if let Some(result) = monitor.poll_scan() {
            let tree =
                result.with_context(|| format!("scan {}", root.display()))?;
            println!(
                "{} files, {} directories, {} bytes",
                tree.files, tree.dirs, tree.bytes
            );
            return Ok(());
        }
        thread::sleep(Duration::from_millis(25));
    }
}
