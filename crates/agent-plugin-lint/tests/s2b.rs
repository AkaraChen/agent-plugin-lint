use agent_plugin_lint::{InputMode, LintOptions, lint_path};
use serde_json::json;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

const SCHEMA: &str = "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json";

fn write_manifest(root: &Path, extra: serde_json::Value) {
    let mut manifest = json!({"$schema": SCHEMA, "name": "a"});
    for (key, value) in extra.as_object().unwrap() {
        manifest[key] = value.clone();
    }
    fs::write(
        root.join("plugin.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
}
fn options(mode: InputMode) -> LintOptions {
    LintOptions {
        mode,
        strict: false,
    }
}

#[cfg(unix)]
fn complete_within<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let result = work();
        let _ = sender.send(result);
    });
    receiver
        .recv_timeout(std::time::Duration::from_secs(2))
        .expect("FIFO lint exceeded the watchdog deadline")
}

#[cfg(unix)]
fn non_utf8_component() -> std::ffi::OsString {
    use std::os::unix::ffi::OsStringExt;
    std::ffi::OsString::from_vec(b"bad\xFF".to_vec())
}

#[test]
fn explicit_plugin_without_manifest_is_a_content_failure() {
    let temp = TempDir::new().unwrap();
    let report = lint_path(temp.path(), options(InputMode::Plugin));
    assert_eq!(report.exit_code, 1);
    assert!(report.errors.is_empty());
    assert_eq!(
        report.plugins[0].findings[0].rule_id.as_str(),
        "AP-MANIFEST-LOCATION"
    );
    assert_eq!(
        report.plugins[0]
            .coverage
            .iter()
            .find(|coverage| coverage.rule_id == "AP-MANIFEST-LOCATION")
            .unwrap()
            .status,
        agent_plugin_lint::CoverageStatus::Fail
    );
    assert!(
        report.plugins[0]
            .coverage
            .iter()
            .filter(|coverage| {
                coverage.rule_id == "AP-SKILL-CONFORMANCE"
                    || coverage.rule_id == "AP-MCP-ENVELOPE"
                    || coverage.rule_id == "AP-EXTENSIONS-OBJECT"
            })
            .all(|coverage| coverage.status == agent_plugin_lint::CoverageStatus::Blocked)
    );
}

#[test]
fn auto_rejects_mixed_directories_and_collection_reports_missing_manifest() {
    let temp = TempDir::new().unwrap();
    let good = temp.path().join("good");
    let missing = temp.path().join("missing");
    fs::create_dir(&good).unwrap();
    fs::create_dir(&missing).unwrap();
    write_manifest(&good, json!({}));
    assert_eq!(
        lint_path(temp.path(), options(InputMode::Auto)).exit_code,
        2
    );
    let collection = lint_path(temp.path(), options(InputMode::Collection));
    assert_eq!(collection.exit_code, 1);
    assert_eq!(collection.plugins.len(), 2);
    assert!(collection.plugins.iter().any(|plugin| {
        plugin
            .findings
            .iter()
            .any(|finding| finding.rule_id.as_str() == "AP-MANIFEST-LOCATION")
    }));
}

#[test]
fn collection_with_no_candidates_is_an_operational_error() {
    let temp = TempDir::new().unwrap();
    let report = lint_path(temp.path(), options(InputMode::Collection));
    assert_eq!(report.exit_code, 2);
    assert!(!report.complete);
}

#[test]
fn invalid_manifest_blocks_unimplemented_component_checks() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("plugin.json"), b"{not json").unwrap();
    let report = lint_path(temp.path(), options(InputMode::Plugin));
    let plugin = &report.plugins[0];
    assert!(
        plugin
            .coverage
            .iter()
            .filter(|coverage| coverage.rule_id == "AP-SKILL-CONFORMANCE"
                || coverage.rule_id == "AP-MCP-ENVELOPE")
            .all(|coverage| coverage.status == agent_plugin_lint::CoverageStatus::Blocked)
    );
    assert_eq!(
        plugin
            .coverage
            .iter()
            .find(|coverage| coverage.rule_id == "AP-ADVICE-DUPLICATE-JSON-KEY")
            .unwrap()
            .status,
        agent_plugin_lint::CoverageStatus::Unchecked
    );
}

#[test]
fn batch_keeps_valid_plugin_when_another_is_missing_manifest() {
    let temp = TempDir::new().unwrap();
    let good = temp.path().join("good");
    let missing = temp.path().join("missing");
    fs::create_dir(&good).unwrap();
    fs::create_dir(&missing).unwrap();
    write_manifest(&good, json!({}));
    let report = lint_path(temp.path(), options(InputMode::Collection));
    assert_eq!(report.plugins.len(), 2);
    assert!(
        report
            .plugins
            .iter()
            .any(|plugin| plugin.name.as_deref() == Some("a"))
    );
}

