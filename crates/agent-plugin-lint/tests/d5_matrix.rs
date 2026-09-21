use agent_plugin_lint::{
    CoverageStatus, InputMode, LintOptions, Obligation, Radius, Report, lint_path,
};
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

const PLUGIN_SCHEMA: &str = "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Case {
    name: String,
    source: String,
    spec: String,
    decision: String,
    mcp: Value,
    #[serde(default)]
    create: Vec<String>,
    default_exit: i32,
    strict_exit: i32,
    rule: String,
    target: String,
    status: String,
    reason: String,
    finding: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Matrix {
    source: String,
    spec: String,
    decision: String,
    cases: Vec<Case>,
}

fn cases() -> Matrix {
    serde_json::from_str(include_str!("fixtures/d5_matrix.json")).unwrap()
}

fn write_case(case: &Case) -> TempDir {
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("plugin.json"),
        format!(r#"{{"$schema":"{PLUGIN_SCHEMA}","name":"d5-case"}}"#),
    )
    .unwrap();
    fs::write(temp.path().join("mcp.json"), case.mcp.to_string()).unwrap();
    for entry in &case.create {
        let path = temp.path().join(entry);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "fixture executable placeholder").unwrap();
    }
    temp
}

fn command(case: &Case) -> Option<&str> {
    case.mcp["mcpServers"]["case"]["command"].as_str()
}

fn assert_fixture_characters(case: &Case) {
    match case.name.as_str() {
        "windows-drive-token" => assert_eq!(command(case), Some("C:\\Tools\\node.exe")),
        "windows-backslash-token" => assert_eq!(command(case), Some("bin\\server")),
        "unc-token" => assert_eq!(command(case), Some("\\\\host\\share\\node.exe")),
        "control-token" => assert_eq!(command(case), Some("node\u{1}")),
        "nul-token" => assert_eq!(command(case), Some("node\0")),
        _ => {}
    }
}

fn status(value: &str) -> CoverageStatus {
    match value {
        "pass" => CoverageStatus::Pass,
        "fail" => CoverageStatus::Fail,
        "unchecked" => CoverageStatus::Unchecked,
        _ => panic!("fixture has unsupported coverage status: {value}"),
    }
}

fn assert_report(case: &Case, report: &Report, strict: bool) {
    assert_eq!(
        report.exit_code,
        if strict {
            case.strict_exit
        } else {
            case.default_exit
        },
        "{}",
        case.name
    );
    let plugin = &report.plugins[0];
    assert!(
        plugin.coverage.iter().any(|coverage| {
            coverage.rule_id == case.rule
                && coverage.target == case.target
                && coverage.status == status(&case.status)
                && coverage.reason_code.as_deref() == Some(case.reason.as_str())
        }),
        "{}: missing fixed coverage expectation",
        case.name
    );
    if let Some(finding) = &case.finding {
        let item = plugin
            .findings
            .iter()
            .find(|item| item.rule_id.as_str() == finding)
            .unwrap();
        match finding.as_str() {
            "AP-ADVICE-AMBIGUOUS-COMMAND" | "AP-ADVICE-ENV-CASE" => {
                assert_eq!(item.radius, Radius::Advisory);
                assert!(!item.normative);
                assert_eq!(item.obligation, Obligation::None);
            }
            "AP-MCP-COMMAND" | "AP-MCP-HTTPS" | "AP-MCP-RESERVED-ENV" => {
                assert_eq!(item.radius, Radius::Component);
                assert!(item.normative);
                assert_eq!(item.obligation, Obligation::Must);
            }
            _ => unreachable!("unexpected fixed D5 finding"),
        }
    }
    if case.default_exit == 0 {
        assert!(
            plugin.findings.iter().all(|item| {
                !(item.normative && item.obligation == agent_plugin_lint::Obligation::Must)
            }),
            "{}: a D5 unchecked/advisory input became a normative MUST finding",
            case.name
        );
    }
    if case.status == "unchecked" {
        assert!(
            !plugin.coverage.iter().any(|coverage| {
                coverage.rule_id == case.rule
                    && coverage.target == case.target
                    && coverage.status == CoverageStatus::Pass
            }),
            "{}: uncertain target also passed",
            case.name
        );
    }
    if case.default_exit == 0 && case.strict_exit == 0 {
        assert!(
            plugin.findings.is_empty(),
            "{}: positive control emitted a finding",
            case.name
        );
    }
}

