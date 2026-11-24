use assert_cmd::Command;
use predicates::prelude::*;

/// Temp HOME for an isolated `ez` config/data dir.
struct TestHome(tempfile::TempDir);

impl TestHome {
    fn new() -> Self {
        TestHome(tempfile::tempdir().unwrap())
    }

    fn path(&self) -> &std::path::Path {
        self.0.path()
    }
}

fn ez(home: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("ez").unwrap();
    cmd.env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("XDG_DATA_HOME", home.join(".local/share"))
        // Don't leave auto-started `ez daemon run` processes behind.
        .env("EZ_NO_DAEMON", "1");
    cmd
}

#[test]
fn rejects_unknown_backend_and_keeps_config() {
    let home = TestHome::new();
    ez(home.path())
        .args(["config", "set", "selector.backend", "native"])
        .assert()
        .success();
    ez(home.path())
        .args(["config", "set", "selector.backend", "skim"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("fzf").and(predicate::str::contains("native")));
    ez(home.path())
        .args(["config", "get", "selector.backend"])
        .assert()
        .success()
        .stdout("native\n");
}

#[test]
fn native_options_get_and_set() {
    let home = TestHome::new();
    ez(home.path())
        .args(["config", "get", "native.preview_ratio"])
        .assert()
        .success()
        .stdout("50\n");
    ez(home.path())
        .args(["config", "set", "native.preview_ratio", "40"])
        .assert()
        .success();
    ez(home.path())
        .args(["config", "get", "native.preview_ratio"])
        .assert()
        .success()
        .stdout("40\n");
    ez(home.path())
        .args(["config", "get", "native.snapshot"])
        .assert()
        .success()
        .stdout("true\n");
    ez(home.path())
        .args(["config", "set", "native.preview_ratio", "500"])
        .assert()
        .failure();
}

#[test]
fn browse_with_unknown_backend_in_file_errors() {
    let home = TestHome::new();
    ez(home.path())
        .args(["config", "get", "selector.backend"])
        .assert()
        .success();
    // Hand-edit whichever config file `ez` created.
    let out = ez(home.path()).args(["config", "show"]).output().unwrap();
    assert!(out.status.success());
    let path = std::fs::read_dir(home.path())
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .find_map(|p| find_config(&p));
    let path = path.expect("config.toml created");
    let text = std::fs::read_to_string(&path)
        .unwrap()
        .replace("backend = \"native\"", "backend = \"skim\"");
    std::fs::write(&path, text).unwrap();
    ez(home.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("fzf").and(predicate::str::contains("native")));
}

fn find_config(dir: &std::path::Path) -> Option<std::path::PathBuf> {
    for e in std::fs::read_dir(dir).ok()?.flatten() {
        let p = e.path();
        if p.is_dir() {
            if let Some(f) = find_config(&p) {
                return Some(f);
            }
        } else if p.file_name().is_some_and(|n| n == "config.toml") {
            return Some(p);
        }
    }
    None
}
