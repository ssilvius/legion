use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::{LegionError, Result};

const RECALL_SCRIPT: &str = r#"#!/bin/bash
# Legion SessionStart hook: recall reflections + surface cross-repo highlights
INPUT=$(cat)
CWD=$(echo "$INPUT" | jq -r '.cwd // empty')

if [ -z "$CWD" ]; then
  exit 0
fi

REPO=$(basename "$CWD")

# Try BM25 search with git branch context first
BRANCH=$(cd "$CWD" && git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "")

OUTPUT=""
if [ -n "$BRANCH" ] && [ "$BRANCH" != "main" ] && [ "$BRANCH" != "master" ]; then
  OUTPUT=$(legion recall --repo "$REPO" --context "$BRANCH" 2>/dev/null)
fi

# Fall back to latest reflections if BM25 found nothing
if [ -z "$OUTPUT" ]; then
  OUTPUT=$(legion recall --repo "$REPO" --latest 2>/dev/null)
fi

# Surface cross-repo highlights (bullpen posts, high-value reflections, chains)
SURFACE=$(legion surface --repo "$REPO" 2>/dev/null)
if [ -n "$SURFACE" ]; then
  if [ -n "$OUTPUT" ]; then
    OUTPUT="$OUTPUT"$'\n\n'"$SURFACE"
  else
    OUTPUT="$SURFACE"
  fi
fi

if [ -n "$OUTPUT" ]; then
  jq -n --arg ctx "$OUTPUT" '{
    "hookSpecificOutput": {
      "hookEventName": "SessionStart",
      "additionalContext": $ctx
    }
  }'
fi
"#;

const REFLECT_SCRIPT: &str = r#"#!/bin/bash
# Legion Stop hook: prompt the agent to reflect before closing
INPUT=$(cat)
STOP_HOOK_ACTIVE=$(echo "$INPUT" | jq -r '.stop_hook_active // false')
CWD=$(echo "$INPUT" | jq -r '.cwd // empty')

# Prevent infinite loops: if this is a re-stop after reflection, let it through
if [ "$STOP_HOOK_ACTIVE" = "true" ]; then
  exit 0
fi

if [ -z "$CWD" ]; then
  exit 0
fi

REPO=$(basename "$CWD")

jq -n --arg reason "Before you stop, reflect on this session. What would you tell another agent who hits this same problem tomorrow? Store your reflection with: legion reflect --repo $REPO --text '<your reflection here>'" '{
  "decision": "block",
  "reason": $reason
}'
"#;

/// Resolve the Claude home directory (~/.claude).
fn claude_home() -> Result<PathBuf> {
    let home: PathBuf = dirs::home_dir().ok_or(LegionError::NoHomeDir)?;
    Ok(home.join(".claude"))
}

/// Run the init command: write hook scripts and update settings.json.
pub fn init(force: bool) -> Result<()> {
    let claude_dir: PathBuf = claude_home()?;
    let hooks_dir: PathBuf = claude_dir.join("hooks");
    let settings_path: PathBuf = claude_dir.join("settings.json");

    if !force {
        eprint!("[legion] This will write hook scripts and update settings.json. Continue? [y/N] ");
        std::io::stderr().flush()?;

        let mut answer: String = String::new();
        std::io::stdin().read_line(&mut answer)?;
        let trimmed: &str = answer.trim();
        if !trimmed.eq_ignore_ascii_case("y") && !trimmed.eq_ignore_ascii_case("yes") {
            eprintln!("[legion] Aborted.");
            return Ok(());
        }
    }

    // Create hooks directory
    if !hooks_dir.exists() {
        eprintln!("[legion] Creating hooks directory: {}", hooks_dir.display());
        std::fs::create_dir_all(&hooks_dir)?;
    }

    // Write hook scripts
    write_hook_script(&hooks_dir.join("legion-recall.sh"), RECALL_SCRIPT)?;
    write_hook_script(&hooks_dir.join("legion-reflect.sh"), REFLECT_SCRIPT)?;

    // Update settings.json
    eprintln!("[legion] Updating {}", settings_path.display());
    update_settings(&settings_path, &hooks_dir)?;

    eprintln!("[legion] Done. Legion hooks configured for Claude Code.");
    Ok(())
}

