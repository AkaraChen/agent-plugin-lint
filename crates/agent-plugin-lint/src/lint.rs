use crate::manifest::{invalid_json_manifest, validate_manifest};
use crate::{
    Coverage, CoverageStatus, Finding, InputMode, Obligation, PluginReport, Policy, Radius, Report,
    RuleId, Scope, Subject, Summary, ToolError,
};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy)]
pub struct LintOptions {
    pub mode: InputMode,
    pub strict: bool,
}
impl Default for LintOptions {
    fn default() -> Self {
        Self {
            mode: InputMode::Auto,
            strict: false,
        }
    }
}

pub fn lint_path(path: &Path, options: LintOptions) -> Report {
    let mut report = empty_report(path, options);
    if path.to_str().is_none() {
        error(
            &mut report.errors,
            path,
            "PATH_ENCODING",
            "路径不能以 UTF-8 表示",
        );
        finish_report(&mut report);
        return report;
    }
    let roots = select_roots(path, options.mode, &mut report.errors);
    let mut canonical_roots = BTreeSet::new();
    for root in roots {
        match fs::canonicalize(&root) {
            Ok(canonical) if !canonical_roots.insert(canonical.clone()) => {
                error(
                    &mut report.errors,
                    &root,
                    "DISCOVERY_AMBIGUOUS_ROOT",
                    "多个逻辑包根指向同一真实目录",
                );
            }
            Ok(_) => {
                let label = root_label(path, &root);
                let outcome = lint_plugin(&root, label);
                if let Some(tool_error) = outcome.error {
                    report.errors.push(tool_error);
                }
                report.plugins.push(outcome.plugin);
            }
            Err(_) => error(
                &mut report.errors,
                &root,
                "ROOT_UNRESOLVED",
                "无法解析包根目录",
            ),
        }
    }
    finish_report(&mut report);
    report
}

fn empty_report(path: &Path, options: LintOptions) -> Report {
    Report {
        schema_version: 1,
        tool_version: env!("CARGO_PKG_VERSION").into(),
        ruleset_version: "1.0.0".into(),
        spec_version: "1.0.0".into(),
        input: display(path),
        mode: options.mode,
        policy: Policy {
            strict: options.strict,
        },
        complete: true,
        plugins: vec![],
        errors: vec![],
        summary: Summary {
            fatal: 0,
            component: 0,
            ignored: 0,
            advisory: 0,
            errors: 0,
        },
        exit_code: 0,
    }
}

fn select_roots(path: &Path, mode: InputMode, errors: &mut Vec<ToolError>) -> Vec<PathBuf> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) if metadata.is_dir() => metadata,
        Ok(_) => {
            error(errors, path, "INPUT_NOT_DIRECTORY", "输入路径必须是目录");
            return vec![];
        }
        Err(_) => {
            error(errors, path, "INPUT_UNREADABLE", "无法读取输入目录");
            return vec![];
        }
    };
    let _ = metadata;
    match mode {
        InputMode::Plugin => vec![path.to_path_buf()],
        InputMode::Collection => {
            let children = direct_children(path, errors);
            if children.is_empty() && errors.is_empty() {
                error(
                    errors,
                    path,
                    "DISCOVERY_EMPTY_COLLECTION",
                    "集合模式没有直接子包",
                );
            }
            children
        }
        InputMode::Auto => {
            match manifest_marker(path) {
                Ok(true) => return vec![path.to_path_buf()],
                Ok(false) => {}
                Err(tool_error) => {
                    errors.push(tool_error);
                    return vec![];
                }
            }
            let children = direct_children(path, errors);
            if !errors.is_empty() {
                return vec![];
            }
            if children.is_empty() {
                error(
                    errors,
                    path,
                    "DISCOVERY_AMBIGUOUS",
                    "自动模式未找到包或完整集合",
                );
                return vec![];
            }
            let mut markers = 0;
            for child in &children {
                match manifest_marker(child) {
                    Ok(true) => markers += 1,
                    Ok(false) => {}
                    Err(tool_error) => errors.push(tool_error),
                }
            }
            if !errors.is_empty() {
                return vec![];
            }
            if markers == children.len() {
                children
            } else {
                error(
                    errors,
                    path,
                    "DISCOVERY_AMBIGUOUS",
                    "自动模式的直接子目录不是完整集合",
                );
                vec![]
            }
        }
    }
}

