use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

#[test]
fn test_help() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_torrite"));
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "A CLI utility to create BitTorrent metainfo files",
        ));
}

#[test]
fn test_version() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_torrite"));
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "torrite {}",
            env!("CARGO_PKG_VERSION")
        )));
}

#[test]
fn test_create_basic() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_torrite"));
    // Create a dummy file for testing
    let temp_dir = tempfile::tempdir().unwrap();
    let source_file = temp_dir.path().join("test.txt");
    fs::write(&source_file, "random data").unwrap();

    cmd.arg("create")
        .arg(&source_file)
        .arg("--output")
        .arg(temp_dir.path().join("test.torrent"))
        .assert()
        .success()
        .stderr(predicate::str::contains("Created:"));

    assert!(temp_dir.path().join("test.torrent").exists());
}

#[test]
fn test_create_implicit() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_torrite"));
    let temp_dir = tempfile::tempdir().unwrap();
    let source_file = temp_dir.path().join("test_implicit.txt");
    fs::write(&source_file, "random data").unwrap();
    let output_file = temp_dir.path().join("test_implicit.torrent");

    cmd.arg(&source_file)
        .arg("-o")
        .arg(&output_file)
        .assert()
        .success();

    assert!(output_file.exists());
}

#[test]
fn test_missing_file() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_torrite"));
    cmd.arg("non_existent_file.txt").assert().failure();
}

#[test]
fn test_output_json() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_torrite"));
    let temp_dir = tempfile::tempdir().unwrap();
    let source_file = temp_dir.path().join("json_test.txt");
    fs::write(&source_file, "json test data").unwrap();

    cmd.arg(&source_file)
        .arg("--json")
        .arg("-o")
        .arg(temp_dir.path().join("out.torrent"))
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\": \"json_test.txt\""))
        .stdout(predicate::str::contains("\"info_hash_v1\":"));
}

#[test]
fn test_verify() {
    let mut cmd_create = Command::new(env!("CARGO_BIN_EXE_torrite"));
    let temp_dir = tempfile::tempdir().unwrap();
    let source_file = temp_dir.path().join("verify_test.txt");
    fs::write(&source_file, "verify test data").unwrap();
    let torrent_file = temp_dir.path().join("verify_test.torrent");

    // Create
    cmd_create
        .arg("create")
        .arg(&source_file)
        .arg("-o")
        .arg(&torrent_file)
        .assert()
        .success();

    // Verify
    let mut cmd_verify = Command::new(env!("CARGO_BIN_EXE_torrite"));
    cmd_verify
        .arg("verify")
        .arg(&torrent_file)
        .arg("--path")
        .arg(&source_file)
        .assert()
        .success()
        .stdout(predicate::str::contains("Verification Successful!"));
}

#[test]
fn test_edit() {
    let mut cmd_create = Command::new(env!("CARGO_BIN_EXE_torrite"));
    let temp_dir = tempfile::tempdir().unwrap();
    let source_file = temp_dir.path().join("edit_test.txt");
    fs::write(&source_file, "edit test data").unwrap();
    let torrent_file = temp_dir.path().join("edit_test.torrent");

    // Create
    cmd_create
        .arg("create")
        .arg(&source_file)
        .arg("-o")
        .arg(&torrent_file)
        .assert()
        .success();

    // Edit
    let mut cmd_edit = Command::new(env!("CARGO_BIN_EXE_torrite"));
    cmd_edit
        .arg("edit")
        .arg(&torrent_file)
        .arg("--comment")
        .arg("New Comment")
        .assert()
        .success();

    // Verify comment (by checking file content or creating again and checking output,
    // but simply checking success is good for now, maybe grep the file content?)
    // A simple check is that the command succeeded.
    // Ideally we would inspect the torrent file, but that requires reading bencode.
    // We can use verify to check if it's still valid.

    let mut cmd_verify = Command::new(env!("CARGO_BIN_EXE_torrite"));
    cmd_verify
        .arg("verify")
        .arg(&torrent_file)
        .arg("--path")
        .arg(&source_file)
        .assert()
        .success();
}

