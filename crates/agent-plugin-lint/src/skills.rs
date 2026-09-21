use crate::{Confidence, Coverage, CoverageStatus, Finding, RuleId, Scope, Subject};
use agent_skills_lint::{IssueLevel, SkillCoverageStatus, SkillRule, validate_source};
use std::collections::BTreeMap;

fn rule(rule: SkillRule) -> (RuleId, &'static str) {
    match rule {
        SkillRule::Frontmatter => (RuleId::AsFrontmatter, "AS-FRONTMATTER"),
        SkillRule::Name => (RuleId::AsName, "AS-NAME"),
        SkillRule::Description => (RuleId::AsDescription, "AS-DESCRIPTION"),
        SkillRule::OptionalFields => (RuleId::AsOptionalFields, "AS-OPTIONAL-FIELDS"),
        SkillRule::SizeGuidance => (RuleId::AsSizeGuidance, "AS-SIZE-GUIDANCE"),
    }
}
fn coverage_status(status: SkillCoverageStatus) -> CoverageStatus {
    match status {
        SkillCoverageStatus::Pass => CoverageStatus::Pass,
        SkillCoverageStatus::Fail => CoverageStatus::Fail,
        SkillCoverageStatus::Blocked => CoverageStatus::Blocked,
        SkillCoverageStatus::Unchecked => CoverageStatus::Unchecked,
        SkillCoverageStatus::Manual => CoverageStatus::Manual,
    }
}

/// The skill library owns per-rule coverage. Issues only create findings: a
/// rule can have several issues, but it must yield one coverage record.
pub fn validate(
    source: &str,
    logical_name: &str,
    logical_path: String,
    findings: &mut Vec<Finding>,
    coverage: &mut Vec<Coverage>,
) {
    let report = validate_source(source, logical_name);
    for issue in report.issues {
        let (rule, _) = rule(issue.rule);
        if issue.level != IssueLevel::Unchecked {
            let meta = rule.metadata();
            findings.push(Finding {
                rule_id: rule,
                spec: meta.spec.iter().map(|s| (*s).into()).collect(),
                radius: meta.radius,
                normative: meta.normative,
                obligation: meta.obligation,
                subject: Subject::Package,
                confidence: Confidence::Certain,
                path: logical_path.clone(),
                pointer: None,
                scope: Scope::Skill(logical_name.into()),
                effect: meta.effect,
                evidence_code: format!("{:?}", issue.kind),
                message: if issue.level == IssueLevel::Advisory {
                    "Skill 建议项需要处理".into()
                } else {
                    "Skill 格式不符合 Agent Skills 规范，已跳过该 skill".into()
                },
                hint: issue.field.clone(),
                line: issue.location.map(|x| x.line),
                column: issue.location.map(|x| x.column),
            });
        }
    }
    let mut library_coverage = BTreeMap::new();
    for item in report.coverage {
        let (_, id) = rule(item.rule);
        library_coverage.entry(id).or_insert(item.status);
    }
    for (id, status) in library_coverage {
        coverage.push(Coverage {
            rule_id: id.into(),
            target: logical_path.clone(),
            status: coverage_status(status),
            reason_code: Some("SKILL_LIBRARY".into()),
        });
    }
}