fn assert_cli(case: &Case, path: &Path, strict: bool) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ap-lint"));
    command.arg(path).arg("--json");
    if strict {
        command.arg("--strict");
    }
    let output = command.output().unwrap();
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let expected_exit = if strict {
        case.strict_exit
    } else {
        case.default_exit
    };
    assert_eq!(output.status.code(), Some(expected_exit), "{}", case.name);
    assert_eq!(report["exitCode"], expected_exit, "{}", case.name);
    assert!(
        report["plugins"][0]["coverage"]
            .as_array()
            .unwrap()
            .iter()
            .any(|coverage| {
                coverage["ruleId"] == case.rule
                    && coverage["target"] == case.target
                    && coverage["status"] == case.status
                    && coverage["reasonCode"] == case.reason
            }),
        "{}: CLI JSON lacks fixed coverage expectation",
        case.name
    );
    if let Some(finding) = &case.finding {
        let item = report["plugins"][0]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["ruleId"] == *finding)
            .unwrap();
        match finding.as_str() {
            "AP-ADVICE-AMBIGUOUS-COMMAND" | "AP-ADVICE-ENV-CASE" => {
                assert_eq!(item["radius"], "advisory");
                assert_eq!(item["normative"], false);
                assert_eq!(item["obligation"], "NONE");
            }
            "AP-MCP-COMMAND" | "AP-MCP-HTTPS" | "AP-MCP-RESERVED-ENV" => {
                assert_eq!(item["radius"], "component");
                assert_eq!(item["normative"], true);
                assert_eq!(item["obligation"], "MUST");
            }
            _ => unreachable!("unexpected fixed D5 finding"),
        }
    }
    if case.default_exit == 0 {
        assert!(
            report["plugins"][0]["findings"]
                .as_array()
                .unwrap()
                .iter()
                .all(|item| { !(item["normative"] == true && item["obligation"] == "MUST") }),
            "{}: CLI JSON contains a normative MUST finding",
            case.name
        );
    }
    if case.default_exit == 0 && case.strict_exit == 0 {
        assert!(
            report["plugins"][0]["findings"]
                .as_array()
                .unwrap()
                .is_empty(),
            "{}: CLI positive control emitted a finding",
            case.name
        );
    }
    if case.status == "unchecked" {
        assert!(
            !report["plugins"][0]["coverage"]
                .as_array()
                .unwrap()
                .iter()
                .any(|coverage| {
                    coverage["ruleId"] == case.rule
                        && coverage["target"] == case.target
                        && coverage["status"] == "pass"
                }),
            "{}: CLI uncertain target also passed",
            case.name
        );
    }
}

