use agent_plugin_lint::{CoverageStatus, InputMode, LintOptions, lint_path};
use std::collections::BTreeSet;
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
    assert!(report.plugins[0].coverage.iter().any(|c| {
        c.rule_id == "AP-MCP-ENVELOPE"
            && c.status == CoverageStatus::Unchecked
            && c.reason_code.as_deref() == Some("JSON_REPRESENTATION")
    }));
    assert!(report.plugins[0].coverage.iter().any(|c| {
        c.rule_id == "AP-ADVICE-DUPLICATE-JSON-KEY"
            && c.status == CoverageStatus::Unchecked
            && c.reason_code.as_deref() == Some("JSON_REPRESENTATION")
    }));
    assert!(
        report.plugins[0]
            .coverage
            .iter()
            .any(|c| c.rule_id == "AP-MCP-SERVER-VARIANT" && c.status == CoverageStatus::Blocked)
    );
}

#[test]
fn coverage_distinguishes_present_unimplemented_targets_from_absent_targets() {
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("plugin.json"),
        format!(r#"{{"$schema":"{SCHEMA}","name":"a","license":"MIT","extensions":{{"com.example":3}}}}"#),
    )
    .unwrap();
    fs::write(
        temp.path().join("mcp.json"),
        r#"{"$schema":"https://agent-plugins.org/schemas/1.0.0/mcp.schema.json","mcpServers":{"s":{"type":"sse","url":"https://example.com"}}}"#,
    )
    .unwrap();
    let report = lint_path(temp.path(), LintOptions::default());
    let coverage = &report.plugins[0].coverage;
    for id in [
        "AP-LICENSE-SPDX",
        "AP-MCP-BUNDLED-COMMAND",
        "AP-MCP-PATH-DEPENDENCE",
    ] {
        assert!(
            coverage
                .iter()
                .any(|c| c.rule_id == id && c.status == CoverageStatus::Manual)
        );
    }
    for id in ["AP-EXTENSION-VALUE", "AP-EXTENSION-FILE-LOCATION"] {
        assert!(
            coverage
                .iter()
                .any(|c| c.rule_id == id && c.status == CoverageStatus::Manual)
        );
    }
    let fatal = lint(&format!(
        r#"{{"$schema":"{SCHEMA}","name":"BAD","license":"MIT"}}"#
    ));
    assert!(
        fatal.plugins[0]
            .coverage
            .iter()
            .any(|c| c.rule_id == "AP-LICENSE-SPDX" && c.status == CoverageStatus::Blocked)
    );
}

#[test]
fn deep_valid_json_is_a_representation_error_with_an_accurate_message() {
    let nested = format!("{}0{}", "[".repeat(160), "]".repeat(160));
    let report = lint(&format!(
        r#"{{"$schema":"{SCHEMA}","name":"a","unknown":{nested}}}"#
    ));
    assert_eq!(report.exit_code, 2);
    assert!(report.errors.iter().any(|error| {
        error.code == "MANIFEST_JSON_REPRESENTATION" && error.message.contains("数值范围或嵌套深度")
    }));
    assert!(
        !report.plugins[0]
            .findings
            .iter()
            .any(|f| f.rule_id.as_str() == "AP-MANIFEST-JSON")
    );
}

#[test]
fn coverage_records_real_successes_and_parent_gates_without_duplicate_keys() {
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("plugin.json"),
        format!(r#"{{"$schema":"{SCHEMA}","name":"a"}}"#),
    )
    .unwrap();
    let skill = temp.path().join("skills/a");
    fs::create_dir_all(&skill).unwrap();
    fs::write(
        skill.join("SKILL.md"),
        "---\nname: a\ndescription: valid\n---\nbody\n",
    )
    .unwrap();
    fs::write(temp.path().join("runner"), "not executed").unwrap();
    fs::write(
        temp.path().join("mcp.json"),
        r#"{"$schema":"https://agent-plugins.org/schemas/1.0.0/mcp.schema.json","mcpServers":{"s":{"type":"stdio","command":"./runner","cwd":"${PLUGIN_DATA}/state"}}}"#,
    )
    .unwrap();
    let report = lint_path(temp.path(), LintOptions::default());
    let coverage = &report.plugins[0].coverage;
    for (id, target, expected) in [
        ("AP-MANIFEST-LOCATION", "plugin.json", CoverageStatus::Pass),
        (
            "AP-PATH-MANIFEST-ESCAPE",
            "plugin.json",
            CoverageStatus::Pass,
        ),
        ("AP-DISCOVERY-KIND", "mcp.json", CoverageStatus::Pass),
        ("AP-PATH-FIXED-ESCAPE", "mcp.json", CoverageStatus::Pass),
        (
            "AP-PATH-SKILL-ESCAPE",
            "skills/a/SKILL.md",
            CoverageStatus::Pass,
        ),
        (
            "AP-PATH-RELATIVE-FORM",
            "mcp.json#/mcpServers/s/command",
            CoverageStatus::Pass,
        ),
    ] {
        assert!(coverage.iter().any(|item| {
            item.rule_id == id && item.target == target && item.status == expected
        }));
    }
    let keys: BTreeSet<_> = coverage
        .iter()
        .map(|item| (&item.rule_id, &item.target))
        .collect();
    assert_eq!(keys.len(), coverage.len());

    fs::write(temp.path().join("mcp.json"), "{").unwrap();
    let blocked = lint_path(temp.path(), LintOptions::default());
    let coverage = &blocked.plugins[0].coverage;
    for id in [
        "AP-MCP-BUNDLED-COMMAND",
        "AP-MCP-PATH-DEPENDENCE",
        "AP-MCP-HEADER-SECRETS",
        "AP-MCP-ENV-SECRETS",
    ] {
        assert!(coverage.iter().any(|item| {
            item.rule_id == id
                && item.target == "mcp.json"
                && item.status == CoverageStatus::Blocked
        }));
    }
}

#[test]
fn registry_placeholders_are_honest_unchecked_but_do_not_make_strict_fail() {
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("plugin.json"),
        format!(r#"{{"$schema":"{SCHEMA}","name":"a"}}"#),
    )
    .unwrap();
    let minimal = lint_path(
        temp.path(),
        LintOptions {
            mode: InputMode::Plugin,
            strict: true,
        },
    );
    assert_eq!(minimal.exit_code, 0);
    assert!(minimal.plugins[0].coverage.iter().any(|item| {
        item.rule_id == "AP-PATH-RELATIVE-FORM"
            && item.target == "plugin"
            && item.status == CoverageStatus::Unchecked
            && item.reason_code.as_deref() == Some("RULE_NOT_EVALUATED")
    }));

    fs::write(
        temp.path().join("mcp.json"),
        r#"{"$schema":"https://agent-plugins.org/schemas/1.0.0/mcp.schema.json","mcpServers":{"s":{"type":"sse","url":"http://127.1"}}}"#,
    )
    .unwrap();
    let ambiguous = lint_path(
        temp.path(),
        LintOptions {
            mode: InputMode::Plugin,
            strict: true,
        },
    );
    assert_eq!(ambiguous.exit_code, 1);
    assert!(ambiguous.plugins[0].coverage.iter().any(|item| {
        item.rule_id == "AP-MCP-URL"
            && item.target == "mcp.json#/mcpServers/s/url"
            && item.status == CoverageStatus::Unchecked
            && item.reason_code.as_deref() == Some("AMBIGUOUS_URL_HOST")
    }));
}
