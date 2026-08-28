use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn deslop(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_deslop"))
        .args(args)
        .output()
        .unwrap()
}

fn stdout(o: &Output) -> String {
    String::from_utf8(o.stdout.clone()).unwrap()
}

fn tmp_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("sub")).unwrap();
    fs::create_dir_all(dir.join("vendor")).unwrap();
    fs::write(dir.join(".gitignore"), "vendor/\n").unwrap();
    fs::write(dir.join("a.md"), "Turns out it works.\n").unwrap();
    fs::write(dir.join("sub/b.txt"), "It's important to note this.\n").unwrap();
    fs::write(dir.join("vendor/c.md"), "Turns out ignored.\n").unwrap();
    fs::write(dir.join("d.py"), "Turns out skipped.\n").unwrap();
    dir
}

#[test]
fn walks_directory_respecting_gitignore_and_extensions() {
    let dir = tmp_dir("walk");
    let out = deslop(&["--jsonl", dir.to_str().unwrap()]);
    let s = stdout(&out);
    assert!(s.contains("a.md"), "{s}");
    assert!(s.contains("b.txt"), "{s}");
    assert!(!s.contains("c.md"), "{s}");
    assert!(!s.contains("d.py"), "{s}");
    assert_eq!(s.lines().count(), 2);

    let out = deslop(&["--jsonl", "--ext", "py", dir.to_str().unwrap()]);
    let s = stdout(&out);
    assert!(s.contains("d.py"), "{s}");
    assert_eq!(s.lines().count(), 1);
}

#[test]
fn inline_text_carries_hint_and_version() {
    let out = deslop(&["--jsonl", "--text", "Turns out it works."]);
    let line = stdout(&out);
    let v: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
    assert_eq!(v["path"], "<text1>");
    assert_eq!(v["pattern"], "turns-out");
    assert_eq!(v["version"], 1);
    assert!(v["hint"].as_str().unwrap().contains("state the finding"));

    let out = deslop(&["--json", "--text", "Turns out it works."]);
    let v: serde_json::Value = serde_json::from_str(&stdout(&out)).unwrap();
    assert_eq!(v["version"], 1);
    assert_eq!(v["total"]["matches"], 1);
}

#[test]
fn text_output_shows_fix_line() {
    let out = deslop(&["--text", "Turns out it works."]);
    let s = stdout(&out);
    assert!(s.contains("turns-out"), "{s}");
    assert!(s.contains("fix: Delete"), "{s}");
}

#[test]
fn quiet_check_only_sets_exit_code() {
    let out = deslop(&["--check", "--quiet", "--text", "Turns out it works."]);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());

    let out = deslop(&["--check", "--quiet", "--text", "A plain sentence."]);
    assert_eq!(out.status.code(), Some(0));
    assert!(out.stdout.is_empty());
}

#[test]
fn agent_help_prints_loop() {
    let out = deslop(&["--agent-help"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(stdout(&out).contains("--check --quiet"));
}

#[test]
fn list_patterns_includes_hint() {
    let out = deslop(&["--list-patterns", "--jsonl"]);
    let first = stdout(&out).lines().next().unwrap().to_string();
    let v: serde_json::Value = serde_json::from_str(&first).unwrap();
    assert!(v["hint"].as_str().unwrap().len() > 5);
}
