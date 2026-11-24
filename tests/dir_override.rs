use assert_cmd::Command;
use predicates::prelude::*;

fn ez(home: &std::path::Path, cfg: &std::path::Path, data: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("ez").unwrap();
    cmd.env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("XDG_DATA_HOME", home.join(".local/share"))
        .env("EZ_CONFIG_DIR", cfg)
        .env("EZ_DATA_DIR", data)
        .env("EZ_NO_DAEMON", "1");
    cmd
}

#[test]
fn overrides_redirect_config_and_leave_home_untouched() {
    let home = tempfile::tempdir().unwrap();
    let dev = tempfile::tempdir().unwrap();
    let cfg = dev.path().join("config");
    let data = dev.path().join("data");

    ez(home.path(), &cfg, &data)
        .args(["config", "set", "selector.backend", "native"])
        .assert()
        .success();
    ez(home.path(), &cfg, &data)
        .args(["config", "get", "selector.backend"])
        .assert()
        .success()
        .stdout(predicate::str::contains("native"));

    assert!(cfg.join("config.toml").is_file());
    let leftovers: Vec<_> = std::fs::read_dir(home.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert!(leftovers.is_empty(), "HOME was touched: {leftovers:?}");
}