/// Write a hook script to disk with executable permissions.
fn write_hook_script(path: &Path, content: &str) -> Result<()> {
    let filename: &str = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("hook script");

    if path.exists() {
        let existing: String = std::fs::read_to_string(path)?;
        if existing == content {
            eprintln!("[legion] {} already up to date", filename);
            return Ok(());
        }
        eprintln!("[legion] Overwriting {} (content changed)", filename);
    } else {
        eprintln!("[legion] Writing {}", filename);
    }

    std::fs::write(path, content)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms: std::fs::Permissions = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(path, perms)?;
    }

    Ok(())
}

/// Read, merge, and write back settings.json with legion hook entries.
fn update_settings(settings_path: &Path, hooks_dir: &Path) -> Result<()> {
    let mut root: serde_json::Value = if settings_path.exists() {
        let raw: String = std::fs::read_to_string(settings_path)?;
        serde_json::from_str(&raw).map_err(|e| {
            LegionError::MalformedSettings(format!(
                "failed to parse {}: {} -- fix or delete the file and re-run",
                settings_path.display(),
                e
            ))
        })?
    } else {
        serde_json::json!({})
    };

    let obj: &mut serde_json::Map<String, serde_json::Value> =
        root.as_object_mut().ok_or_else(|| {
            LegionError::MalformedSettings("settings.json root is not a JSON object".to_string())
        })?;

    let hooks: &mut serde_json::Map<String, serde_json::Value> = obj
        .entry("hooks")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .ok_or_else(|| {
            LegionError::MalformedSettings("hooks key is not a JSON object".to_string())
        })?;

    let recall_path: String = hooks_dir
        .join("legion-recall.sh")
        .to_string_lossy()
        .into_owned();
    let reflect_path: String = hooks_dir
        .join("legion-reflect.sh")
        .to_string_lossy()
        .into_owned();

    merge_hook_entry(
        hooks,
        "SessionStart",
        &recall_path,
        Some("startup"),
        Some(10),
    );
    merge_hook_entry(hooks, "Stop", &reflect_path, None, Some(10));

    let formatted: String = serde_json::to_string_pretty(&root)?;
    std::fs::write(settings_path, formatted.as_bytes())?;

    Ok(())
}

/// Merge a single legion hook entry into the hooks map, preserving non-legion entries.
fn merge_hook_entry(
    hooks: &mut serde_json::Map<String, serde_json::Value>,
    event: &str,
    command: &str,
    matcher: Option<&str>,
    timeout: Option<u64>,
) {
    let existing: &mut serde_json::Value =
        hooks.entry(event).or_insert_with(|| serde_json::json!([]));

    let arr: &mut Vec<serde_json::Value> = match existing.as_array_mut() {
        Some(a) => a,
        None => {
            // If it is not an array, replace with empty array
            *existing = serde_json::json!([]);
            existing.as_array_mut().expect("just created array")
        }
    };

    // Remove any existing legion entries (identified by command path containing "legion-")
    arr.retain(|entry| !is_legion_hook_entry(entry));

    // Build the new legion entry
    let mut hook_obj: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
    hook_obj.insert("type".to_string(), serde_json::json!("command"));
    hook_obj.insert("command".to_string(), serde_json::json!(command));
    if let Some(t) = timeout {
        hook_obj.insert("timeout".to_string(), serde_json::json!(t));
    }

    let mut entry: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
    if let Some(m) = matcher {
        entry.insert("matcher".to_string(), serde_json::json!(m));
    }
    entry.insert(
        "hooks".to_string(),
        serde_json::json!([serde_json::Value::Object(hook_obj)]),
    );

    arr.push(serde_json::Value::Object(entry));
}

