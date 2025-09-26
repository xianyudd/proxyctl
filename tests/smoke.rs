use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::{fs, path::PathBuf, process::Command};
use tempfile::TempDir;

// 统一清理所有可能的代理相关环境变量（大小写 + no_proxy）
fn scrub_proxy_env(cmd: &mut Command) {
    for k in [
        "http_proxy",
        "https_proxy",
        "all_proxy",
        "no_proxy",
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "NO_PROXY",
    ] {
        cmd.env_remove(k);
    }
}

// helper：在临时 HOME 下运行被测二进制，且清理代理相关环境变量
fn bin_in_temp_home(tmp: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("proxyctl").expect("binary not built");
    cmd.env("HOME", tmp.path());
    scrub_proxy_env(&mut cmd);
    cmd
}

fn cfg_path(tmp: &TempDir) -> PathBuf {
    tmp.path().join(".proxyctl.toml")
}

#[test]
fn help_works() {
    let mut cmd = Command::cargo_bin("proxyctl").unwrap();
    scrub_proxy_env(&mut cmd);
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: proxyctl"));
}

#[test]
fn status_triggers_config_autogeneration() {
    let tmp = TempDir::new().unwrap();
    let cfg = cfg_path(&tmp);

    bin_in_temp_home(&tmp).arg("status").assert().success();

    assert!(cfg.exists(), "expect {} to be created", cfg.display());
    let content = fs::read_to_string(cfg).unwrap();
    assert!(content.contains("profile"), "config should contain profile");
    assert!(content.contains("mixed_port = 7890"));
    assert!(content.contains("socks_port = 7891"));
    assert!(content.contains("http_port  = 7892"));
}

#[test]
fn on_process_mode_mixed_profile_outputs_expected_ports() {
    let tmp = TempDir::new().unwrap();
    bin_in_temp_home(&tmp)
        .args(["--mode", "process", "on", "--ip", "1.2.3.4"])
        .assert()
        .success()
        .stdout(predicate::str::contains("http 7890, socks 7890"));
}

#[test]
fn on_process_mode_split_profile_outputs_expected_ports() {
    let tmp = TempDir::new().unwrap();
    bin_in_temp_home(&tmp)
        .args([
            "--mode",
            "process",
            "--profile",
            "split",
            "on",
            "--ip",
            "1.2.3.4",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("http 7892, socks 7891"));
}

#[test]
fn off_process_mode_succeeds() {
    let tmp = TempDir::new().unwrap();
    bin_in_temp_home(&tmp)
        .args(["--mode", "process", "off"])
        .assert()
        .success()
        .stdout(predicate::str::contains("proxy OFF (process)"));
}

#[test]
fn status_process_mode_prints_not_set() {
    let tmp = TempDir::new().unwrap();
    bin_in_temp_home(&tmp)
        .args(["--mode", "process", "status"])
        .assert()
        .success()
        // 不和整行比，只要包含 Not Set 即可；前面可能带“已生成默认配置”提示
        .stdout(predicate::str::contains("Not Set"));
}
