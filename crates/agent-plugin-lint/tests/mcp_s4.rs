use agent_plugin_lint::{CoverageStatus, LintOptions, lint_path};
use serde_json::json;
use std::fs;
use tempfile::TempDir;

const SCHEMA: &str = "https://agent-plugins.org/schemas/1.0.0/";

fn fixture(mcp: serde_json::Value) -> TempDir {
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("plugin.json"),
        json!({"$schema": format!("{SCHEMA}plugin.schema.json"), "name":"a"}).to_string(),
    )
    .unwrap();
    fs::write(temp.path().join("mcp.json"), mcp.to_string()).unwrap();
    temp
}
fn mcp(servers: serde_json::Value) -> serde_json::Value {
    json!({"$schema": format!("{SCHEMA}mcp.schema.json"), "mcpServers": servers})
}
fn report(temp: &TempDir) -> agent_plugin_lint::Report {
    lint_path(temp.path(), LintOptions::default())
}
fn has(report: &agent_plugin_lint::Report, id: &str) -> bool {
    report.plugins[0]
        .findings
        .iter()
        .any(|f| f.rule_id.as_str() == id)
}

#[test]
fn closed_envelope_and_entries_are_isolated_with_pointer_targets() {
    let temp = fixture(mcp(
        json!({"bad": 1, "good":{"type":"stdio","command":"node"}}),
    ));
    let r = report(&temp);
    assert!(has(&r, "AP-MCP-SERVER-VARIANT"));
    assert!(!has(&r, "AP-MCP-ENVELOPE"));
    assert!(
        r.plugins[0]
            .coverage
            .iter()
            .any(|c| c.rule_id == "AP-MCP-COMMAND"
                && c.target.contains("/mcpServers/good/command")
                && c.status == CoverageStatus::Pass)
    );
}

#[test]
fn schema_version_matching_is_not_a_prefix_suffix_heuristic() {
    let temp = fixture(
        json!({"$schema":"https://agent-plugins.org/schemas/1.1.0/mcp.schema.json","mcpServers":{}}),
    );
    assert!(has(&report(&temp), "AP-MCP-VERSION-MATCH"));
    let temp = fixture(
        json!({"$schema":"https://agent-plugins.org/schemas/garbage/path/mcp.schema.json","mcpServers":{}}),
    );
    assert!(has(&report(&temp), "AP-MCP-SCHEMA-ID"));
}

#[test]
fn invalid_variant_blocks_derived_static_checks() {
    let temp = fixture(mcp(json!({"bad":{"type":"stdio","command":3}})));
    let r = report(&temp);
    assert!(
        r.plugins[0]
            .coverage
            .iter()
            .any(|c| c.rule_id == "AP-MCP-COMMAND" && c.status == CoverageStatus::Blocked)
    );
    assert!(
        r.plugins[0]
            .coverage
            .iter()
            .any(|c| c.rule_id == "AP-MCP-URL" && c.status == CoverageStatus::Blocked)
    );
}

#[test]
fn stdio_command_forms_and_cwd_are_checked_from_plugin_root() {
    let temp = fixture(mcp(json!({
        "empty":{"type":"stdio","command":""},
        "bare_path":{"type":"stdio","command":"bin/server"},
        "inside":{"type":"stdio","command":"./bin/../server", "cwd":"./data"},
        "ambiguous":{"type":"stdio","command":"node --version"}
    })));
    fs::create_dir(temp.path().join("bin")).unwrap();
    fs::write(temp.path().join("server"), "x").unwrap();
    fs::create_dir(temp.path().join("data")).unwrap();
    let r = report(&temp);
    assert_eq!(r.exit_code, 1);
    assert!(has(&r, "AP-MCP-COMMAND"));
    assert!(has(&r, "AP-ADVICE-AMBIGUOUS-COMMAND"));
    assert!(
        r.plugins[0]
            .coverage
            .iter()
            .any(|c| c.target.ends_with("/inside/cwd") && c.status == CoverageStatus::Pass)
    );
}

#[test]
fn remote_raw_authority_boundaries_remain_unchecked_or_invalid() {
    let temp = fixture(mcp(json!({
        "percent":{"type":"sse","url":"http://%6cocalhost"},
        "hex":{"type":"sse","url":"http://0x7f000001"},
        "bad":{"type":"sse","url":"https:///example.com"},
        "ok":{"type":"sse","url":"HTTP://LOCALHOST", "headers":{"X-A":"literal"}}
    })));
    let r = report(&temp);
    assert!(has(&r, "AP-MCP-URL"));
    assert!(
        r.plugins[0]
            .coverage
            .iter()
            .any(|c| c.rule_id == "AP-MCP-URL" && c.status == CoverageStatus::Unchecked)
    );
    assert!(
        r.plugins[0]
            .coverage
            .iter()
            .any(|c| c.rule_id == "AP-MCP-HEADERS" && c.status == CoverageStatus::Pass)
    );
}

