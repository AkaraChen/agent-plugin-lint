use agent_plugin_lint::{CoverageStatus, InputMode, LintOptions, lint_path};
use std::fs;
use std::process::Command;
use tempfile::TempDir;

const SCHEMA: &str = "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json";

fn lint(source: &str) -> agent_plugin_lint::Report {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("plugin.json"), source).unwrap();
    lint_path(
        temp.path(),
        LintOptions {
            mode: InputMode::Plugin,
            strict: false,
        },
    )
}

#[test]
fn extension_namespaces_are_conservative_and_gate_components_only_when_certainly_invalid() {
    let report = lint(&format!(
        r#"{{"$schema":"{SCHEMA}","name":"a","extensions":{{"com.example":null,"example":[],"com..x":{{}}}}}}"#
    ));
    assert!(report.plugins[0].findings.is_empty());
    assert_eq!(report.plugins[0].status, "accepted");
    assert!(
        report.plugins[0]
            .coverage
            .iter()
            .any(|c| c.rule_id == "AP-EXTENSION-UNKNOWN" && c.status == CoverageStatus::Manual)
    );
    let report = lint(&format!(
        r#"{{"$schema":"{SCHEMA}","name":"a","extensions":{{"":{{}}}}}}"#
    ));
    assert_eq!(report.plugins[0].status, "rejected");
    assert!(
        report.plugins[0]
            .findings
            .iter()
            .any(|f| f.rule_id.as_str() == "AP-EXTENSION-NAMESPACE")
    );
    assert!(
        report.plugins[0]
            .coverage
            .iter()
            .any(|c| c.rule_id == "AP-MCP-ENVELOPE" && c.status == CoverageStatus::Blocked)
    );
}

#[test]
fn duplicate_keys_are_advisory_and_value_validation_uses_the_last_member() {
    let report = lint(&format!(
        r#"{{"$schema":"bad","$schema":"{SCHEMA}","name":"a"}}"#
    ));
    assert_eq!(report.exit_code, 0);
    assert!(
        report.plugins[0]
            .findings
            .iter()
            .any(|f| f.rule_id.as_str() == "AP-ADVICE-DUPLICATE-JSON-KEY")
    );
    assert!(
        report.plugins[0].coverage.iter().any(
            |c| c.rule_id == "AP-ADVICE-DUPLICATE-JSON-KEY" && c.status == CoverageStatus::Fail
        )
    );
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("plugin.json"),
        format!(r#"{{"$schema":"{SCHEMA}","name":"a","na\u006de":"a"}}"#),
    )
    .unwrap();
    let strict = lint_path(
        temp.path(),
        LintOptions {
            mode: InputMode::Plugin,
            strict: true,
        },
    );
    assert_eq!(strict.exit_code, 1);
}

#[test]
fn syntactically_valid_unrepresentable_number_is_tool_error_not_manifest_json_failure() {
    let report = lint(&format!(
        r#"{{"$schema":"{SCHEMA}","name":"a","unknown":1e400}}"#
    ));
    assert_eq!(report.exit_code, 2);
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.code == "MANIFEST_JSON_REPRESENTATION")
    );
    assert!(
        !report.plugins[0]
            .findings
            .iter()
            .any(|f| f.rule_id.as_str() == "AP-MANIFEST-JSON")
    );
    assert!(
        report.plugins[0]
            .coverage
            .iter()
            .any(|c| c.rule_id == "AP-MANIFEST-JSON" && c.status == CoverageStatus::Unchecked)
    );
}

#[test]
fn mode_missing_value_with_json_is_structured_and_does_not_lint() {
    let output = Command::new(env!("CARGO_BIN_EXE_ap-lint"))
        .args(["--json", "--mode"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["errors"][0]["code"], "ARGUMENT");
}

#[test]
fn mcp_duplicate_keys_and_unrepresentable_number_are_not_misreported_as_invalid_json() {
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("plugin.json"),
        format!(r#"{{"$schema":"{SCHEMA}","name":"a"}}"#),
    )
    .unwrap();
    fs::write(temp.path().join("mcp.json"), r#"{"$schema":"https://agent-plugins.org/schemas/1.0.0/mcp.schema.json","mcpServers":{"a":{"type":"sse","url":"https://one","url":"https://two"}}}"#).unwrap();
    let report = lint_path(temp.path(), LintOptions::default());
    assert!(
        report.plugins[0]
            .findings
            .iter()
            .any(|f| f.rule_id.as_str() == "AP-ADVICE-DUPLICATE-JSON-KEY")
    );
    assert!(
        !report.plugins[0]
            .findings
            .iter()
            .any(|f| f.rule_id.as_str() == "AP-MCP-ENVELOPE")
    );
    fs::write(temp.path().join("mcp.json"), r#"{"unknown":1e400}"#).unwrap();
    let report = lint_path(temp.path(), LintOptions::default());
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.code == "MCP_JSON_REPRESENTATION")
    );
    assert!(
        report.plugins[0]
            .coverage
            .iter()
            .any(|c| c.rule_id == "AP-MCP-ENVELOPE" && c.status == CoverageStatus::Unchecked)
    );
    assert!(
        report.plugins[0]
            .coverage
            .iter()
            .any(|c| c.rule_id == "AP-MCP-SERVER-VARIANT" && c.status == CoverageStatus::Blocked)
    );
}
