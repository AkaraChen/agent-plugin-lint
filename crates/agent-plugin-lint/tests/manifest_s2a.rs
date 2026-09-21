use agent_plugin_lint::{CoverageStatus, validate_manifest, vendor};
use serde_json::json;
use sha2::{Digest, Sha256};

const SCHEMA: &str = "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json";

fn manifest(extra: serde_json::Value) -> serde_json::Value {
    let mut value = json!({"$schema": SCHEMA, "name": "a"});
    for (key, item) in extra.as_object().expect("test object") {
        value[key] = item.clone();
    }
    value
}

fn coverage<'a>(
    report: &'a agent_plugin_lint::ManifestValidation,
    rule: &str,
) -> &'a agent_plugin_lint::Coverage {
    report
        .coverage
        .iter()
        .find(|item| item.rule_id == rule)
        .expect("coverage")
}

#[test]
fn vendored_schemas_match_research_identity() {
    assert_eq!(
        vendor::PLUGIN_SCHEMA,
        include_str!("../../../research/schemas/plugin.schema.json")
    );
    assert_eq!(
        vendor::MCP_SCHEMA,
        include_str!("../../../research/schemas/mcp.schema.json")
    );
    assert_eq!(
        format!("{:x}", Sha256::digest(vendor::PLUGIN_SCHEMA)),
        "0a4aad95ce337878ad38802ebf0daa3fde76abe3f65400c86bcbb1ec0b3ab883"
    );
    assert_eq!(
        format!("{:x}", Sha256::digest(vendor::MCP_SCHEMA)),
        "6539175bfcdf43085855183e86da40ea94b166547a72b47ae9a0a390516d3acb"
    );
    assert!(vendor::PLUGIN_SCHEMA.contains(&format!("\"$id\": \"{}\"", vendor::PLUGIN_SCHEMA_ID)));
    assert!(vendor::MCP_SCHEMA.contains(&format!("\"$id\": \"{}\"", vendor::MCP_SCHEMA_ID)));
    assert!(vendor::PLUGIN_SCHEMA.contains("\"required\": [\"$schema\", \"name\"]"));
}

#[test]
fn pointers_escape_complete_segments() {
    let report = validate_manifest(&manifest(json!({"a~/b": 3, "author": {"x/y": true}})));
    let findings = serde_json::to_value(&report.findings).unwrap();
    assert!(
        findings
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["pointer"] == "/a~0~1b")
    );
    assert!(
        findings
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["pointer"] == "/author/x~1y")
    );
}

#[test]
fn semver_is_a_should_and_does_not_reject() {
    let report = validate_manifest(&manifest(json!({"version": "nightly"})));
    let findings = serde_json::to_value(&report.findings).unwrap();
    assert!(
        findings
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["ruleId"] == "AP-VERSION-SEMVER"
                && item["obligation"] == "SHOULD"
                && item["radius"] == "advisory")
    );
    assert!(!report.rejected);
}

#[test]
fn obligation_serialization_uses_contract_tokens() {
    let values = [
        (agent_plugin_lint::Obligation::Must, "MUST"),
        (agent_plugin_lint::Obligation::Should, "SHOULD"),
        (agent_plugin_lint::Obligation::Recommended, "RECOMMENDED"),
        (agent_plugin_lint::Obligation::None, "NONE"),
    ];
    for (obligation, expected) in values {
        assert_eq!(serde_json::to_value(obligation).unwrap(), json!(expected));
    }
}

#[test]
fn unknown_extension_is_unchecked_without_a_finding() {
    let report = validate_manifest(&manifest(json!({"extensions": {"com.example": 3}})));
    assert!(report.findings.is_empty());
    let item = coverage(&report, "AP-EXTENSION-UNKNOWN");
    assert_eq!(item.status, CoverageStatus::Unchecked);
    assert_eq!(item.reason_code.as_deref(), Some("UNIMPLEMENTED_NAMESPACE"));
}

#[test]
fn ignored_violations_are_failed_checks_without_rejection() {
    let report = validate_manifest(&manifest(json!({"unknown": 3, "extensions": false})));
    for rule in ["AP-MANIFEST-UNKNOWN-FIELD", "AP-EXTENSIONS-OBJECT"] {
        assert_eq!(
            coverage(&report, rule).status,
            CoverageStatus::Fail,
            "{rule}"
        );
    }
    assert!(!report.rejected);
}