fn direct_children(path: &Path, errors: &mut Vec<ToolError>) -> Vec<PathBuf> {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => {
            error(errors, path, "DISCOVERY_READ", "无法枚举输入目录");
            return vec![];
        }
    };
    let mut children = vec![];
    for entry in entries {
        match entry {
            Ok(entry) if entry.file_name().to_str().is_none() => error(
                errors,
                &entry.path(),
                "PATH_ENCODING",
                "目录项路径不能以 UTF-8 表示",
            ),
            Ok(entry) => match fs::metadata(entry.path()) {
                Ok(metadata) if metadata.is_dir() => children.push(entry.path()),
                Ok(_) => {}
                Err(_) => error(
                    errors,
                    &entry.path(),
                    "DISCOVERY_ENTRY",
                    "无法读取目录项类型",
                ),
            },
            Err(_) => error(errors, path, "DISCOVERY_ENTRY", "无法读取目录项"),
        }
    }
    children.sort();
    children
}

fn manifest_marker(root: &Path) -> Result<bool, ToolError> {
    match fs::read_dir(root) {
        Ok(entries) => {
            for entry in entries {
                let entry = entry
                    .map_err(|_| tool_error(root, "MANIFEST_ENUMERATE", "无法枚举包根目录"))?;
                if entry.file_name() == "plugin.json" {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err(tool_error(root, "MANIFEST_ENUMERATE", "无法枚举包根目录")),
    }
}

struct PluginOutcome {
    plugin: PluginReport,
    error: Option<ToolError>,
}

fn lint_plugin(root: &Path, root_name: String) -> PluginOutcome {
    match manifest_marker(root) {
        Ok(true) => {}
        Ok(false) => {
            return outcome(location_report(
                root_name,
                "MANIFEST_MISSING",
                "包根缺少 plugin.json",
            ));
        }
        Err(tool_error) => {
            return PluginOutcome {
                plugin: unread_plugin(root_name),
                error: Some(tool_error),
            };
        }
    }
    let root_canonical = match fs::canonicalize(root) {
        Ok(root) => root,
        Err(_) => return outcome_error(root_name, root, "ROOT_UNRESOLVED", "无法解析包根目录"),
    };
    let manifest_path = root.join("plugin.json");
    match fs::symlink_metadata(&manifest_path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => outcome(location_report(
            root_name,
            "MANIFEST_MISSING",
            "包根缺少 plugin.json",
        )),
        Err(_) => outcome_error(
            root_name,
            &manifest_path,
            "MANIFEST_METADATA",
            "无法读取 plugin.json 元数据",
        ),
        Ok(_) => {
            let canonical_manifest = match fs::canonicalize(&manifest_path) {
                Ok(path) => path,
                Err(error) if unresolved_location(&error) => {
                    return outcome(location_report(
                        root_name,
                        "MANIFEST_UNRESOLVED",
                        "plugin.json 链接无法解析",
                    ));
                }
                Err(_) => {
                    return outcome_error(
                        root_name,
                        &manifest_path,
                        "MANIFEST_CANONICALIZE",
                        "无法解析 plugin.json",
                    );
                }
            };
            if !canonical_manifest.starts_with(&root_canonical) {
                return outcome(path_escape_report(root_name));
            }
            match fs::metadata(&canonical_manifest) {
                Ok(metadata) if metadata.is_file() => match fs::read_to_string(&canonical_manifest)
                {
                    Ok(source) => match serde_json::from_str::<Value>(&source) {
                        Ok(value) => outcome(from_validation(root_name, validate_manifest(&value))),
                        Err(_) => outcome(from_validation(root_name, invalid_json_manifest())),
                    },
                    Err(_) => outcome_error(
                        root_name,
                        &canonical_manifest,
                        "MANIFEST_READ",
                        "无法读取 plugin.json 内容",
                    ),
                },
                Ok(_) => outcome(location_report(
                    root_name,
                    "MANIFEST_NOT_REGULAR",
                    "plugin.json 必须是普通文件",
                )),
                Err(_) => outcome_error(
                    root_name,
                    &canonical_manifest,
                    "MANIFEST_METADATA",
                    "无法读取 plugin.json 元数据",
                ),
            }
        }
    }
}

fn from_validation(root: String, validation: crate::ManifestValidation) -> PluginReport {
    let mut coverage = validation.coverage;
    let status = if validation.rejected {
        "rejected"
    } else {
        "accepted"
    }
    .into();
    let gate = if validation.rejected {
        CoverageStatus::Blocked
    } else {
        CoverageStatus::Unchecked
    };
    let reason = if validation.rejected {
        "MANIFEST_REJECTED"
    } else {
        "NOT_IMPLEMENTED_S2B"
    };
    for rule in [
        RuleId::AdviceDuplicateJsonKey,
        RuleId::McpEnvelope,
        RuleId::SkillConformance,
    ] {
        let (status, reason_code) = if rule == RuleId::AdviceDuplicateJsonKey {
            (CoverageStatus::Unchecked, "NOT_IMPLEMENTED_S2B")
        } else {
            (gate, reason)
        };
        coverage.push(Coverage {
            rule_id: rule.as_str().into(),
            target: "plugin.json".into(),
            status,
            reason_code: Some(reason_code.into()),
        });
    }
    sort_coverage(&mut coverage);
    PluginReport {
        root,
        name: validation.name,
        declared_spec: validation.declared_spec,
        status,
        findings: validation.findings,
        coverage,
    }
}

fn location_report(root: String, code: &str, message: &str) -> PluginReport {
    one_finding_report(root, RuleId::ManifestLocation, code, message)
}
fn path_escape_report(root: String) -> PluginReport {
    one_finding_report(
        root,
        RuleId::PathManifestEscape,
        "MANIFEST_OUTSIDE_ROOT",
        "plugin.json 位于包根之外，未读取其内容",
    )
}
fn one_finding_report(root: String, rule: RuleId, code: &str, message: &str) -> PluginReport {
    let meta = rule.metadata();
    let finding = Finding {
        rule_id: rule,
        spec: meta.spec.iter().map(|item| (*item).into()).collect(),
        radius: meta.radius,
        normative: meta.normative,
        obligation: meta.obligation,
        subject: meta.subject,
        confidence: meta.confidence,
        path: "plugin.json".into(),
        pointer: None,
        scope: Scope::Plugin,
        effect: meta.effect,
        evidence_code: code.into(),
        message: message.into(),
        hint: None,
    };
    PluginReport {
        root,
        name: None,
        declared_spec: None,
        status: "rejected".into(),
        findings: vec![finding],
        coverage: unread_coverage(Some(rule)),
    }
}
fn unread_coverage(failed_rule: Option<RuleId>) -> Vec<Coverage> {
    let mut coverage = [
        RuleId::ManifestLocation,
        RuleId::PathManifestEscape,
        RuleId::ManifestJson,
        RuleId::ManifestUnknownField,
        RuleId::ManifestRequired,
        RuleId::ManifestSchemaId,
        RuleId::ManifestMetadataType,
        RuleId::ManifestAuthor,
        RuleId::NameLength,
        RuleId::NameCharset,
        RuleId::NameEnds,
        RuleId::NameRepetition,
        RuleId::VersionSemver,
        RuleId::ExtensionsObject,
        RuleId::ExtensionUnknown,
        RuleId::AdviceDuplicateJsonKey,
        RuleId::McpEnvelope,
        RuleId::SkillConformance,
    ]
    .into_iter()
    .map(|rule| Coverage {
        rule_id: rule.as_str().into(),
        target: "plugin.json".into(),
        status: match failed_rule {
            Some(failed_rule) if rule == failed_rule => CoverageStatus::Fail,
            None if rule == RuleId::ManifestLocation => CoverageStatus::Unchecked,
            _ => CoverageStatus::Blocked,
        },
        reason_code: Some(
            match failed_rule {
                Some(failed_rule) if rule == failed_rule => "MANIFEST_GATE_FAILURE",
                None => "MANIFEST_IO",
                _ => "MANIFEST_UNREAD",
            }
            .into(),
        ),
    })
    .collect::<Vec<_>>();
    sort_coverage(&mut coverage);
    coverage
}
fn outcome(plugin: PluginReport) -> PluginOutcome {
    PluginOutcome {
        plugin,
        error: None,
    }
}
fn outcome_error(root: String, path: &Path, code: &str, message: &str) -> PluginOutcome {
    let plugin = unread_plugin(root);
    PluginOutcome {
        plugin,
        error: Some(tool_error(path, code, message)),
    }
}
fn unread_plugin(root: String) -> PluginReport {
    PluginReport {
        root,
        name: None,
        declared_spec: None,
        status: "tool-error".into(),
        findings: vec![],
        coverage: unread_coverage(None),
    }
}
fn unresolved_location(error: &std::io::Error) -> bool {
    error.kind() == std::io::ErrorKind::NotFound || (cfg!(unix) && error.raw_os_error() == Some(40)) // ELOOP
}

fn finish_report(report: &mut Report) {
    report
        .plugins
        .sort_by(|left, right| left.root.cmp(&right.root));
    report
        .errors
        .sort_by(|left, right| (&left.path, &left.code).cmp(&(&right.path, &right.code)));
    let mut summary = Summary {
        fatal: 0,
        component: 0,
        ignored: 0,
        advisory: 0,
        errors: report.errors.len(),
    };
    let mut must_violation = false;
    let mut strict_violation = false;
    for plugin in &mut report.plugins {
        plugin.findings.sort_by(|left, right| {
            (
                &left.path,
                &left.pointer,
                left.rule_id.as_str(),
                scope_key(&left.scope),
                left.effect.as_str(),
            )
                .cmp(&(
                    &right.path,
                    &right.pointer,
                    right.rule_id.as_str(),
                    scope_key(&right.scope),
                    right.effect.as_str(),
                ))
        });
        sort_coverage(&mut plugin.coverage);
        for finding in &plugin.findings {
            match finding.radius {
                Radius::Fatal => summary.fatal += 1,
                Radius::Component => summary.component += 1,
                Radius::Ignored => summary.ignored += 1,
                Radius::Advisory => summary.advisory += 1,
            }
            must_violation |= finding.normative
                && finding.confidence == crate::Confidence::Certain
                && finding.subject == Subject::Package
                && finding.obligation == Obligation::Must;
            strict_violation |= finding.radius == Radius::Advisory;
        }
        strict_violation |= plugin.coverage.iter().any(|coverage| {
            coverage.rule_id == RuleId::AdviceDuplicateJsonKey.as_str()
                && coverage.status == CoverageStatus::Unchecked
        });
    }
    report.summary = summary;
    report.complete = report.errors.is_empty();
    report.exit_code = if !report.errors.is_empty() {
        2
    } else if must_violation || (report.policy.strict && strict_violation) {
        1
    } else {
        0
    };
}
fn sort_coverage(coverage: &mut [Coverage]) {
    coverage.sort_by(|left, right| {
        (&left.rule_id, &left.target, left.status.as_str()).cmp(&(
            &right.rule_id,
            &right.target,
            right.status.as_str(),
        ))
    });
}
fn error(errors: &mut Vec<ToolError>, path: &Path, code: &str, message: &str) {
    errors.push(tool_error(path, code, message));
}
fn tool_error(path: &Path, code: &str, message: &str) -> ToolError {
    ToolError {
        path: display(path),
        code: code.into(),
        message: message.into(),
    }
}
fn root_label(input: &Path, root: &Path) -> String {
    if root == input {
        ".".into()
    } else {
        root.strip_prefix(input)
            .ok()
            .and_then(Path::to_str)
            .unwrap_or("<non-utf8-path>")
            .into()
    }
}
fn scope_key(scope: &Scope) -> (&str, Option<&str>) {
    match scope {
        Scope::Plugin => ("plugin", None),
        Scope::ComponentType(value) => ("component-type", Some(value)),
        Scope::Skill(value) => ("skill", Some(value)),
        Scope::Server(value) => ("server", Some(value)),
        Scope::Path(value) => ("path", Some(value)),
    }
}
fn display(path: &Path) -> String {
    path.to_str().unwrap_or("<non-utf8-path>").into()
}
