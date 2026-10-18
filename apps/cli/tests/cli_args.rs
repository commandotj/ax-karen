use std::fs;
use std::process::{Command, Output};

fn karen(args: &[&str], dir: &std::path::Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_karen"))
        .args(args)
        .current_dir(dir)
        .env("HOME", dir)
        .output()
        .expect("run Karen CLI")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn help_flags_print_usage() {
    let dir = tempfile::tempdir().expect("temp dir");
    for flag in ["--help", "-h"] {
        let output = karen(&[flag], dir.path());
        assert!(output.status.success(), "{flag}");
        assert!(stdout(&output).contains("usage: karen [PATH]"), "{flag}");
    }
}

#[test]
fn extra_arguments_fail_with_usage() {
    let dir = tempfile::tempdir().expect("temp dir");
    let output = karen(&["one", "two"], dir.path());
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("usage: karen"));
}

#[test]
fn plain_scan_prints_totals_for_a_path_or_the_current_directory() {
    let dir = tempfile::tempdir().expect("temp dir");
    fs::create_dir(dir.path().join("sub")).expect("create sub");
    fs::write(dir.path().join("sub").join("a.txt"), "a").expect("write");
    fs::write(dir.path().join("b.txt"), "b").expect("write");

    let named = karen(&[&dir.path().to_string_lossy()], dir.path());
    assert!(named.status.success());
    assert!(
        stdout(&named).starts_with("2 files, "),
        "{}",
        stdout(&named)
    );
    let current = karen(&[], dir.path());
    assert!(current.status.success());
    assert_eq!(stdout(&current), stdout(&named));
}

#[test]
fn scanning_a_missing_path_fails_with_context() {
    let dir = tempfile::tempdir().expect("temp dir");
    let missing = dir.path().join("missing");
    let output = karen(&[&missing.to_string_lossy()], dir.path());
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!("scan {}", missing.display())),
        "{stderr}"
    );
}