#[test]
fn repeated_reports_are_byte_stable() {
    let temp = TempDir::new().unwrap();
    write_manifest(temp.path(), json!({"unknown": 3}));
    let first = serde_json::to_string(&lint_path(temp.path(), options(InputMode::Plugin))).unwrap();
    let second =
        serde_json::to_string(&lint_path(temp.path(), options(InputMode::Plugin))).unwrap();
    assert_eq!(first, second);
}

#[test]
fn strict_does_not_promote_duplicate_key_check_when_a_document_was_checked() {
    let temp = TempDir::new().unwrap();
    write_manifest(temp.path(), json!({}));
    let report = lint_path(
        temp.path(),
        LintOptions {
            mode: InputMode::Plugin,
            strict: true,
        },
    );
    assert_eq!(report.exit_code, 0);
}

#[test]
fn json_argument_errors_are_one_document() {
    let binary = env!("CARGO_BIN_EXE_ap-lint");
    let output = Command::new(binary)
        .args(["--json", "--bogus"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.ends_with("\n"));
    let report: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(report["exitCode"], 2);
}

#[test]
fn argument_errors_do_not_scan_cwd_and_conflicts_are_explicit() {
    let binary = env!("CARGO_BIN_EXE_ap-lint");
    let valid = TempDir::new().unwrap();
    write_manifest(valid.path(), json!({}));
    let bad_cwd = TempDir::new().unwrap();
    fs::write(bad_cwd.path().join("plugin.json"), b"{").unwrap();
    for args in [
        vec![
            valid.path().as_os_str(),
            std::ffi::OsStr::new("--bogus"),
            std::ffi::OsStr::new("--json"),
        ],
        vec![
            valid.path().as_os_str(),
            std::ffi::OsStr::new("--mode"),
            std::ffi::OsStr::new("plugin"),
            std::ffi::OsStr::new("--mode"),
            std::ffi::OsStr::new("collection"),
            std::ffi::OsStr::new("--json"),
        ],
        vec![
            valid.path().as_os_str(),
            std::ffi::OsStr::new("--spec"),
            std::ffi::OsStr::new("1.0.0"),
            std::ffi::OsStr::new("--spec"),
            std::ffi::OsStr::new("1.0.0"),
            std::ffi::OsStr::new("--json"),
        ],
    ] {
        let output = Command::new(binary)
            .args(args)
            .current_dir(bad_cwd.path())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["errors"][0]["code"], "ARGUMENT");
        assert_eq!(
            report["summary"],
            json!({"fatal": 0, "component": 0, "ignored": 0, "advisory": 0, "errors": 1})
        );
        assert_eq!(report["plugins"], json!([]));
    }
}

#[test]
fn roots_are_relative_to_the_input() {
    let single = TempDir::new().unwrap();
    write_manifest(single.path(), json!({}));
    assert_eq!(
        lint_path(single.path(), options(InputMode::Plugin)).plugins[0].root,
        "."
    );
    let collection = TempDir::new().unwrap();
    let child = collection.path().join("one");
    fs::create_dir(&child).unwrap();
    write_manifest(&child, json!({}));
    assert_eq!(
        lint_path(collection.path(), options(InputMode::Collection)).plugins[0].root,
        "one"
    );
}

#[cfg(unix)]
#[test]
fn non_utf8_paths_are_operational_errors() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join(non_utf8_component());
    let report = lint_path(&path, options(InputMode::Plugin));
    assert_eq!(report.exit_code, 2);
    assert!(
        report
            .errors
            .iter()
            .any(|error| error.code == "PATH_ENCODING")
    );
    assert_eq!(report.input, "<non-utf8-path>");
}

