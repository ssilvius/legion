use std::process::Command;

fn legion_cmd(data_dir: &std::path::Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_legion"));
    cmd.env("LEGION_DATA_DIR", data_dir);
    cmd
}

#[test]
fn reflect_and_recall_roundtrip() {
    let dir = tempfile::tempdir().unwrap();

    let output = legion_cmd(dir.path())
        .args([
            "reflect",
            "--repo",
            "test",
            "--text",
            "arrays are tricky in codegen",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "reflect failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output = legion_cmd(dir.path())
        .args(["recall", "--repo", "test", "--context", "codegen arrays"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "recall failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("arrays are tricky"),
        "expected reflection in output, got: {stdout}"
    );
}

#[test]
fn stats_on_empty_db() {
    let dir = tempfile::tempdir().unwrap();

    let output = legion_cmd(dir.path()).args(["stats"]).output().unwrap();
    assert!(
        output.status.success(),
        "stats failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("no reflections"),
        "expected empty message, got: {stdout}"
    );
}

#[test]
fn reflect_no_input_errors() {
    let dir = tempfile::tempdir().unwrap();

    let output = legion_cmd(dir.path())
        .args(["reflect", "--repo", "test"])
        .output()
        .unwrap();
    // clap allows the call but the binary returns an error since
    // neither --text nor --transcript is provided.
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("NoReflectionInput"),
        "expected missing input error, got: {stderr}"
    );
}

#[test]
fn stats_after_reflections() {
    let dir = tempfile::tempdir().unwrap();

    let out = legion_cmd(dir.path())
        .args(["reflect", "--repo", "kelex", "--text", "first reflection"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "reflect 1 failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = legion_cmd(dir.path())
        .args(["reflect", "--repo", "kelex", "--text", "second reflection"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "reflect 2 failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = legion_cmd(dir.path())
        .args([
            "reflect",
            "--repo",
            "rafters",
            "--text",
            "rafters reflection",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "reflect 3 failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let output = legion_cmd(dir.path()).args(["stats"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("kelex"), "should show kelex stats");
    assert!(stdout.contains("rafters"), "should show rafters stats");
}

#[test]
fn recall_with_no_matches() {
    let dir = tempfile::tempdir().unwrap();

    let out = legion_cmd(dir.path())
        .args([
            "reflect",
            "--repo",
            "test",
            "--text",
            "rust ownership rules",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "reflect failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let output = legion_cmd(dir.path())
        .args([
            "recall",
            "--repo",
            "other-repo",
            "--context",
            "rust ownership",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    // Should succeed but return empty since repo does not match
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        !stdout.contains("rust ownership"),
        "should not find results in different repo"
    );
}

#[test]
fn data_dir_is_created_automatically() {
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("deep").join("nested").join("dir");

    let output = legion_cmd(&nested).args(["stats"]).output().unwrap();
    assert!(
        output.status.success(),
        "should create nested dirs: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(nested.exists(), "data dir should have been created");
}

#[test]
fn consult_across_repos() {
    let dir = tempfile::tempdir().unwrap();

    // Reflect into two different repos
    let out = legion_cmd(dir.path())
        .args([
            "reflect",
            "--repo",
            "kelex",
            "--text",
            "Zod schema mapping is fragile",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "reflect kelex failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let out = legion_cmd(dir.path())
        .args([
            "reflect",
            "--repo",
            "platform",
            "--text",
            "Zod validation at the edge works well",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "reflect platform failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Consult across all repos
    let output = legion_cmd(dir.path())
        .args(["consult", "--context", "Zod", "--limit", "10"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "consult failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("[kelex]"),
        "expected [kelex] in output, got: {stdout}"
    );
    assert!(
        stdout.contains("[platform]"),
        "expected [platform] in output, got: {stdout}"
    );
    assert!(
        stdout.contains("Cross-repo reflections"),
        "expected header in output, got: {stdout}"
    );
}

#[test]
fn cli_compound_repo_flag() {
    let dir = tempfile::tempdir().unwrap();

    let output = legion_cmd(dir.path())
        .args([
            "reflect",
            "--repo",
            "platform,legion",
            "--text",
            "compound test reflection",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "compound reflect failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("stored reflection for platform"),
        "expected platform confirmation, got: {stderr}"
    );
    assert!(
        stderr.contains("stored reflection for legion"),
        "expected legion confirmation, got: {stderr}"
    );

    // Verify both repos have the reflection via recall
    let output = legion_cmd(dir.path())
        .args(["recall", "--repo", "platform", "--context", "compound test"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("compound test reflection"),
        "expected reflection in platform recall, got: {stdout}"
    );

    let output = legion_cmd(dir.path())
        .args(["recall", "--repo", "legion", "--context", "compound test"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("compound test reflection"),
        "expected reflection in legion recall, got: {stdout}"
    );
}

#[test]
fn cli_single_repo_still_works() {
    let dir = tempfile::tempdir().unwrap();

    let output = legion_cmd(dir.path())
        .args([
            "reflect",
            "--repo",
            "platform",
            "--text",
            "single repo test",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "single repo reflect failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("stored reflection for platform"),
        "expected confirmation, got: {stderr}"
    );
}

#[test]
fn post_and_board_roundtrip() {
    let dir = tempfile::tempdir().unwrap();

    // Post a message
    let out = legion_cmd(dir.path())
        .args([
            "post",
            "--repo",
            "kelex",
            "--text",
            "shared insight about schema parsing",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "post failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("posted to board for kelex"),
        "expected post confirmation, got: {stderr}"
    );

    // Read the board from a different repo
    let output = legion_cmd(dir.path())
        .args(["board", "--repo", "rafters"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "board failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("[kelex]"),
        "expected repo attribution, got: {stdout}"
    );
    assert!(
        stdout.contains("shared insight about schema parsing"),
        "expected post text, got: {stdout}"
    );
    assert!(
        stdout.contains("[Legion] Board"),
        "expected board header, got: {stdout}"
    );
}

#[test]
fn board_count_output() {
    let dir = tempfile::tempdir().unwrap();

    // Post two messages
    let out = legion_cmd(dir.path())
        .args(["post", "--repo", "kelex", "--text", "first shared thought"])
        .output()
        .unwrap();
    assert!(out.status.success());

    let out = legion_cmd(dir.path())
        .args([
            "post",
            "--repo",
            "rafters",
            "--text",
            "second shared thought",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());

    // Check count from a reader that has not read the board
    let output = legion_cmd(dir.path())
        .args(["board", "--repo", "platform", "--count"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "board count failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("2 unread posts on the board"),
        "expected unread count, got: {stdout}"
    );

    // Read the board to mark as read
    let out = legion_cmd(dir.path())
        .args(["board", "--repo", "platform"])
        .output()
        .unwrap();
    assert!(out.status.success());

    // Count should now be zero (no output)
    let output = legion_cmd(dir.path())
        .args(["board", "--repo", "platform", "--count"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.is_empty(),
        "expected no output for zero unread, got: {stdout}"
    );
}

#[test]
fn consult_no_matches() {
    let dir = tempfile::tempdir().unwrap();

    // Reflect something so the DB/index exist
    let out = legion_cmd(dir.path())
        .args([
            "reflect",
            "--repo",
            "test",
            "--text",
            "rust ownership rules",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "reflect failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Consult with a term that will not match
    let output = legion_cmd(dir.path())
        .args(["consult", "--context", "nonexistent_term_xyz"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "consult should succeed even with no matches: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("no reflections matched"),
        "expected no-match message on stderr, got: {stderr}"
    );
}