#[test]
fn test_dry_run() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_torrite"));
    let temp_dir = tempfile::tempdir().unwrap();
    let source_file = temp_dir.path().join("dry_run.txt");
    fs::write(&source_file, "dry run data").unwrap();

    cmd.arg("create")
        .arg(&source_file)
        .arg("--dry-run")
        .assert()
        .success()
        .stderr(predicate::str::contains("Dry Run Results:"));

    let output_file = temp_dir.path().join("dry_run_out.torrent");
    let mut cmd2 = Command::new(env!("CARGO_BIN_EXE_torrite"));
    cmd2.arg("create")
        .arg(&source_file)
        .arg("-o")
        .arg(&output_file)
        .arg("--dry-run")
        .assert()
        .success();

    assert!(!output_file.exists());
}

#[test]
fn test_inspect() {
    let mut cmd_create = Command::new(env!("CARGO_BIN_EXE_torrite"));
    let temp_dir = tempfile::tempdir().unwrap();
    let source_file = temp_dir.path().join("inspect.txt");
    fs::write(&source_file, "inspect data").unwrap();
    let torrent_file = temp_dir.path().join("inspect.torrent");

    // Create
    cmd_create
        .arg("create")
        .arg(&source_file)
        .arg("-o")
        .arg(&torrent_file)
        .assert()
        .success();

    // Inspect

    let mut cmd_inspect = Command::new(env!("CARGO_BIN_EXE_torrite"));

    cmd_inspect
        .arg("inspect")
        .arg(&torrent_file)
        .assert()
        .success()
        .stdout(predicate::str::contains("Torrent Metadata:"))
        .stdout(predicate::str::contains("Name:"));
}

#[test]
fn test_config_file_and_profile() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_torrite"));
    let temp_dir = tempfile::tempdir().unwrap();
    let source_file = temp_dir.path().join("profile_test.txt");
    fs::write(&source_file, "profile test data").unwrap();

    // Create a config file
    let config_path = temp_dir.path().join("config.toml");
    let mut config_file = fs::File::create(&config_path).unwrap();
    use std::io::Write; // Ensure Write trait is in scope for writeln!
    writeln!(
        config_file,
        r#"
        [profiles.my_custom]
        source = "MY_SOURCE"
        comment = "Profile Comment"
        piece_length = 18
    "#
    )
    .unwrap();

    let output_file = temp_dir.path().join("profile_out.torrent");

    cmd.arg("create")
        .arg(&source_file)
        .arg("--config")
        .arg(&config_path)
        .arg("-P")
        .arg("my_custom")
        .arg("-o")
        .arg(&output_file)
        .arg("--json") // Use JSON output to easily verify metadata
        .assert()
        .success()
        .stdout(predicate::str::contains("\"source\": \"MY_SOURCE\""))
        .stdout(predicate::str::contains("\"comment\": \"Profile Comment\""));
}

#[test]
fn test_tracker_defaults_ptp() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_torrite"));
    let temp_dir = tempfile::tempdir().unwrap();
    let source_file = temp_dir.path().join("ptp_test.txt");
    fs::write(&source_file, "ptp test data").unwrap();
    let output_file = temp_dir.path().join("ptp_out.torrent");

    cmd.arg("create")
        .arg(&source_file)
        .arg("-a")
        .arg("https://passthepopcorn.me/announce")
        .arg("-o")
        .arg(&output_file)
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"source\": \"PTP\"")); // Should auto-apply source "PTP"
}

#[test]
fn test_tracker_defaults_override() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_torrite"));
    let temp_dir = tempfile::tempdir().unwrap();
    let source_file = temp_dir.path().join("override_test.txt");
    fs::write(&source_file, "override test data").unwrap();
    let output_file = temp_dir.path().join("override_out.torrent");

    cmd.arg("create")
        .arg(&source_file)
        .arg("-a")
        .arg("https://passthepopcorn.me/announce")
        .arg("-s")
        .arg("CUSTOM_SOURCE") // Manually override the auto-default
        .arg("-o")
        .arg(&output_file)
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"source\": \"CUSTOM_SOURCE\""));
}
