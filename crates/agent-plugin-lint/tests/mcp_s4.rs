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
    assert_eq!(
        agent_plugin_lint::expand_once_for_test(
            "${PLUGIN_ROOT}${PLUGIN_ROOT}${UNKNOWN}",
            "${PLUGIN_DATA}",
            "D"
        ),
        "${PLUGIN_DATA}${PLUGIN_DATA}${UNKNOWN}"
    );
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

#[test]
fn relative_cwd_expands_once_before_being_joined_to_root() {
    let temp = fixture(mcp(
        json!({"s":{"type":"stdio","command":"node", "cwd":"./${PLUGIN_ROOT}"}}),
    ));
    let stripped = temp.path().strip_prefix("/").unwrap();
    fs::create_dir_all(temp.path().join(stripped)).unwrap();
    let r = report(&temp);
    assert_eq!(r.exit_code, 0);
    assert!(
        r.plugins[0]
            .coverage
            .iter()
            .any(|c| c.rule_id == "AP-PATH-SERVER-ESCAPE" && c.status == CoverageStatus::Pass)
    );
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