#[test]
fn invalid_root_blocks_every_follow_up_rule() {
    let report = validate_manifest(&json!([]));
    assert!(report.rejected);
    assert_eq!(
        coverage(&report, "AP-MANIFEST-JSON").status,
        CoverageStatus::Fail
    );
    assert!(
        report
            .coverage
            .iter()
            .filter(|item| item.rule_id != "AP-MANIFEST-JSON")
            .all(|item| item.status == CoverageStatus::Blocked)
    );
}

#[test]
fn author_failure_does_not_skip_independent_checks() {
    let report = validate_manifest(&manifest(json!({"author": 3, "extensions": false})));
    let findings = serde_json::to_value(&report.findings).unwrap();
    assert!(
        findings
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["ruleId"] == "AP-EXTENSIONS-OBJECT")
    );
}

#[test]
fn required_failure_suppresses_derived_schema_and_name_findings() {
    let report = validate_manifest(&manifest(json!({"$schema": "", "name": ""})));
    let findings = serde_json::to_value(&report.findings).unwrap();
    assert_eq!(
        findings
            .as_array()
            .unwrap()
            .iter()
            .filter(|item| item["pointer"] == "/$schema")
            .count(),
        1
    );
    assert_eq!(
        findings
            .as_array()
            .unwrap()
            .iter()
            .filter(|item| item["pointer"] == "/name")
            .count(),
        1
    );
    assert_eq!(
        coverage(&report, "AP-MANIFEST-SCHEMA-ID").status,
        CoverageStatus::Blocked
    );
    assert_eq!(
        coverage(&report, "AP-MANIFEST-REQUIRED").status,
        CoverageStatus::Fail
    );
    assert_eq!(report.declared_spec, None);
    assert_eq!(report.name, None);
}

#[test]
fn canonical_schema_maps_to_recognized_version_only() {
    assert_eq!(
        validate_manifest(&manifest(json!({})))
            .declared_spec
            .as_deref(),
        Some("1.0.0")
    );
    assert_eq!(
        validate_manifest(&manifest(json!({"$schema": "https://example.com/private"})))
            .declared_spec,
        None
    );
}

#[test]
fn name_constraints_and_metadata_format_boundaries() {
    for name in ["a", "a.1", "a.-b", "a-.b"] {
        assert!(
            !validate_manifest(&manifest(json!({"name": name}))).rejected,
            "{name}"
        );
    }
    for name in ["A", "a--b", "a..b", "-a", "a.", "a_b", ""] {
        assert!(
            validate_manifest(&manifest(json!({"name": name}))).rejected,
            "{name}"
        );
    }
    let report = validate_manifest(&manifest(
        json!({"version": "nightly", "homepage": "local", "repository": "x", "license": "x", "author": {"email": "x", "url": "x"}, "keywords": []}),
    ));
    assert!(!report.rejected);
}

#[test]
fn serialized_shape_is_stable_and_has_reason_codes() {
    let input = manifest(json!({"unknown": 3}));
    let first = serde_json::to_string(&validate_manifest(&input).findings).unwrap();
    let second = serde_json::to_string(&validate_manifest(&input).findings).unwrap();
    assert_eq!(first, second);
    let findings = serde_json::to_value(&validate_manifest(&input).findings).unwrap();
    assert_eq!(findings[0]["scope"], json!({"kind": "plugin", "id": null}));
    let coverage = serde_json::to_value(&validate_manifest(&input).coverage).unwrap();
    assert!(
        coverage
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item.get("reasonCode").is_some())
    );
}

#[test]
fn findings_and_coverage_have_lexical_ordering() {
    let report = validate_manifest(&manifest(json!({"name": "A".repeat(65)})));
    let ids: Vec<_> = report
        .findings
        .iter()
        .filter(|finding| finding.pointer.as_deref() == Some("/name"))
        .map(|finding| finding.rule_id.as_str())
        .collect();
    assert_eq!(ids, ["AP-NAME-CHARSET", "AP-NAME-LENGTH"]);
    assert!(report.coverage.windows(2).all(|items| {
        (
            items[0].rule_id.as_str(),
            items[0].target.as_str(),
            items[0].status.as_str(),
        ) <= (
            items[1].rule_id.as_str(),
            items[1].target.as_str(),
            items[1].status.as_str(),
        )
    }));
}