#[test]
fn expansion_is_single_pass_and_runtime_fields_stay_opaque() {
    let temp = fixture(mcp(
        json!({"s":{"type":"stdio","command":"node", "args":["../x", "${PLUGIN_ROOT}"], "env":{"X":"${PLUGIN_DATA}/../x"}, "cwd":"${PLUGIN_DATA}/future"}}),
    ));
    let r = report(&temp);
    assert_eq!(r.exit_code, 0);
    assert!(
        r.plugins[0]
            .coverage
            .iter()
            .any(|c| c.rule_id == "AP-PATH-SERVER-ESCAPE" && c.status == CoverageStatus::Runtime)
    );
}

#[cfg(unix)]
#[test]
fn relative_cwd_expands_once_before_being_joined_to_canonical_root() {
    let temp = fixture(mcp(
        json!({"s":{"type":"stdio","command":"node", "cwd":"./${PLUGIN_ROOT}"}}),
    ));
    let canonical_root = fs::canonicalize(temp.path()).unwrap();
    let stripped = canonical_root.strip_prefix("/").unwrap();
    fs::create_dir_all(canonical_root.join(stripped)).unwrap();
    let r = report(&temp);
    assert_eq!(r.exit_code, 0);
    assert!(
        r.plugins[0]
            .coverage
            .iter()
            .any(|c| c.rule_id == "AP-PATH-SERVER-ESCAPE" && c.status == CoverageStatus::Pass)
    );
}

#[cfg(windows)]
#[test]
fn relative_cwd_with_root_placeholder_is_path_io_on_windows() {
    let temp = fixture(mcp(
        json!({"s":{"type":"stdio","command":"node", "cwd":"./${PLUGIN_ROOT}"}}),
    ));
    let r = report(&temp);
    assert_eq!(r.exit_code, 2);
    assert!(r.errors.iter().any(|error| error.code == "MCP_PATH_IO"));
    assert!(
        r.plugins[0]
            .coverage
            .iter()
            .any(|c| c.rule_id == "AP-PATH-SERVER-ESCAPE" && c.status == CoverageStatus::Unchecked)
    );
    assert!(!has(&r, "AP-PATH-SERVER-ESCAPE"));
}

#[test]
fn replacement_text_is_not_expanded_again() {
    let parent = TempDir::new().unwrap();
    let root = parent.path().join("${PLUGIN_DATA}");
    fs::create_dir(&root).unwrap();
    fs::write(
        root.join("plugin.json"),
        json!({"$schema": format!("{SCHEMA}plugin.schema.json"), "name":"a"}).to_string(),
    )
    .unwrap();
    fs::write(
        root.join("mcp.json"),
        mcp(json!({"s":{"type":"stdio","command":"node", "cwd":"${PLUGIN_ROOT}"}})).to_string(),
    )
    .unwrap();
    let r = lint_path(&root, LintOptions::default());
    assert_eq!(
        r.exit_code, 0,
        "replacement text must not be expanded again"
    );
    assert!(
        r.plugins[0]
            .coverage
            .iter()
            .any(|c| c.rule_id == "AP-PATH-SERVER-ESCAPE" && c.status == CoverageStatus::Pass)
    );
}

#[test]
fn noncanonical_ip_spellings_and_mapped_ipv6_are_unchecked() {
    let temp = fixture(mcp(json!({
        "hex_octet":{"type":"sse","url":"http://127.0x0.0.1"},
        "mapped":{"type":"sse","url":"http://[0:0:0:0:0:ffff:7f00:1]"},
        "v4":{"type":"sse","url":"http://127.2.3.4"},
        "v6":{"type":"sse","url":"http://[::1]"}
    })));
    let r = report(&temp);
    assert_eq!(r.exit_code, 0);
    assert!(
        r.plugins[0]
            .coverage
            .iter()
            .filter(|c| c.rule_id == "AP-MCP-HTTPS")
            .any(|c| c.status == CoverageStatus::Unchecked)
    );
    assert!(
        r.plugins[0]
            .coverage
            .iter()
            .filter(|c| c.rule_id == "AP-MCP-HTTPS")
            .any(|c| c.status == CoverageStatus::Pass)
    );
}