/// Check if a hook entry belongs to legion (contains "legion-" in any command path).
fn is_legion_hook_entry(entry: &serde_json::Value) -> bool {
    if let Some(hooks_arr) = entry.get("hooks").and_then(|h| h.as_array()) {
        for hook in hooks_arr {
            if let Some(cmd) = hook.get("command").and_then(|c| c.as_str())
                && cmd.contains("legion-")
            {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().expect("failed to create temp dir")
    }

    #[test]
    fn init_creates_hook_scripts_with_correct_content_and_permissions() {
        let tmp: tempfile::TempDir = setup_temp_dir();
        let hooks_dir: PathBuf = tmp.path().join("hooks");
        fs::create_dir_all(&hooks_dir).expect("create hooks dir");

        let recall_path: PathBuf = hooks_dir.join("legion-recall.sh");
        let reflect_path: PathBuf = hooks_dir.join("legion-reflect.sh");

        write_hook_script(&recall_path, RECALL_SCRIPT).expect("write recall");
        write_hook_script(&reflect_path, REFLECT_SCRIPT).expect("write reflect");

        let recall_content: String = fs::read_to_string(&recall_path).expect("read recall");
        let reflect_content: String = fs::read_to_string(&reflect_path).expect("read reflect");

        assert_eq!(recall_content, RECALL_SCRIPT);
        assert_eq!(reflect_content, REFLECT_SCRIPT);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let recall_mode: u32 = fs::metadata(&recall_path)
                .expect("recall metadata")
                .permissions()
                .mode();
            assert_eq!(recall_mode & 0o777, 0o755);

            let reflect_mode: u32 = fs::metadata(&reflect_path)
                .expect("reflect metadata")
                .permissions()
                .mode();
            assert_eq!(reflect_mode & 0o777, 0o755);
        }
    }

    #[test]
    fn init_merges_settings_preserving_existing_hooks() {
        let tmp: tempfile::TempDir = setup_temp_dir();
        let hooks_dir: PathBuf = tmp.path().join("hooks");
        fs::create_dir_all(&hooks_dir).expect("create hooks dir");

        let settings_path: PathBuf = tmp.path().join("settings.json");

        // Write settings with an existing non-legion hook
        let initial: serde_json::Value = serde_json::json!({
            "permissions": {"allow": ["Bash(*)"]},
            "hooks": {
                "SessionStart": [
                    {
                        "matcher": "startup",
                        "hooks": [
                            {
                                "type": "command",
                                "command": "/some/other/hook.sh",
                                "timeout": 5
                            }
                        ]
                    }
                ]
            }
        });
        fs::write(
            &settings_path,
            serde_json::to_string_pretty(&initial).expect("serialize"),
        )
        .expect("write settings");

        update_settings(&settings_path, &hooks_dir).expect("update settings");

        let result: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&settings_path).expect("read settings"))
                .expect("parse settings");

        // Existing hook preserved
        let session_start: &Vec<serde_json::Value> = result["hooks"]["SessionStart"]
            .as_array()
            .expect("SessionStart array");
        assert_eq!(
            session_start.len(),
            2,
            "should have existing + legion entry"
        );

        // First entry is the existing non-legion hook
        assert_eq!(
            session_start[0]["hooks"][0]["command"]
                .as_str()
                .expect("command str"),
            "/some/other/hook.sh"
        );

        // Second entry is the legion hook
        let legion_cmd: &str = session_start[1]["hooks"][0]["command"]
            .as_str()
            .expect("legion command");
        assert!(legion_cmd.contains("legion-recall.sh"));

        // Stop hook also added
        let stop: &Vec<serde_json::Value> = result["hooks"]["Stop"].as_array().expect("Stop array");
        assert_eq!(stop.len(), 1);
        let stop_cmd: &str = stop[0]["hooks"][0]["command"]
            .as_str()
            .expect("stop command");
        assert!(stop_cmd.contains("legion-reflect.sh"));

        // Permissions preserved
        assert!(result["permissions"]["allow"].is_array());
    }

    #[test]
    fn init_is_idempotent() {
        let tmp: tempfile::TempDir = setup_temp_dir();
        let hooks_dir: PathBuf = tmp.path().join("hooks");
        fs::create_dir_all(&hooks_dir).expect("create hooks dir");

        let settings_path: PathBuf = tmp.path().join("settings.json");
        fs::write(&settings_path, "{}").expect("write empty settings");

        // Run twice
        update_settings(&settings_path, &hooks_dir).expect("first update");
        update_settings(&settings_path, &hooks_dir).expect("second update");

        let result: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&settings_path).expect("read settings"))
                .expect("parse settings");

        // Should have exactly one entry per hook event, not duplicates
        let session_start: &Vec<serde_json::Value> = result["hooks"]["SessionStart"]
            .as_array()
            .expect("SessionStart array");
        assert_eq!(session_start.len(), 1, "no duplicate SessionStart entries");

        let stop: &Vec<serde_json::Value> = result["hooks"]["Stop"].as_array().expect("Stop array");
        assert_eq!(stop.len(), 1, "no duplicate Stop entries");
    }

    #[test]
    fn init_creates_missing_directories() {
        let tmp: tempfile::TempDir = setup_temp_dir();
        let hooks_dir: PathBuf = tmp.path().join("deep").join("nested").join("hooks");

        // Directory does not exist yet
        assert!(!hooks_dir.exists());

        fs::create_dir_all(&hooks_dir).expect("create hooks dir");
        assert!(hooks_dir.exists());

        let recall_path: PathBuf = hooks_dir.join("legion-recall.sh");
        write_hook_script(&recall_path, RECALL_SCRIPT).expect("write recall");
        assert!(recall_path.exists());
    }

    #[test]
    fn init_creates_settings_when_missing() {
        let tmp: tempfile::TempDir = setup_temp_dir();
        let hooks_dir: PathBuf = tmp.path().join("hooks");
        fs::create_dir_all(&hooks_dir).expect("create hooks dir");

        let settings_path: PathBuf = tmp.path().join("settings.json");
        // Do not create the file -- it should be created by update_settings
        assert!(!settings_path.exists());

        update_settings(&settings_path, &hooks_dir).expect("update settings");

        assert!(settings_path.exists());
        let result: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&settings_path).expect("read"))
                .expect("parse");
        assert!(result["hooks"]["SessionStart"].is_array());
        assert!(result["hooks"]["Stop"].is_array());
    }

    #[test]
    fn malformed_settings_returns_error() {
        let tmp: tempfile::TempDir = setup_temp_dir();
        let hooks_dir: PathBuf = tmp.path().join("hooks");
        fs::create_dir_all(&hooks_dir).expect("create hooks dir");

        let settings_path: PathBuf = tmp.path().join("settings.json");
        fs::write(&settings_path, "not valid json {{{").expect("write bad json");

        let result = update_settings(&settings_path, &hooks_dir);
        assert!(result.is_err());
        let err_msg: String = result.unwrap_err().to_string();
        assert!(err_msg.contains("failed to parse"), "got: {}", err_msg);
    }

    #[test]
    fn hook_script_overwrite_when_content_differs() {
        let tmp: tempfile::TempDir = setup_temp_dir();
        let path: PathBuf = tmp.path().join("legion-recall.sh");

        // Write old content
        fs::write(&path, "#!/bin/bash\nold content").expect("write old");

        // Overwrite with new content
        write_hook_script(&path, RECALL_SCRIPT).expect("overwrite");

        let content: String = fs::read_to_string(&path).expect("read");
        assert_eq!(content, RECALL_SCRIPT);
    }
}
