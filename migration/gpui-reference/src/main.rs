//! Karen command-line scanner.

mod stream_json;

use std::path::PathBuf;

use anyhow::{Context as _, Result};
use monitor_disk::scan::{ScanHandle, ScanOptions};
use std::thread;
use std::time::Duration;

const USAGE: &str = "Karen — system monitor\n\nusage: karen [PATH] | karen --stream-json\n";

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.as_slice() == ["--stream-json"] {
        return stream_json::run();
    }
    if args.is_empty() {
        return scan(PathBuf::from("."));
    }
    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        print!("{USAGE}");
        return Ok(());
    }
    if args.len() != 1 {
        anyhow::bail!("{USAGE}");
    }
    scan(PathBuf::from(&args[0]))
}

fn scan(root: PathBuf) -> Result<()> {
    let handle = ScanHandle::spawn(root.clone(), ScanOptions::default());
    loop {
        if let Some(result) = handle.poll() {
            let tree = result.with_context(|| format!("scan {}", root.display()))?;
            println!(
                "{} files, {} directories, {} bytes",
                tree.files, tree.dirs, tree.bytes
            );
            return Ok(());
        }
        thread::sleep(Duration::from_millis(25));
    }
}