#[cfg(unix)]
#[test]
fn raw_symlink_parent_cwd_uses_kernel_resolution() {
    use std::os::unix::fs::symlink;
    let temp = fixture(mcp(
        json!({"inside":{"type":"stdio","command":"node", "cwd":"./bridge/../safe"}, "outside":{"type":"stdio","command":"node", "cwd":"./bridge/../escape"}}),
    ));
    fs::create_dir_all(temp.path().join("inner/deep")).unwrap();
    fs::create_dir(temp.path().join("inner/safe")).unwrap();
    let outside = TempDir::new().unwrap();
    fs::create_dir(outside.path().join("escape")).unwrap();
    symlink(temp.path().join("inner/deep"), temp.path().join("bridge")).unwrap();
    let r = report(&temp);
    assert!(
        r.plugins[0]
            .coverage
            .iter()
            .any(|c| c.target.ends_with("/inside/cwd") && c.status == CoverageStatus::Pass)
    );
    fs::remove_file(temp.path().join("bridge")).unwrap();
    fs::create_dir(outside.path().join("deep")).unwrap();
    symlink(outside.path().join("deep"), temp.path().join("bridge")).unwrap();
    let r = report(&temp);
    assert!(has(&r, "AP-PATH-SERVER-ESCAPE"));
}

#[test]
fn empty_server_map_is_not_applicable_rather_than_unevaluated() {
    let temp = fixture(mcp(json!({})));
    let report = report(&temp);
    assert_eq!(report.exit_code, 0);
    for rule in [
        "AP-MCP-COMMAND",
        "AP-MCP-URL",
        "AP-MCP-HEADERS",
        "AP-PATH-RELATIVE-FORM",
    ] {
        assert!(
            report.plugins[0].coverage.iter().any(|coverage| {
                coverage.rule_id == rule
                    && coverage.target == "mcp.json"
                    && coverage.status == CoverageStatus::NotApplicable
                    && coverage.reason_code.as_deref() == Some("NO_SERVERS")
            }),
            "{rule} {:#?}",
            report.plugins[0].coverage
        );
    }
}

#[test]
fn rejected_stdio_command_blocks_the_checks_that_did_not_run() {
    let temp = fixture(mcp(json!({"s":{"type":"stdio","command":"../bin/server"}})));
    let report = report(&temp);
    assert_eq!(report.exit_code, 1);
    assert!(has(&report, "AP-MCP-COMMAND"));
    for rule in ["AP-MCP-CWD-FORM", "AP-MCP-RESERVED-ENV"] {
        assert!(
            report.plugins[0].coverage.iter().any(|coverage| {
                coverage.rule_id == rule
                    && coverage.target.contains("/mcpServers/s/")
                    && coverage.status == CoverageStatus::Blocked
            }),
            "{rule}"
        );
    }
    assert!(report.plugins[0].coverage.iter().any(|coverage| {
        coverage.rule_id == "AP-MCP-URL"
            && coverage.status == CoverageStatus::NotApplicable
            && coverage.reason_code.as_deref() == Some("STDIO_NO_REMOTE")
    }));
}

#[test]
fn reserved_env_blocks_cwd_instead_of_leaving_it_unevaluated() {
    let temp = fixture(mcp(
        json!({"s":{"type":"stdio","command":"node","env":{"PLUGIN_ROOT":"x"}}}),
    ));
    let report = report(&temp);
    assert_eq!(report.exit_code, 1);
    assert!(report.plugins[0].coverage.iter().any(|coverage| {
        coverage.rule_id == "AP-MCP-CWD-FORM"
            && coverage.target.ends_with("/cwd")
            && coverage.status == CoverageStatus::Blocked
            && coverage.reason_code.as_deref() == Some("RESERVED_ENV")
    }));
}

#[test]
fn unresolved_dot_command_records_command_coverage_without_a_must() {
    let temp = fixture(mcp(json!({"s":{"type":"stdio","command":"./missing"}})));
    let report = report(&temp);
    assert_eq!(report.exit_code, 0);
    assert!(!has(&report, "AP-MCP-COMMAND"));
    assert!(report.plugins[0].coverage.iter().any(|coverage| {
        coverage.rule_id == "AP-MCP-COMMAND"
            && coverage.target.ends_with("/mcpServers/s/command")
            && coverage.status == CoverageStatus::Unchecked
            && coverage.reason_code.as_deref() == Some("PATH_UNRESOLVED")
    }));
    assert!(report.plugins[0].coverage.iter().any(|coverage| {
        coverage.rule_id == "AP-PATH-RELATIVE-FORM"
            && coverage.target.ends_with("/command")
            && coverage.status == CoverageStatus::Pass
    }));
    assert!(report.plugins[0].coverage.iter().any(|coverage| {
        coverage.rule_id == "AP-PATH-SERVER-ESCAPE"
            && coverage.target.ends_with("/command")
            && coverage.status == CoverageStatus::Unchecked
            && coverage.reason_code.as_deref() == Some("PATH_UNRESOLVED")
    }));
    assert!(report.plugins[0].coverage.iter().all(|coverage| {
        !(coverage.rule_id == "AP-MCP-COMMAND"
            && coverage.reason_code.as_deref() == Some("RULE_NOT_EVALUATED"))
    }));
}