fn assert_policy_case(case: &Value) {
    assert_eq!(case["source"], "research/1.0.0.md");
    assert!(!case["spec"].as_str().unwrap().is_empty());
    assert!(case["decision"].as_str().unwrap().contains('D'));
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("plugin.json"),
        case["manifest"].to_string(),
    )
    .unwrap();
    for strict in [false, true] {
        let expected_exit = case[if strict { "strictExit" } else { "defaultExit" }]
            .as_i64()
            .unwrap() as i32;
        let report = lint_path(
            temp.path(),
            LintOptions {
                mode: InputMode::Auto,
                strict,
            },
        );
        assert_eq!(report.exit_code, expected_exit);
        assert!(
            report.plugins[0].coverage.iter().any(|coverage| {
                coverage.rule_id == case["rule"].as_str().unwrap()
                    && coverage.target == case["target"].as_str().unwrap()
                    && coverage.status.as_str() == case["status"].as_str().unwrap()
                    && coverage.reason_code.as_deref() == case["reason"].as_str()
            }),
            "missing policy coverage: {case:?}"
        );
        if let Some(rule) = case.get("finding").and_then(Value::as_str) {
            let finding = report.plugins[0]
                .findings
                .iter()
                .find(|finding| finding.rule_id.as_str() == rule)
                .unwrap();
            let json = serde_json::to_value(finding).unwrap();
            assert_eq!(json["radius"], case["radius"]);
            assert_eq!(json["normative"], case["normative"]);
            assert_eq!(json["obligation"], case["obligation"]);
        }
        if case["rule"] == "AP-EXTENSION-UNKNOWN" {
            assert!(report.plugins[0].findings.is_empty());
            assert!(report.plugins[0].coverage.iter().any(|coverage| {
                coverage.rule_id == "AP-EXTENSION-VALUE"
                    && coverage.target == "plugin.json#/extensions"
                    && coverage.status == CoverageStatus::Manual
                    && coverage.reason_code.as_deref() == Some("UNIMPLEMENTED_NAMESPACE")
            }));
            assert!(report.plugins[0].coverage.iter().any(|coverage| {
                coverage.rule_id == "AP-EXTENSION-NAMESPACE"
                    && coverage.status == CoverageStatus::Unchecked
                    && coverage.reason_code.as_deref() == Some("NAMESPACE_SYNTAX_UNSPECIFIED")
            }));
        }
        assert_cli_policy(temp.path(), strict, expected_exit, case);
    }
}