#[cfg(unix)]
#[test]
fn manifest_link_loops_are_unresolved_location_failures() {
    use std::os::unix::fs::symlink;
    for two_nodes in [false, true] {
        let temp = TempDir::new().unwrap();
        if two_nodes {
            symlink("other", temp.path().join("plugin.json")).unwrap();
            symlink("plugin.json", temp.path().join("other")).unwrap();
        } else {
            symlink("plugin.json", temp.path().join("plugin.json")).unwrap();
        }
        let report = lint_path(temp.path(), options(InputMode::Plugin));
        assert_eq!(report.exit_code, 1);
        assert!(report.errors.is_empty());
        assert!(
            report.plugins[0]
                .findings
                .iter()
                .any(|finding| finding.rule_id.as_str() == "AP-MANIFEST-LOCATION")
        );
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn non_utf8_collection_child_is_not_silently_skipped() {
    let temp = TempDir::new().unwrap();
    let valid = temp.path().join("valid");
    fs::create_dir(&valid).unwrap();
    write_manifest(&valid, json!({}));
    let invalid = temp.path().join(non_utf8_component());
    fs::create_dir(&invalid).unwrap();
    let report = lint_path(temp.path(), options(InputMode::Collection));
    assert_eq!(report.exit_code, 2);
    assert!(
        report
            .errors
            .iter()
            .any(|error| error.code == "PATH_ENCODING")
    );
}

#[cfg(target_os = "macos")]
#[test]
fn macos_filesystem_rejects_non_utf8_collection_child_creation() {
    let temp = TempDir::new().unwrap();
    let invalid = temp.path().join(non_utf8_component());
    let error =
        fs::create_dir(&invalid).expect_err("runner filesystem rejects invalid UTF-8 bytes");
    assert_eq!(error.raw_os_error(), Some(libc::EILSEQ));
}

#[cfg(unix)]
#[test]
fn manifest_permission_error_has_no_normative_failure() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let temp = TempDir::new().unwrap();
    let locked = temp.path().join("locked");
    fs::create_dir(&locked).unwrap();
    fs::write(locked.join("real"), b"{}").unwrap();
    symlink("locked/real", temp.path().join("plugin.json")).unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    let report = lint_path(temp.path(), options(InputMode::Plugin));
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(report.exit_code, 2);
    assert!(!report.complete);
    assert!(
        report
            .errors
            .iter()
            .any(|error| error.code == "MANIFEST_CANONICALIZE")
    );
    assert!(report.plugins[0].findings.is_empty());
    assert!(
        report.plugins[0]
            .coverage
            .iter()
            .all(|coverage| coverage.status != agent_plugin_lint::CoverageStatus::Fail)
    );
    let location = report.plugins[0]
        .coverage
        .iter()
        .find(|coverage| coverage.rule_id == "AP-MANIFEST-LOCATION")
        .unwrap();
    assert_eq!(
        location.status,
        agent_plugin_lint::CoverageStatus::Unchecked
    );
    assert_eq!(location.reason_code.as_deref(), Some("MANIFEST_IO"));
}

#[cfg(unix)]
#[test]
fn external_links_and_fifos_are_never_read_as_manifests() {
    use std::os::unix::fs::symlink;
    let temp = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    write_manifest(outside.path(), json!({}));
    symlink(
        outside.path().join("plugin.json"),
        temp.path().join("plugin.json"),
    )
    .unwrap();
    let escape = lint_path(temp.path(), options(InputMode::Plugin));
    assert_eq!(escape.exit_code, 1);
    assert!(
        escape.plugins[0]
            .findings
            .iter()
            .any(|finding| finding.rule_id.as_str() == "AP-PATH-MANIFEST-ESCAPE")
    );

    let fifo = TempDir::new().unwrap();
    assert!(
        Command::new("mkfifo")
            .arg(fifo.path().join("plugin.json"))
            .status()
            .unwrap()
            .success()
    );
    let fifo_root = fifo.path().to_path_buf();
    let report = complete_within(move || lint_path(&fifo_root, options(InputMode::Plugin)));
    assert_eq!(report.exit_code, 1);
    assert!(
        report.plugins[0]
            .findings
            .iter()
            .any(|finding| finding.rule_id.as_str() == "AP-MANIFEST-LOCATION")
    );
}

#[cfg(unix)]
#[test]
fn internal_manifest_link_and_root_alias_ambiguity_are_handled() {
    use std::os::unix::fs::symlink;
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join("data")).unwrap();
    write_manifest(&temp.path().join("data"), json!({}));
    symlink(
        temp.path().join("data").join("plugin.json"),
        temp.path().join("plugin.json"),
    )
    .unwrap();
    assert_eq!(
        lint_path(temp.path(), options(InputMode::Plugin)).exit_code,
        0
    );

    let collection = TempDir::new().unwrap();
    let first = collection.path().join("first");
    fs::create_dir(&first).unwrap();
    write_manifest(&first, json!({}));
    symlink(&first, collection.path().join("alias")).unwrap();
    let report = lint_path(collection.path(), options(InputMode::Collection));
    assert_eq!(report.exit_code, 2);
    assert!(
        report
            .errors
            .iter()
            .any(|error| error.code == "DISCOVERY_AMBIGUOUS_ROOT")
    );
}
