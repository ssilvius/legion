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
fn post_and_bullpen_roundtrip() {
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
        stderr.contains("posted to bullpen for kelex"),
        "expected post confirmation, got: {stderr}"
    );

    // Read the bullpen from a different repo
    let output = legion_cmd(dir.path())
        .args(["bullpen", "--repo", "rafters"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "bullpen failed: {}",
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
        stdout.contains("[Legion] Bullpen"),
        "expected bullpen header, got: {stdout}"
    );
}

#[test]
fn bullpen_count_output() {
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

    // Check count from a reader that has not read the bullpen
    let output = legion_cmd(dir.path())
        .args(["bullpen", "--repo", "platform", "--count"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "bullpen count failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("2 unread posts on the bullpen"),
        "expected unread count, got: {stdout}"
    );

    // Read the bullpen to mark as read
    let out = legion_cmd(dir.path())
        .args(["bullpen", "--repo", "platform"])
        .output()
        .unwrap();
    assert!(out.status.success());

    // Count should now be zero (no output)
    let output = legion_cmd(dir.path())
        .args(["bullpen", "--repo", "platform", "--count"])
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
fn reindex_rebuilds_from_database() {
    let dir = tempfile::tempdir().unwrap();

    // Create some reflections
    let out = legion_cmd(dir.path())
        .args([
            "reflect",
            "--repo",
            "test",
            "--text",
            "reindex test reflection about search",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "reflect failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Run reindex
    let output = legion_cmd(dir.path()).args(["reindex"]).output().unwrap();
    assert!(
        output.status.success(),
        "reindex failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("reindexed 1 reflections"),
        "expected reindex count, got: {stderr}"
    );

    // Verify search still works after reindex
    let output = legion_cmd(dir.path())
        .args(["recall", "--repo", "test", "--context", "search"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("reindex test reflection"),
        "expected reflection after reindex, got: {stdout}"
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

#[test]
fn reflect_with_metadata_flags() {
    let dir = tempfile::tempdir().unwrap();

    let output = legion_cmd(dir.path())
        .args([
            "reflect",
            "--repo",
            "kelex",
            "--text",
            "oklch color tokens work well",
            "--domain",
            "color-tokens",
            "--tags",
            "semantic-tokens,consumer",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "reflect with meta failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("stored reflection for kelex"),
        "expected confirmation, got: {stderr}"
    );
}

#[test]
fn boost_and_chain_roundtrip() {
    let dir = tempfile::tempdir().unwrap();

    // Create a reflection
    let out = legion_cmd(dir.path())
        .args([
            "reflect",
            "--repo",
            "kelex",
            "--text",
            "first insight in a chain",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    // Extract the ID from the first line of stderr: "stored reflection for kelex (UUID)"
    let first_line = stderr.lines().next().unwrap_or("");
    let id = first_line
        .rsplit('(')
        .next()
        .unwrap()
        .trim_end_matches(')')
        .to_string();

    // Boost the reflection
    let output = legion_cmd(dir.path())
        .args(["boost", "--id", &id])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "boost failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("boosted reflection"),
        "expected boost confirmation, got: {stderr}"
    );

    // Chain with a single node
    let output = legion_cmd(dir.path())
        .args(["chain", "--id", &id])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "chain failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("first insight"),
        "expected chain output, got: {stderr}"
    );
}

#[test]
fn chain_with_follows() {
    let dir = tempfile::tempdir().unwrap();

    // Create parent reflection
    let out = legion_cmd(dir.path())
        .args(["reflect", "--repo", "kelex", "--text", "root of the chain"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    let first_line = stderr.lines().next().unwrap_or("");
    let parent_id = first_line
        .rsplit('(')
        .next()
        .unwrap()
        .trim_end_matches(')')
        .to_string();

    // Create child reflection with --follows
    let out = legion_cmd(dir.path())
        .args([
            "reflect",
            "--repo",
            "kelex",
            "--text",
            "builds on root",
            "--follows",
            &parent_id,
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "child reflect failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    let first_line = stderr.lines().next().unwrap_or("");
    let child_id = first_line
        .rsplit('(')
        .next()
        .unwrap()
        .trim_end_matches(')')
        .to_string();

    // Chain from child should show both
    let output = legion_cmd(dir.path())
        .args(["chain", "--id", &child_id])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("root of the chain"),
        "expected parent in chain, got: {stderr}"
    );
    assert!(
        stderr.contains("builds on root"),
        "expected child in chain, got: {stderr}"
    );
}

#[test]
fn boost_nonexistent_id() {
    let dir = tempfile::tempdir().unwrap();

    // Need to create the DB first
    let out = legion_cmd(dir.path())
        .args(["reflect", "--repo", "test", "--text", "setup"])
        .output()
        .unwrap();
    assert!(out.status.success());

    let output = legion_cmd(dir.path())
        .args(["boost", "--id", "nonexistent-uuid"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "boost should succeed even for missing ID: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("reflection not found"),
        "expected not-found message, got: {stderr}"
    );
}

#[test]
fn post_with_metadata_flags() {
    let dir = tempfile::tempdir().unwrap();

    let output = legion_cmd(dir.path())
        .args([
            "post",
            "--repo",
            "rafters",
            "--text",
            "shared domain knowledge",
            "--domain",
            "auth",
            "--tags",
            "security,jwt",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "post with meta failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("posted to bullpen for rafters"),
        "expected post confirmation, got: {stderr}"
    );

    // Verify it shows up on the bullpen
    let output = legion_cmd(dir.path())
        .args(["bullpen", "--repo", "kelex"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("shared domain knowledge"),
        "expected post on bullpen, got: {stdout}"
    );
}

#[test]
fn surface_shows_recent_posts() {
    let dir = tempfile::tempdir().unwrap();

    // Post to the bullpen
    let out = legion_cmd(dir.path())
        .args(["post", "--repo", "rafters", "--text", "synapse insight"])
        .output()
        .unwrap();
    assert!(out.status.success());

    // Surface for a different repo should show the post
    let output = legion_cmd(dir.path())
        .args(["surface", "--repo", "kelex"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "surface failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("[Synapse]"),
        "expected synapse header, got: {stdout}"
    );
    assert!(
        stdout.contains("synapse insight"),
        "expected post in surface output, got: {stdout}"
    );
}

#[test]
fn surface_empty_database() {
    let dir = tempfile::tempdir().unwrap();

    // Need to initialize the DB first
    let out = legion_cmd(dir.path())
        .args(["reflect", "--repo", "test", "--text", "setup"])
        .output()
        .unwrap();
    assert!(out.status.success());

    let output = legion_cmd(dir.path())
        .args(["surface", "--repo", "kelex"])
        .output()
        .unwrap();
    assert!(output.status.success());
    // No bullpen posts, no high-value, no chains -- should be empty
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.is_empty(),
        "expected empty surface for no highlights, got: {stdout}"
    );
}

#[test]
fn bullpen_aliases_backward_compatible() {
    let dir = tempfile::tempdir().unwrap();

    // Seed a post
    let out = legion_cmd(dir.path())
        .args(["post", "--repo", "kelex", "--text", "alias test"])
        .output()
        .unwrap();
    assert!(out.status.success());

    // Old "board" alias still works
    let output = legion_cmd(dir.path())
        .args(["board", "--repo", "rafters"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "board alias should still work: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Short "bp" alias works
    let output = legion_cmd(dir.path())
        .args(["bp", "--repo", "rafters"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "bp alias should work: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn signal_command_posts_formatted_signal() {
    let dir = tempfile::tempdir().unwrap();

    let output = legion_cmd(dir.path())
        .args([
            "signal", "--repo", "kelex", "--to", "legion", "--verb", "review", "--status",
            "approved", "--note", "ship it",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "signal failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify signal appears on the bullpen
    let output = legion_cmd(dir.path())
        .args(["bullpen", "--repo", "platform"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("@legion"),
        "expected signal recipient on bullpen, got: {stdout}"
    );
    assert!(
        stdout.contains("review"),
        "expected signal verb on bullpen, got: {stdout}"
    );
}

#[test]
fn signal_with_details() {
    let dir = tempfile::tempdir().unwrap();

    let output = legion_cmd(dir.path())
        .args([
            "signal",
            "--repo",
            "kelex",
            "--to",
            "legion",
            "--verb",
            "review",
            "--status",
            "approved",
            "--details",
            "surface:cap-output,chain:confirmed",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "signal with details failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn bullpen_signals_filter() {
    let dir = tempfile::tempdir().unwrap();

    // Post a signal
    legion_cmd(dir.path())
        .args([
            "signal", "--repo", "kelex", "--to", "legion", "--verb", "review", "--status",
            "approved",
        ])
        .output()
        .unwrap();

    // Post a musing
    legion_cmd(dir.path())
        .args([
            "post",
            "--repo",
            "rafters",
            "--text",
            "deep thoughts about design patterns",
        ])
        .output()
        .unwrap();

    // --signals should show only the signal
    let output = legion_cmd(dir.path())
        .args(["bullpen", "--repo", "platform", "--signals"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("@legion"), "expected signal, got: {stdout}");
    assert!(
        !stdout.contains("deep thoughts"),
        "expected no musings in --signals, got: {stdout}"
    );

    // --musings should show only the musing
    let output = legion_cmd(dir.path())
        .args(["bullpen", "--repo", "courses", "--musings"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("deep thoughts"),
        "expected musing, got: {stdout}"
    );
    assert!(
        !stdout.contains("@legion"),
        "expected no signals in --musings, got: {stdout}"
    );
}