fn assert_cli_policy(path: &Path, strict: bool, expected_exit: i32, case: &Value) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ap-lint"));
    command.arg(path).arg("--json");
    if strict {
        command.arg("--strict");
    }
    let output = command.output().unwrap();
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(expected_exit));
    assert!(
        json["plugins"][0]["coverage"]
            .as_array()
            .unwrap()
            .iter()
            .any(|coverage| {
                coverage["ruleId"] == case["rule"].as_str().unwrap()
                    && coverage["target"] == case["target"].as_str().unwrap()
                    && coverage["status"] == case["status"].as_str().unwrap()
                    && coverage["reasonCode"] == case["reason"].as_str().unwrap()
            })
    );
    if let Some(rule) = case.get("finding").and_then(Value::as_str) {
        let finding = json["plugins"][0]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|finding| finding["ruleId"] == rule)
            .unwrap();
        assert_eq!(finding["radius"], case["radius"]);
        assert_eq!(finding["normative"], case["normative"]);
        assert_eq!(finding["obligation"], case["obligation"]);
    }
    if case["rule"] == "AP-EXTENSION-UNKNOWN" {
        assert!(
            json["plugins"][0]["findings"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(
            json["plugins"][0]["coverage"]
                .as_array()
                .unwrap()
                .iter()
                .any(|coverage| {
                    coverage["ruleId"] == "AP-EXTENSION-VALUE"
                        && coverage["target"] == "plugin.json#/extensions"
                        && coverage["status"] == "manual"
                        && coverage["reasonCode"] == "UNIMPLEMENTED_NAMESPACE"
                })
        );
        assert!(
            json["plugins"][0]["coverage"]
                .as_array()
                .unwrap()
                .iter()
                .any(|coverage| {
                    coverage["ruleId"] == "AP-EXTENSION-NAMESPACE"
                        && coverage["status"] == "unchecked"
                        && coverage["reasonCode"] == "NAMESPACE_SYNTAX_UNSPECIFIED"
                })
        );
    }
}

#[test]
fn d5_fixed_matrix_exercises_library_and_cli_on_every_platform() {
    let matrix = cases();
    assert!(matrix.source.contains("§7.2.1"));
    assert!(matrix.spec.contains("§9.2"));
    assert_eq!(matrix.decision, "D5");
    for case in matrix.cases {
        assert_eq!(case.source, "research/1.0.0.md");
        assert_eq!(case.decision, "D5");
        assert!(!case.spec.is_empty());
        if case.name.starts_with("env-") {
            assert!(case.spec.contains("§9.1") && case.spec.contains("§9.2"));
            if matches!(
                case.name.as_str(),
                "env-opaque-value" | "env-unicode-value-opaque"
            ) {
                assert!(case.spec.contains("§4.1"));
            }
        } else if case.name.starts_with("http-") || case.name.starts_with("https-") {
            assert_eq!(case.spec, "§7.2.1 L351");
        } else {
            assert_eq!(case.spec, "§7.2.1 L325");
        }
        assert_fixture_characters(&case);
        let temp = write_case(&case);
        for strict in [false, true] {
            let library = lint_path(
                temp.path(),
                LintOptions {
                    mode: InputMode::Auto,
                    strict,
                },
            );
            assert_report(&case, &library, strict);

            assert_cli(&case, temp.path(), strict);
        }
    }
}

#[test]
fn d3_d4_fixed_policy_matrix_keeps_unknown_data_conservative_and_errors_preferred() {
    let policy: Value = serde_json::from_str(include_str!("fixtures/d5_policy.json")).unwrap();
    assert!(policy["source"].as_str().unwrap().contains("§8.1"));
    assert!(policy["spec"].as_str().unwrap().contains("§5.2"));
    assert_eq!(policy["decision"], "D3, D4, D5");
    for name in [
        "unknownExtension",
        "illegalNamespace",
        "unknownTop",
        "nonObjectExtensions",
        "semverAdvice",
    ] {
        assert_policy_case(&policy[name]);
    }
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("plugin.json"),
        policy["unknownExtension"]["manifest"].to_string(),
    )
    .unwrap();
    let strict = lint_path(
        temp.path(),
        LintOptions {
            mode: InputMode::Auto,
            strict: true,
        },
    );
    assert!(strict.plugins[0].findings.is_empty());
    assert!(
        !strict.plugins[0]
            .findings
            .iter()
            .any(|finding| finding.rule_id.as_str() == "AP-EXTENSION-VALUE")
    );
    assert!(strict.plugins[0].coverage.iter().any(|coverage| {
        coverage.rule_id == "AP-EXTENSION-NAMESPACE"
            && coverage.status == CoverageStatus::Unchecked
            && coverage.reason_code.as_deref() == Some("NAMESPACE_SYNTAX_UNSPECIFIED")
    }));

    let collection = TempDir::new().unwrap();
    for (name, manifest) in [
        ("ignored", policy["unknownTop"]["manifest"].clone()),
        (
            "representation",
            Value::String(
                policy["representation"]["sourceText"]
                    .as_str()
                    .unwrap()
                    .into(),
            ),
        ),
    ] {
        let root = collection.path().join(name);
        fs::create_dir(&root).unwrap();
        let source = manifest
            .as_str()
            .map_or_else(|| manifest.to_string(), str::to_owned);
        fs::write(root.join("plugin.json"), source).unwrap();
    }
    let report = lint_path(
        collection.path(),
        LintOptions {
            mode: InputMode::Collection,
            strict: false,
        },
    );
    assert_eq!(report.exit_code, 2);
    assert!(!report.complete);
    assert!(
        report
            .errors
            .iter()
            .any(|error| error.code == "MANIFEST_JSON_REPRESENTATION")
    );
    assert!(
        report
            .plugins
            .iter()
            .any(|plugin| plugin.findings.iter().any(|finding| {
                finding.rule_id.as_str() == "AP-MANIFEST-UNKNOWN-FIELD"
                    && finding.radius == Radius::Ignored
                    && finding.normative
                    && finding.obligation == Obligation::Must
            }))
    );
    let mut command = Command::new(env!("CARGO_BIN_EXE_ap-lint"));
    let output = command
        .arg(collection.path())
        .args(["--mode", "collection", "--json"])
        .output()
        .unwrap();
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(json["exitCode"], 2);
    assert_eq!(json["complete"], false);
    assert!(
        json["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|error| error["code"] == "MANIFEST_JSON_REPRESENTATION")
    );
    assert!(json["plugins"].as_array().unwrap().iter().any(|plugin| {
        plugin["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| {
                finding["ruleId"] == "AP-MANIFEST-UNKNOWN-FIELD"
                    && finding["radius"] == "ignored"
                    && finding["normative"] == true
                    && finding["obligation"] == "MUST"
            })
    }));
}
