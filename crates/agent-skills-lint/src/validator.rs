use crate::parser::{Frontmatter, ParseError, SkillIoError, parse_frontmatter, read_source};
use crate::{
    IssueLevel, SkillCoverage, SkillCoverageStatus, SkillIssue, SkillIssueKind, SkillProperties,
    SkillReport, SkillRule, YamlValue,
};
use std::path::Path;
const NAME: usize = 64;
const DESCRIPTION: usize = 1024;
const COMPATIBILITY: usize = 500;
/// 校验源码；该核心 API 不访问文件系统。
pub fn validate_source(source: &str, directory_name: &str) -> SkillReport {
    let mut report = SkillReport {
        properties: None,
        issues: Vec::new(),
        coverage: Vec::new(),
    };
    match parse_frontmatter(source) {
        Ok(fm) => fields(source, &fm, directory_name, &mut report),
        Err(error) => parse_error(error, &mut report),
    };
    finish(&mut report);
    report
}
/// 目录便利 API；只有文件系统失败会作为 `Err` 返回。
pub fn validate_directory(dir: &Path) -> Result<SkillReport, SkillIoError> {
    let directory_name = dir
        .file_name()
        .and_then(|x| x.to_str())
        .ok_or_else(|| SkillIoError::NonUtf8Directory(dir.to_path_buf()))?;
    let source = read_source(dir)?;
    Ok(validate_source(&source, directory_name))
}
fn parse_error(error: ParseError, report: &mut SkillReport) {
    let (kind, level) = match error {
        ParseError::MissingFrontmatter => (SkillIssueKind::MissingFrontmatter, IssueLevel::Error),
        ParseError::UnclosedFrontmatter => (SkillIssueKind::UnclosedFrontmatter, IssueLevel::Error),
        ParseError::InvalidYaml => (SkillIssueKind::InvalidYaml, IssueLevel::Error),
        ParseError::NotMapping => (SkillIssueKind::NotMapping, IssueLevel::Error),
        ParseError::UnsupportedYaml => (SkillIssueKind::UnsupportedYaml, IssueLevel::Unchecked),
    };
    issue(
        report,
        SkillRule::Frontmatter,
        kind,
        None,
        level,
        None,
        None,
    )
}
fn fields(source: &str, fm: &Frontmatter, dir: &str, report: &mut SkillReport) {
    for key in fm.fields.keys() {
        if ![
            "name",
            "description",
            "license",
            "compatibility",
            "metadata",
            "allowed-tools",
        ]
        .contains(&key.as_str())
        {
            issue(
                report,
                SkillRule::OptionalFields,
                SkillIssueKind::UnknownField,
                Some(key),
                IssueLevel::Unchecked,
                None,
                None,
            )
        }
    }
    string(fm, "name", SkillRule::Name, true, report, |v, r| {
        name(v, dir, r)
    });
    string(
        fm,
        "description",
        SkillRule::Description,
        true,
        report,
        |v, r| length("description", v, DESCRIPTION, SkillRule::Description, r),
    );
    for key in ["license", "allowed-tools"] {
        string(fm, key, SkillRule::OptionalFields, false, report, |_, _| {})
    }
    string(
        fm,
        "compatibility",
        SkillRule::OptionalFields,
        false,
        report,
        |v, r| {
            if v.is_empty() {
                issue(
                    r,
                    SkillRule::OptionalFields,
                    SkillIssueKind::Empty,
                    Some("compatibility"),
                    IssueLevel::Error,
                    Some(0),
                    Some(1),
                )
            }
            length(
                "compatibility",
                v,
                COMPATIBILITY,
                SkillRule::OptionalFields,
                r,
            )
        },
    );
    metadata(fm, report);
    if !report.has_errors() {
        report.properties = properties(fm)
    }
    if source.lines().count() >= 500 {
        issue(
            report,
            SkillRule::SizeGuidance,
            SkillIssueKind::TooManyLines,
            None,
            IssueLevel::Advisory,
            Some(source.lines().count()),
            Some(499),
        )
    }
}
fn string<F: FnOnce(&str, &mut SkillReport)>(
    fm: &Frontmatter,
    key: &str,
    rule: SkillRule,
    required: bool,
    report: &mut SkillReport,
    f: F,
) {
    match fm.fields.get(key) {
        None if required => issue(
            report,
            rule,
            SkillIssueKind::MissingField,
            Some(key),
            IssueLevel::Error,
            None,
            None,
        ),
        None => {}
        Some(YamlValue::String(value)) if required && value.is_empty() => issue(
            report,
            rule,
            SkillIssueKind::Empty,
            Some(key),
            IssueLevel::Error,
            Some(0),
            Some(1),
        ),
        Some(YamlValue::String(value)) => f(value, report),
        Some(_) => issue(
            report,
            rule,
            SkillIssueKind::WrongType,
            Some(key),
            IssueLevel::Error,
            None,
            None,
        ),
    }
}
fn name(value: &str, dir: &str, report: &mut SkillReport) {
    length("name", value, NAME, SkillRule::Name, report);
    if !value.is_ascii() {
        issue(
            report,
            SkillRule::Name,
            SkillIssueKind::AmbiguousUnicode,
            Some("name"),
            IssueLevel::Unchecked,
            None,
            None,
        );
        return;
    }
    if !value
        .bytes()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        issue(
            report,
            SkillRule::Name,
            SkillIssueKind::InvalidCharacters,
            Some("name"),
            IssueLevel::Error,
            None,
            None,
        )
    }
    if value.starts_with('-') || value.ends_with('-') || value.contains("--") {
        issue(
            report,
            SkillRule::Name,
            SkillIssueKind::InvalidHyphens,
            Some("name"),
            IssueLevel::Error,
            None,
            None,
        )
    }
    if dir.is_ascii() && value != dir {
        issue(
            report,
            SkillRule::Name,
            SkillIssueKind::DirectoryMismatch,
            Some("name"),
            IssueLevel::Error,
            None,
            None,
        )
    } else if !dir.is_ascii() && value != dir {
        issue(
            report,
            SkillRule::Name,
            SkillIssueKind::AmbiguousUnicode,
            Some("name"),
            IssueLevel::Unchecked,
            None,
            None,
        )
    }
}
fn length(field: &str, value: &str, limit: usize, rule: SkillRule, report: &mut SkillReport) {
    let actual = value.chars().count();
    if actual > limit {
        issue(
            report,
            rule,
            SkillIssueKind::TooLong,
            Some(field),
            IssueLevel::Error,
            Some(actual),
            Some(limit),
        )
    }
}
fn metadata(fm: &Frontmatter, report: &mut SkillReport) {
    let Some(value) = fm.fields.get("metadata") else {
        return;
    };
    let Some(map) = value.mapping() else {
        issue(
            report,
            SkillRule::OptionalFields,
            SkillIssueKind::WrongType,
            Some("metadata"),
            IssueLevel::Error,
            None,
            None,
        );
        return;
    };
    for (key, value) in map {
        if value.string().is_none() {
            issue(
                report,
                SkillRule::OptionalFields,
                SkillIssueKind::WrongType,
                Some(&format!("metadata.{key}")),
                IssueLevel::Error,
                None,
                None,
            )
        }
    }
}
fn properties(fm: &Frontmatter) -> Option<SkillProperties> {
    let s = |key| {
        fm.fields
            .get(key)
            .and_then(YamlValue::string)
            .map(str::to_owned)
    };
    Some(SkillProperties {
        name: s("name")?,
        description: s("description")?,
        license: s("license"),
        compatibility: s("compatibility"),
        allowed_tools: s("allowed-tools"),
        metadata: fm
            .fields
            .get("metadata")
            .and_then(YamlValue::mapping)
            .map(|m| {
                m.iter()
                    .filter_map(|(k, v)| v.string().map(|v| (k.clone(), v.to_owned())))
                    .collect()
            }),
    })
}
fn issue(
    report: &mut SkillReport,
    rule: SkillRule,
    kind: SkillIssueKind,
    field: Option<&str>,
    level: IssueLevel,
    actual: Option<usize>,
    limit: Option<usize>,
) {
    report.issues.push(SkillIssue {
        rule,
        kind,
        field: field.map(str::to_owned),
        level,
        location: None,
        actual,
        limit,
    })
}
fn finish(report: &mut SkillReport) {
    for rule in [
        SkillRule::Frontmatter,
        SkillRule::Name,
        SkillRule::Description,
        SkillRule::OptionalFields,
        SkillRule::SizeGuidance,
    ] {
        let status = if report
            .issues
            .iter()
            .any(|x| x.rule == SkillRule::Frontmatter)
            && rule != SkillRule::Frontmatter
        {
            SkillCoverageStatus::Blocked
        } else if report
            .issues
            .iter()
            .any(|x| x.rule == rule && x.level == IssueLevel::Error)
        {
            SkillCoverageStatus::Fail
        } else if report
            .issues
            .iter()
            .any(|x| x.rule == rule && x.level == IssueLevel::Unchecked)
        {
            SkillCoverageStatus::Unchecked
        } else if rule == SkillRule::SizeGuidance {
            SkillCoverageStatus::Manual
        } else {
            SkillCoverageStatus::Pass
        };
        report.coverage.push(SkillCoverage { rule, status });
    }
    report
        .issues
        .sort_by_key(|x| (x.rule, x.field.clone(), x.kind));
}