#[test]
fn invalid_cwd_form_blocks_server_escape() {
    let temp = fixture(mcp(
        json!({"s":{"type":"stdio","command":"node","cwd":"../x"}}),
    ));
    let report = report(&temp);
    assert_eq!(report.exit_code, 1);
    assert!(has(&report, "AP-MCP-CWD-FORM"));
    assert!(report.plugins[0].coverage.iter().any(|coverage| {
        coverage.rule_id == "AP-PATH-SERVER-ESCAPE"
            && coverage.target.ends_with("/mcpServers/s/cwd")
            && coverage.status == CoverageStatus::Blocked
            && coverage.reason_code.as_deref() == Some("CWD_FORM")
    }));
    assert!(report.plugins[0].coverage.iter().all(|coverage| {
        !(coverage.rule_id == "AP-PATH-SERVER-ESCAPE"
            && coverage.reason_code.as_deref() == Some("RULE_NOT_EVALUATED"))
    }));
}

#[test]
fn unresolved_cwd_records_cwd_form() {
    let temp = fixture(mcp(
        json!({"s":{"type":"stdio","command":"node","cwd":"./missing"}}),
    ));
    let report = report(&temp);
    assert_eq!(report.exit_code, 0);
    assert!(!has(&report, "AP-MCP-CWD-FORM"));
    assert!(report.plugins[0].coverage.iter().any(|coverage| {
        coverage.rule_id == "AP-MCP-CWD-FORM"
            && coverage.target.ends_with("/mcpServers/s/cwd")
            && coverage.status == CoverageStatus::Unchecked
            && coverage.reason_code.as_deref() == Some("PATH_UNRESOLVED")
    }));
    assert!(report.plugins[0].coverage.iter().any(|coverage| {
        coverage.rule_id == "AP-PATH-SERVER-ESCAPE"
            && coverage.target.ends_with("/cwd")
            && coverage.status == CoverageStatus::Unchecked
            && coverage.reason_code.as_deref() == Some("PATH_UNRESOLVED")
    }));
}

#[cfg(unix)]
#[test]
fn escaped_cwd_symlink_records_cwd_form_as_blocked() {
    use std::os::unix::fs::symlink;
    let temp = fixture(mcp(
        json!({"s":{"type":"stdio","command":"node","cwd":"./out"}}),
    ));
    let outside = TempDir::new().unwrap();
    symlink(outside.path(), temp.path().join("out")).unwrap();
    let report = report(&temp);
    assert_eq!(report.exit_code, 1);
    assert!(has(&report, "AP-PATH-SERVER-ESCAPE"));
    assert!(report.plugins[0].coverage.iter().any(|coverage| {
        coverage.rule_id == "AP-MCP-CWD-FORM"
            && coverage.target.ends_with("/mcpServers/s/cwd")
            && coverage.status == CoverageStatus::Blocked
            && coverage.reason_code.as_deref() == Some("SERVER_OUTSIDE_ROOT")
    }));
}

#[test]
fn https_failure_records_the_url_form_and_blocks_headers() {
    let temp = fixture(mcp(json!({
        "s": {
            "type": "sse",
            "url": "http://api.example/x",
            "headers": {"X-Token": "secret"}
        }
    })));
    let report = report(&temp);
    assert_eq!(report.exit_code, 1);
    assert!(has(&report, "AP-MCP-HTTPS"));
    assert!(!has(&report, "AP-ADVICE-POSSIBLE-SECRET"));
    assert!(report.plugins[0].coverage.iter().any(|coverage| {
        coverage.rule_id == "AP-MCP-URL"
            && coverage.target.ends_with("/url")
            && coverage.status == CoverageStatus::Pass
            && coverage.reason_code.as_deref() == Some("URL_VALID")
    }));
    assert!(report.plugins[0].coverage.iter().any(|coverage| {
        coverage.rule_id == "AP-MCP-HEADERS"
            && coverage.status == CoverageStatus::Blocked
            && coverage.reason_code.as_deref() == Some("MCP_HTTPS")
    }));
    assert!(report.plugins[0].coverage.iter().any(|coverage| {
        coverage.rule_id == "AP-MCP-COMMAND"
            && coverage.status == CoverageStatus::NotApplicable
            && coverage.reason_code.as_deref() == Some("REMOTE_NO_STDIO")
    }));
}
