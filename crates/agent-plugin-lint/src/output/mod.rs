mod json;
mod text;

use crate::Report;

/// One stdout transform for a finished [`Report`](crate::Report).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Text,
    Json,
}

impl Format {
    pub fn render(self, report: &Report) -> String {
        match self {
            Self::Text => text::render(report),
            Self::Json => json::render(report),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Format;
    use crate::{
        Confidence, Coverage, CoverageStatus, Effect, Finding, InputMode, Obligation, PluginReport,
        Policy, Radius, Report, RuleId, Scope, Subject, Summary, ToolError,
    };

    fn report() -> Report {
        Report {
            schema_version: 1,
            tool_version: "0.1.0".into(),
            ruleset_version: "1.0.0".into(),
            spec_version: "1.0.0".into(),
            input: "plugin".into(),
            mode: InputMode::Plugin,
            policy: Policy { strict: false },
            complete: true,
            plugins: vec![PluginReport {
                root: ".".into(),
                name: Some("a".into()),
                declared_spec: None,
                status: "accepted".into(),
                findings: vec![
                    Finding {
                        rule_id: RuleId::NameCharset,
                        spec: vec!["§5.5".into()],
                        radius: Radius::Fatal,
                        normative: true,
                        obligation: Obligation::Must,
                        subject: Subject::Package,
                        confidence: Confidence::Certain,
                        path: "plugin.json".into(),
                        pointer: Some("/name".into()),
                        line: None,
                        column: None,
                        scope: Scope::Plugin,
                        effect: Effect::RejectPlugin,
                        evidence_code: "NAME_CHARSET".into(),
                        message: "plugin name contains characters that are not allowed".into(),
                        hint: None,
                    },
                    Finding {
                        rule_id: RuleId::AsName,
                        spec: vec!["§7.1".into(), "§7.2".into()],
                        radius: Radius::Component,
                        normative: true,
                        obligation: Obligation::Must,
                        subject: Subject::Package,
                        confidence: Confidence::Certain,
                        path: "skills/bad/SKILL.md".into(),
                        pointer: None,
                        line: Some(2),
                        column: Some(1),
                        scope: Scope::Skill("bad".into()),
                        effect: Effect::SkipSkill,
                        evidence_code: "InvalidCharacters".into(),
                        message: "skill does not match the Agent Skills specification; this skill was skipped".into(),
                        hint: Some("name".into()),
                    },
                ],
                coverage: vec![
                    Coverage {
                        rule_id: "AP-NAME-LENGTH".into(),
                        target: "plugin.json".into(),
                        status: CoverageStatus::Pass,
                        reason_code: None,
                    },
                    Coverage {
                        rule_id: "AP-MCP-URL".into(),
                        target: "mcp.json".into(),
                        status: CoverageStatus::NotApplicable,
                        reason_code: Some("NO_URL".into()),
                    },
                    Coverage {
                        rule_id: "AP-MCP-ENVELOPE".into(),
                        target: "mcp.json".into(),
                        status: CoverageStatus::Blocked,
                        reason_code: Some("MANIFEST_REJECTED".into()),
                    },
                ],
            }],
            errors: vec![ToolError {
                path: String::new(),
                code: "ARGUMENT".into(),
                message: "missing input path".into(),
            }],
            summary: Summary {
                fatal: 1,
                component: 1,
                ignored: 0,
                advisory: 0,
                errors: 1,
            },
            exit_code: 1,
        }
    }

    #[test]
    fn text_lists_each_record_and_leaves_out_empty_fields() {
        let text = Format::Text.render(&report());
        assert_eq!(
            text,
            "\
error
  code: ARGUMENT
  path: 
  message: missing input path

plugin
  root: .
  status: accepted

finding
  rule: AP-NAME-CHARSET
  path: plugin.json
  spec: §5.5
  radius: fatal
  effect: reject-plugin
  scope: plugin
  evidence: NAME_CHARSET
  message: plugin name contains characters that are not allowed
  pointer: /name

finding
  rule: AS-NAME
  path: skills/bad/SKILL.md
  spec: §7.1, §7.2
  radius: component
  effect: skip-skill
  scope: skill bad
  evidence: InvalidCharacters
  message: skill does not match the Agent Skills specification; this skill was skipped
  hint: name
  line: 2
  column: 1

coverage
  rule: AP-MCP-ENVELOPE
  target: mcp.json
  status: blocked
  reason: MANIFEST_REJECTED

summary
  fatal: 1
  component: 1
  ignored: 0
  advisory: 0
  errors: 1
  exit-code: 1
"
        );
    }

    #[test]
    fn json_is_the_compact_report_document() {
        let report = report();
        let json = Format::Json.render(&report);
        assert_eq!(
            json,
            format!("{}\n", serde_json::to_string(&report).unwrap())
        );
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["schemaVersion"], 1);
        assert_eq!(parsed["exitCode"], 1);
        assert_eq!(
            parsed["plugins"][0]["findings"][0]["ruleId"],
            "AP-NAME-CHARSET"
        );
        assert_eq!(
            parsed["plugins"][0]["findings"][0]["evidenceCode"],
            "NAME_CHARSET"
        );
    }
}
