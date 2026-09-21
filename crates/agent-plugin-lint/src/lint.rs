use crate::json::{self, Parsed};
use crate::manifest::{invalid_json_manifest, validate_manifest_document};
use crate::{
    Coverage, CoverageStatus, Finding, InputMode, Obligation, PluginReport, Policy, Radius, Report,
    RuleId, Scope, Subject, Summary, ToolError,
};
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
                report.errors.extend(outcome.errors);
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
    errors: Vec<ToolError>,
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
                errors: vec![tool_error],
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
                Ok(metadata) if metadata.is_file() => {
                    match crate::containment::read_safe(&root_canonical, &manifest_path) {
                        Ok(source) => match json::parse(&source) {
                            Parsed::Value {
                                value,
                                duplicate_keys,
                            } => from_validation(
                                root_name,
                                &root_canonical,
                                validate_manifest_document(&value, duplicate_keys),
                            ),
                            Parsed::Syntax => {
                                from_validation(root_name, &root_canonical, invalid_json_manifest())
                            }
                            Parsed::Representation => outcome_representation(
                                root_name,
                                &manifest_path,
                                "MANIFEST_JSON_REPRESENTATION",
                                "plugin.json 超出当前解析器表示能力（如数值范围或嵌套深度）",
                            ),
                        },
                        Err(crate::containment::ReadError::InputChanged) => outcome_error(
                            root_name,
                            &manifest_path,
                            "INPUT_CHANGED",
                            "读取 plugin.json 时输入发生变化",
                        ),
                        Err(crate::containment::ReadError::Unsafe) => outcome_error(
                            root_name,
                            &manifest_path,
                            "MANIFEST_READ",
                            "plugin.json 不是可安全读取的普通文件",
                        ),
                        Err(crate::containment::ReadError::Unsupported) => {
                            outcome_safe_read_unsupported(root_name, &manifest_path)
                        }
                        Err(read_error @ crate::containment::ReadError::Io(_)) => {
                            let _ = read_error.io_kind();
                            outcome_error(
                                root_name,
                                &canonical_manifest,
                                "MANIFEST_READ",
                                "无法读取 plugin.json 内容",
                            )
                        }
                    }
                }
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

fn from_validation(
    root: String,
    root_canonical: &Path,
    validation: crate::ManifestValidation,
) -> PluginOutcome {
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
            match validation.duplicate_json_keys {
                Some(true) => (CoverageStatus::Fail, "DUPLICATE_JSON_KEY"),
                Some(false) => (CoverageStatus::Pass, "NO_DUPLICATE_JSON_KEY"),
                None => (CoverageStatus::Unchecked, "SOURCE_UNAVAILABLE"),
            }
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
    let mut plugin = PluginReport {
        root,
        name: validation.name,
        declared_spec: validation.declared_spec,
        status,
        findings: validation.findings,
        coverage,
    };
    plugin.coverage.push(Coverage {
        rule_id: RuleId::ManifestLocation.as_str().into(),
        target: "plugin.json".into(),
        status: CoverageStatus::Pass,
        reason_code: Some("MANIFEST_AT_ROOT".into()),
    });
    plugin.coverage.push(Coverage {
        rule_id: RuleId::PathManifestEscape.as_str().into(),
        target: "plugin.json".into(),
        status: CoverageStatus::Pass,
        reason_code: Some("MANIFEST_CONTAINED".into()),
    });
    // Preserve context required to distinguish an absent target from a rule
    // the current static scanner deliberately does not implement.
    let has_license = validation.has_license;
    let has_extensions = validation.has_extensions;
    if validation.duplicate_json_keys == Some(true) {
        add_finding(
            &mut plugin,
            RuleId::AdviceDuplicateJsonKey,
            "plugin.json".into(),
            Scope::Plugin,
            "DUPLICATE_JSON_KEY",
            "JSON 对象存在重复键；后续校验按最后一个值进行",
        );
    }
    let mut errors = vec![];
    if !validation.rejected {
        plugin.coverage.retain(|c| {
            c.rule_id != RuleId::SkillConformance.as_str()
                && c.rule_id != RuleId::McpEnvelope.as_str()
        });
        scan_components(root_canonical, &mut plugin, &mut errors);
        let skill_coverages: Vec<_> = plugin
            .coverage
            .iter()
            .filter(|c| {
                c.target.ends_with("/SKILL.md")
                    && matches!(
                        c.rule_id.as_str(),
                        "AS-FRONTMATTER"
                            | "AS-NAME"
                            | "AS-DESCRIPTION"
                            | "AS-OPTIONAL-FIELDS"
                            | "AP-SKILL-CONFORMANCE"
                    )
            })
            .collect();
        if !skill_coverages.is_empty() {
            let status = if skill_coverages.iter().any(|c| {
                matches!(
                    c.rule_id.as_str(),
                    "AS-FRONTMATTER" | "AS-NAME" | "AS-DESCRIPTION" | "AS-OPTIONAL-FIELDS"
                ) && c.status == CoverageStatus::Fail
            }) {
                CoverageStatus::Fail
            } else if skill_coverages.iter().any(|c| {
                c.status == CoverageStatus::Unchecked || c.status == CoverageStatus::Blocked
            }) {
                CoverageStatus::Unchecked
            } else {
                CoverageStatus::Pass
            };
            let has_unchecked = skill_coverages.iter().any(|c| {
                matches!(
                    c.status,
                    CoverageStatus::Unchecked | CoverageStatus::Blocked
                )
            });
            plugin.coverage.push(Coverage {
                rule_id: RuleId::SkillConformance.as_str().into(),
                target: "skills".into(),
                status,
                reason_code: Some("SKILL_AGGREGATE".into()),
            });
            if has_unchecked {
                add_finding(
                    &mut plugin,
                    RuleId::AdviceSkillsUnchecked,
                    "skills".into(),
                    Scope::ComponentType("skills".into()),
                    "SKILL_UNCHECKED",
                    "部分 skill 规则未检查，不能视为完全合规",
                );
            }
        }
    }
    annotate_unimplemented_context(&mut plugin, has_license, has_extensions);
    PluginOutcome { plugin, errors }
}

fn annotate_unimplemented_context(
    plugin: &mut PluginReport,
    has_license: bool,
    has_extensions: bool,
) {
    // Manual author/runtime questions are meaningful only after the MCP
    // envelope was accepted.  A rejected envelope blocks those questions;
    // it must not receive a second, contradictory manual state.
    let mcp_present = plugin.coverage.iter().any(|coverage| {
        coverage.rule_id == RuleId::McpEnvelope.as_str() && coverage.status == CoverageStatus::Pass
    });
    let reason = if plugin.status == "rejected" {
        "MANIFEST_REJECTED"
    } else {
        "STATIC_NOT_IMPLEMENTED"
    };
    if has_license {
        plugin.coverage.push(Coverage {
            rule_id: "AP-LICENSE-SPDX".into(),
            target: "plugin.json#/license".into(),
            status: if plugin.status != "accepted" {
                CoverageStatus::Blocked
            } else {
                CoverageStatus::Manual
            },
            reason_code: Some(if plugin.status != "accepted" {
                reason.into()
            } else {
                "PUBLISHER_MANUAL".into()
            }),
        });
    }
    if has_extensions {
        let extensions_ignored = plugin.coverage.iter().any(|coverage| {
            coverage.rule_id == RuleId::ExtensionsObject.as_str()
                && coverage.status == CoverageStatus::Fail
        });
        for id in ["AP-EXTENSION-VALUE", "AP-EXTENSION-FILE-LOCATION"] {
            plugin.coverage.push(Coverage {
                rule_id: id.into(),
                target: "plugin.json#/extensions".into(),
                status: if plugin.status != "accepted" || extensions_ignored {
                    CoverageStatus::Blocked
                } else {
                    CoverageStatus::Manual
                },
                reason_code: Some(if plugin.status == "rejected" {
                    "MANIFEST_REJECTED".into()
                } else {
                    "UNIMPLEMENTED_NAMESPACE".into()
                }),
            });
        }
    }
    if mcp_present {
        plugin.coverage.push(Coverage {
            rule_id: "AP-ENV-BASE-DEPENDENCE".into(),
            target: "mcp.json".into(),
            status: if plugin.status != "accepted" {
                CoverageStatus::Blocked
            } else {
                CoverageStatus::Manual
            },
            reason_code: Some(if plugin.status != "accepted" {
                reason.into()
            } else {
                "RUNTIME_ENVIRONMENT".into()
            }),
        });
        for id in [
            "AP-MCP-BUNDLED-COMMAND",
            "AP-MCP-PATH-DEPENDENCE",
            "AP-MCP-HEADER-SECRETS",
            "AP-MCP-ENV-SECRETS",
        ] {
            plugin.coverage.push(Coverage {
                rule_id: id.into(),
                target: "mcp.json".into(),
                status: if plugin.status != "accepted" {
                    CoverageStatus::Blocked
                } else {
                    CoverageStatus::Manual
                },
                reason_code: Some(if plugin.status != "accepted" {
                    reason.into()
                } else {
                    "PUBLISHER_OR_RUNTIME_MANUAL".into()
                }),
            });
        }
        if plugin.coverage.iter().any(|coverage| {
            coverage.rule_id == RuleId::McpEnvelope.as_str()
                && coverage.status == CoverageStatus::Pass
        }) {
            for id in [RuleId::DiscoveryKind, RuleId::PathFixedEscape] {
                plugin.coverage.push(Coverage {
                    rule_id: id.as_str().into(),
                    target: "mcp.json".into(),
                    status: CoverageStatus::Pass,
                    reason_code: Some("MCP_FIXED_FILE_CONTAINED".into()),
                });
            }
        }
    }
    // Description quality requires the body.  Do not add a manual result for
    // a skill whose safe read failed: that would contradict its blocked
    // frontmatter/field checks.
    if plugin.coverage.iter().any(|coverage| {
        coverage.rule_id == RuleId::AsDescription.as_str()
            && coverage.target.ends_with("/SKILL.md")
            && coverage.reason_code.as_deref() == Some("SKILL_LIBRARY")
    }) {
        plugin.coverage.push(Coverage {
            rule_id: "AS-DESCRIPTION-QUALITY".into(),
            target: "skills".into(),
            status: CoverageStatus::Manual,
            reason_code: Some("QUALITY_MANUAL".into()),
        });
    }
}

fn block_unread_skill(plugin: &mut PluginReport, target: &str) {
    for rule in [
        RuleId::AsFrontmatter,
        RuleId::AsName,
        RuleId::AsDescription,
        RuleId::AsOptionalFields,
        RuleId::AsSizeGuidance,
    ] {
        plugin.coverage.push(Coverage {
            rule_id: rule.as_str().into(),
            target: target.into(),
            status: CoverageStatus::Blocked,
            reason_code: Some("SAFE_READ_UNSUPPORTED".into()),
        });
    }
}

fn outcome_representation(root: String, path: &Path, code: &str, message: &str) -> PluginOutcome {
    let mut plugin = unread_plugin(root);
    for coverage in &mut plugin.coverage {
        if coverage.rule_id == RuleId::ManifestJson.as_str() {
            coverage.status = CoverageStatus::Unchecked;
            coverage.reason_code = Some("JSON_REPRESENTATION".into());
        } else {
            coverage.status = CoverageStatus::Blocked;
            coverage.reason_code = Some("JSON_REPRESENTATION".into());
        }
    }
    PluginOutcome {
        plugin,
        errors: vec![tool_error(path, code, message)],
    }
}

pub(crate) fn add_finding(
    plugin: &mut PluginReport,
    rule: RuleId,
    path: String,
    scope: Scope,
    code: &str,
    message: &str,
) {
    let meta = rule.metadata();
    let coverage_target = path.clone();
    plugin.findings.push(Finding {
        rule_id: rule,
        spec: meta.spec.iter().map(|x| (*x).into()).collect(),
        radius: meta.radius,
        normative: meta.normative,
        obligation: meta.obligation,
        subject: meta.subject,
        confidence: meta.confidence,
        path,
        pointer: None,
        line: None,
        column: None,
        scope,
        effect: meta.effect,
        evidence_code: code.into(),
        message: message.into(),
        hint: None,
    });
    // A finding is itself evidence that this exact rule was evaluated.  Keep
    // that evidence next to the finding rather than asking the registry
    // backfill to infer it later.
    plugin.coverage.push(Coverage {
        rule_id: rule.as_str().into(),
        target: coverage_target,
        status: CoverageStatus::Fail,
        reason_code: Some(code.into()),
    });
}

pub(crate) fn add_finding_pointer(
    plugin: &mut PluginReport,
    rule: RuleId,
    path: String,
    scope: Scope,
    pointer: Option<String>,
    code: &str,
    message: &str,
) {
    let meta = rule.metadata();
    let coverage_target = pointer
        .as_deref()
        .map_or_else(|| path.clone(), |pointer| format!("{path}#{pointer}"));
    plugin.findings.push(Finding {
        rule_id: rule,
        spec: meta.spec.iter().map(|x| (*x).into()).collect(),
        radius: meta.radius,
        normative: meta.normative,
        obligation: meta.obligation,
        subject: meta.subject,
        confidence: meta.confidence,
        path,
        pointer,
        line: None,
        column: None,
        scope,
        effect: meta.effect,
        evidence_code: code.into(),
        message: message.into(),
        hint: None,
    });
    plugin.coverage.push(Coverage {
        rule_id: rule.as_str().into(),
        target: coverage_target,
        status: CoverageStatus::Fail,
        reason_code: Some(code.into()),
    });
}

fn scan_components(root: &Path, plugin: &mut PluginReport, errors: &mut Vec<ToolError>) {
    scan_skills(root, plugin, errors);
    crate::mcp::scan(root, plugin, errors);
}

fn scan_skills(root: &Path, plugin: &mut PluginReport, errors: &mut Vec<ToolError>) {
    let skills = root.join("skills");
    match fs::symlink_metadata(&skills) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            plugin.coverage.push(Coverage {
                rule_id: RuleId::SkillConformance.as_str().into(),
                target: "skills".into(),
                status: CoverageStatus::NotApplicable,
                reason_code: Some("SKILLS_ABSENT".into()),
            });
            return;
        }
        Err(_) => {
            error(
                errors,
                &skills,
                "SKILLS_METADATA_IO",
                "无法读取 skills 元数据",
            );
            plugin.coverage.push(Coverage {
                rule_id: RuleId::SkillConformance.as_str().into(),
                target: "skills".into(),
                status: CoverageStatus::Unchecked,
                reason_code: Some("SKILLS_METADATA_IO".into()),
            });
            return;
        }
        Ok(_) => {}
    }
    let skills_actual = match crate::containment::resolve(root, &skills) {
        Ok(crate::containment::Resolution::Inside(p)) => match crate::containment::directory(&p) {
            Ok(true) => {
                plugin.coverage.push(Coverage {
                    rule_id: RuleId::DiscoveryKind.as_str().into(),
                    target: "skills".into(),
                    status: CoverageStatus::Pass,
                    reason_code: Some("SKILLS_DIRECTORY".into()),
                });
                plugin.coverage.push(Coverage {
                    rule_id: RuleId::PathFixedEscape.as_str().into(),
                    target: "skills".into(),
                    status: CoverageStatus::Pass,
                    reason_code: Some("SKILLS_CONTAINED".into()),
                });
                p
            }
            Ok(false) => {
                add_finding(
                    plugin,
                    RuleId::DiscoveryKind,
                    "skills".into(),
                    Scope::ComponentType("skills".into()),
                    "SKILLS_WRONG_KIND",
                    "skills 必须是目录，已禁用 skills 组件",
                );
                return;
            }
            Err(_) => {
                error(
                    errors,
                    &skills,
                    "SKILLS_METADATA_IO",
                    "无法读取 skills 元数据",
                );
                plugin.coverage.push(Coverage {
                    rule_id: RuleId::SkillConformance.as_str().into(),
                    target: "skills".into(),
                    status: CoverageStatus::Unchecked,
                    reason_code: Some("SKILLS_METADATA_IO".into()),
                });
                return;
            }
        },
        Ok(crate::containment::Resolution::Outside) => {
            add_finding(
                plugin,
                RuleId::PathFixedEscape,
                "skills".into(),
                Scope::ComponentType("skills".into()),
                "SKILLS_OUTSIDE_ROOT",
                "skills 位于包根之外，已禁用 skills 组件",
            );
            return;
        }
        Ok(crate::containment::Resolution::Unresolved) => {
            error(
                errors,
                &skills,
                "SKILLS_CANONICALIZE",
                "无法解析 skills 路径",
            );
            return;
        }
        Err(_) => {
            error(
                errors,
                &skills,
                "SKILLS_CANONICALIZE",
                "无法解析 skills 路径",
            );
            return;
        }
    };
    let entries = match fs::read_dir(&skills_actual) {
        Ok(x) => x,
        Err(_) => {
            error(
                errors,
                &skills_actual,
                "SKILLS_READ_IO",
                "无法枚举 skills 目录",
            );
            plugin.coverage.push(Coverage {
                rule_id: RuleId::SkillConformance.as_str().into(),
                target: "skills".into(),
                status: CoverageStatus::Unchecked,
                reason_code: Some("SKILLS_READ_IO".into()),
            });
            return;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                error(
                    errors,
                    &skills_actual,
                    "SKILLS_ENTRY_IO",
                    "无法读取 skills 目录项",
                );
                continue;
            }
        };
        let name = match entry.file_name().into_string() {
            Ok(x) => x,
            Err(_) => {
                error(
                    errors,
                    &entry.path(),
                    "PATH_ENCODING",
                    "路径不能以 UTF-8 表示",
                );
                plugin.coverage.push(Coverage {
                    rule_id: RuleId::SkillConformance.as_str().into(),
                    target: "skills".into(),
                    status: CoverageStatus::Unchecked,
                    reason_code: Some("PATH_ENCODING".into()),
                });
                continue;
            }
        };
        let candidate = entry.path();
        let logical = format!("skills/{name}");
        let directory = match crate::containment::resolve(root, &candidate) {
            Ok(crate::containment::Resolution::Inside(p)) => {
                match crate::containment::directory(&p) {
                    Ok(true) => p,
                    Ok(false) => continue,
                    Err(_) => {
                        error(
                            errors,
                            &candidate,
                            "SKILL_DIRECTORY_METADATA_IO",
                            "无法读取 skill 目录元数据",
                        );
                        continue;
                    }
                }
            }
            Ok(crate::containment::Resolution::Outside) => {
                // only a skill if exact SKILL.md metadata says so
                match external_directory_has_skill(&candidate) {
                    Ok(true) => {
                        let target = format!("{logical}/SKILL.md");
                        add_finding(
                            plugin,
                            RuleId::PathSkillEscape,
                            logical,
                            Scope::Skill(name),
                            "SKILL_OUTSIDE_ROOT",
                            "skill 位于包根之外，已跳过该 skill",
                        );
                        plugin.coverage.push(Coverage {
                            rule_id: RuleId::SkillConformance.as_str().into(),
                            target,
                            status: CoverageStatus::Blocked,
                            reason_code: Some("SKILL_OUTSIDE_ROOT".into()),
                        });
                    }
                    Ok(false) => {
                        add_finding(
                            plugin,
                            RuleId::PathResourceEscape,
                            logical.clone(),
                            Scope::Path(logical.clone()),
                            "RESOURCE_OUTSIDE_ROOT",
                            "资源路径位于包根之外，访问时会被拒绝",
                        );
                    }
                    Err(_) => error(
                        errors,
                        &candidate.join("SKILL.md"),
                        "SKILL_METADATA_IO",
                        "无法读取外部 SKILL.md 元数据",
                    ),
                }
                continue;
            }
            Ok(crate::containment::Resolution::Unresolved) => continue,
            Err(_) => {
                error(
                    errors,
                    &candidate,
                    "SKILL_CANONICALIZE",
                    "无法解析 skill 路径",
                );
                continue;
            }
        };
        // Enumerate the contained candidate only, and accept the exact ASCII
        // entry name. This is discovery, not a recursive skill search.
        let md = match internal_skill_entry(root, &candidate) {
            Ok(Some(path)) => path,
            Ok(None) => continue,
            Err(_) => {
                error(
                    errors,
                    &candidate,
                    "SKILL_ENUMERATE_IO",
                    "无法枚举 skill 目录",
                );
                continue;
            }
        };
        let md_logical = format!("{logical}/SKILL.md");
        match crate::containment::resolve(root, &md) {
            Ok(crate::containment::Resolution::Inside(actual)) => {
                match crate::containment::regular(&actual) {
                    Ok(true) => match crate::containment::read_safe(root, &md) {
                        Ok(source) => {
                            plugin.coverage.push(Coverage {
                                rule_id: RuleId::PathSkillEscape.as_str().into(),
                                target: md_logical.clone(),
                                status: CoverageStatus::Pass,
                                reason_code: Some("SKILL_CONTAINED".into()),
                            });
                            crate::skills::validate(
                                &source,
                                &name,
                                md_logical.clone(),
                                &mut plugin.findings,
                                &mut plugin.coverage,
                            );
                            audit_resources(root, &directory, &logical, plugin, errors);
                        }
                        Err(crate::containment::ReadError::InputChanged) => {
                            error(errors, &md, "INPUT_CHANGED", "读取 SKILL.md 时输入发生变化");
                            plugin.coverage.push(Coverage {
                                rule_id: RuleId::SkillConformance.as_str().into(),
                                target: md_logical,
                                status: CoverageStatus::Unchecked,
                                reason_code: Some("INPUT_CHANGED".into()),
                            });
                        }
                        Err(crate::containment::ReadError::Unsafe) => {
                            error(errors, &md, "SKILL_READ_IO", "无法安全读取 SKILL.md 内容");
                            plugin.coverage.push(Coverage {
                                rule_id: RuleId::SkillConformance.as_str().into(),
                                target: md_logical,
                                status: CoverageStatus::Unchecked,
                                reason_code: Some("SKILL_READ_IO".into()),
                            });
                        }
                        Err(crate::containment::ReadError::Unsupported) => {
                            error(
                                errors,
                                &md,
                                "SAFE_READ_UNSUPPORTED",
                                "当前平台无法安全读取 SKILL.md 内容",
                            );
                            plugin.coverage.push(Coverage {
                                rule_id: RuleId::SkillConformance.as_str().into(),
                                target: md_logical.clone(),
                                status: CoverageStatus::Unchecked,
                                reason_code: Some("SAFE_READ_UNSUPPORTED".into()),
                            });
                            block_unread_skill(plugin, &md_logical);
                        }
                        Err(read_error @ crate::containment::ReadError::Io(_)) => {
                            let _ = read_error.io_kind();
                            error(errors, &md, "SKILL_READ_IO", "无法安全读取 SKILL.md 内容");
                            plugin.coverage.push(Coverage {
                                rule_id: RuleId::SkillConformance.as_str().into(),
                                target: md_logical,
                                status: CoverageStatus::Unchecked,
                                reason_code: Some("SKILL_READ_IO".into()),
                            });
                        }
                    },
                    Ok(false) => {}
                    Err(_) => error(errors, &md, "SKILL_METADATA_IO", "无法读取 SKILL.md 元数据"),
                }
            }
            Ok(crate::containment::Resolution::Outside) => match external_file_is_skill(&md) {
                Ok(true) => {
                    add_finding(
                        plugin,
                        RuleId::PathSkillEscape,
                        md_logical.clone(),
                        Scope::Skill(name),
                        "SKILL_MD_OUTSIDE_ROOT",
                        "SKILL.md 位于包根之外，已跳过该 skill",
                    );
                    plugin.coverage.push(Coverage {
                        rule_id: RuleId::SkillConformance.as_str().into(),
                        target: md_logical,
                        status: CoverageStatus::Blocked,
                        reason_code: Some("SKILL_MD_OUTSIDE_ROOT".into()),
                    });
                }
                Ok(false) => {
                    add_finding(
                        plugin,
                        RuleId::PathResourceEscape,
                        md_logical.clone(),
                        Scope::Path(md_logical),
                        "RESOURCE_OUTSIDE_ROOT",
                        "资源路径位于包根之外，访问时会被拒绝",
                    );
                }
                Err(_) => error(
                    errors,
                    &md,
                    "SKILL_METADATA_IO",
                    "无法读取外部 SKILL.md 元数据",
                ),
            },
            Ok(crate::containment::Resolution::Unresolved) => add_finding(
                plugin,
                RuleId::AdviceUnresolvedPath,
                md_logical,
                Scope::Path(logical.clone()),
                "SKILL_MD_UNRESOLVED",
                "SKILL.md 无法解析，未检查该路径",
            ),
            Err(_) => error(errors, &md, "SKILL_CANONICALIZE", "无法解析 SKILL.md 路径"),
        }
    }
}

fn audit_resources(
    root: &Path,
    dir: &Path,
    logical: &str,
    plugin: &mut PluginReport,
    errors: &mut Vec<ToolError>,
) {
    let mut seen = BTreeSet::new();
    audit_dir(root, dir, logical, plugin, &mut seen, errors);
}
fn internal_skill_entry(root: &Path, directory: &Path) -> std::io::Result<Option<PathBuf>> {
    let actual = fs::canonicalize(directory)?;
    if !actual.starts_with(root) {
        return Err(std::io::Error::other("candidate left plugin root"));
    }
    for entry in fs::read_dir(actual)? {
        let entry = entry?;
        if entry.file_name() == "SKILL.md" {
            // Keep the original entry for read_safe's second path check.
            return Ok(Some(directory.join("SKILL.md")));
        }
    }
    Ok(None)
}
/// Outside candidates are never enumerated. Only the required exact entry is
/// queried, and an absent entry means "not a skill", not an operational error.
fn external_directory_has_skill(directory: &Path) -> std::io::Result<bool> {
    external_file_is_skill(&directory.join("SKILL.md"))
}
fn external_file_is_skill(file: &Path) -> std::io::Result<bool> {
    match fs::canonicalize(file) {
        Ok(path) => Ok(fs::metadata(path)?.is_file()),
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
            ) || cfg!(unix) && error.raw_os_error() == Some(libc::ELOOP) =>
        {
            Ok(false)
        }
        Err(error) => Err(error),
    }
}
fn audit_dir(
    root: &Path,
    dir: &Path,
    logical: &str,
    plugin: &mut PluginReport,
    seen: &mut BTreeSet<PathBuf>,
    errors: &mut Vec<ToolError>,
) {
    let actual = match fs::canonicalize(dir) {
        Ok(x) => x,
        Err(_) => {
            error(errors, dir, "RESOURCE_CANONICALIZE", "无法解析资源目录");
            return;
        }
    };
    // Recheck after canonicalization and before enumeration. An escaped
    // directory is reported by its entry and must never be traversed.
    if !actual.starts_with(root) {
        error(
            errors,
            dir,
            "RESOURCE_OUTSIDE_ROOT",
            "资源目录位于包根之外，未枚举其内容",
        );
        return;
    }
    if !seen.insert(actual) {
        return;
    }
    let entries = match fs::read_dir(dir) {
        Ok(x) => x,
        Err(_) => {
            error(errors, dir, "RESOURCE_READ_IO", "无法枚举资源目录");
            return;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(x) => x,
            Err(_) => {
                error(errors, dir, "RESOURCE_ENTRY_IO", "无法读取资源目录项");
                continue;
            }
        };
        let path = entry.path();
        let name = match entry.file_name().into_string() {
            Ok(x) => x,
            Err(_) => {
                error(errors, &path, "PATH_ENCODING", "路径不能以 UTF-8 表示");
                continue;
            }
        };
        let lp = format!("{logical}/{name}");
        match crate::containment::resolve(root, &path) {
            Ok(crate::containment::Resolution::Outside) => add_finding(
                plugin,
                RuleId::PathResourceEscape,
                lp.clone(),
                Scope::Path(lp.clone()),
                "RESOURCE_OUTSIDE_ROOT",
                "资源路径位于包根之外，访问时会被拒绝",
            ),
            Ok(crate::containment::Resolution::Inside(real)) => {
                plugin.coverage.push(Coverage {
                    rule_id: RuleId::PathResourceEscape.as_str().into(),
                    target: lp.clone(),
                    status: CoverageStatus::Pass,
                    reason_code: Some("RESOURCE_CONTAINED".into()),
                });
                match fs::metadata(&real) {
                    Ok(metadata) if metadata.is_dir() => {
                        audit_dir(root, &path, &lp, plugin, seen, errors)
                    }
                    Ok(_) => {}
                    Err(_) => error(errors, &path, "RESOURCE_METADATA_IO", "无法读取资源元数据"),
                }
            }
            Ok(crate::containment::Resolution::Unresolved) => add_finding(
                plugin,
                RuleId::AdviceUnresolvedPath,
                lp.clone(),
                Scope::Path(lp),
                "RESOURCE_UNRESOLVED",
                "资源路径无法解析，未检查该路径",
            ),
            Err(_) => error(errors, &path, "RESOURCE_CANONICALIZE", "无法解析资源路径"),
        }
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
        line: None,
        column: None,
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
        RuleId::ExtensionNamespace,
        RuleId::ExtensionUnknown,
        RuleId::ExtensionFileLocation,
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
        errors: vec![],
    }
}
fn outcome_error(root: String, path: &Path, code: &str, message: &str) -> PluginOutcome {
    let plugin = unread_plugin(root);
    PluginOutcome {
        plugin,
        errors: vec![tool_error(path, code, message)],
    }
}
fn outcome_safe_read_unsupported(root: String, path: &Path) -> PluginOutcome {
    let mut plugin = unread_plugin(root);
    for coverage in &mut plugin.coverage {
        coverage.status = if coverage.rule_id == RuleId::ManifestLocation.as_str() {
            CoverageStatus::Unchecked
        } else {
            CoverageStatus::Blocked
        };
        coverage.reason_code = Some("SAFE_READ_UNSUPPORTED".into());
    }
    PluginOutcome {
        plugin,
        errors: vec![tool_error(
            path,
            "SAFE_READ_UNSUPPORTED",
            "当前平台无法安全读取 plugin.json 内容",
        )],
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
    error.kind() == std::io::ErrorKind::NotFound
        || (cfg!(unix) && error.raw_os_error() == Some(libc::ELOOP))
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
        ensure_registry_coverage(plugin);
        normalize_coverage(&mut plugin.coverage);
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
            strict_violation |=
                finding.radius == Radius::Advisory && finding.subject == Subject::Package;
        }
        strict_violation |= plugin.coverage.iter().any(|coverage| {
            crate::rules::registry_subject(&coverage.rule_id) == Subject::Package
                && coverage.status == CoverageStatus::Unchecked
                // The registry is an audit index, not fabricated evidence of
                // a concrete target. Strict only promotes unchecked results
                // produced by an actual target-level scanner branch.
                && coverage.reason_code.as_deref() != Some("RULE_NOT_EVALUATED")
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

/// Fill the original 91-row table after concrete scanners have recorded their
/// branch-level result.  An index entry with no target-level evidence is not
/// evidence that the target is absent, so package rules remain unchecked.
/// Concrete scanners alone may record `not-applicable`.
fn ensure_registry_coverage(plugin: &mut PluginReport) {
    let blocked_by_parent = plugin.status != "accepted";
    for rule in crate::rules::rule_registry() {
        if plugin
            .coverage
            .iter()
            .any(|coverage| coverage.rule_id == rule.id)
        {
            continue;
        }
        let finding = plugin
            .findings
            .iter()
            .find(|finding| finding.rule_id.as_str() == rule.id);
        let (status, reason) = if finding.is_some() {
            (CoverageStatus::Fail, "FINDING_RECORDED")
        } else if blocked_by_parent {
            (CoverageStatus::Blocked, "MANIFEST_REJECTED")
        } else {
            match rule.subject {
                Subject::Client | Subject::Publisher => {
                    (CoverageStatus::Manual, "CLIENT_OR_PUBLISHER_MANUAL")
                }
                Subject::Package => (CoverageStatus::Unchecked, "RULE_NOT_EVALUATED"),
                Subject::Tool => (CoverageStatus::Unchecked, "TOOL_POLICY_UNAVAILABLE"),
            }
        };
        plugin.coverage.push(Coverage {
            rule_id: rule.id.into(),
            target: "plugin".into(),
            status,
            reason_code: Some(reason.into()),
        });
    }
}

/// Coalesce duplicate reports of the *same* branch only when they agree.  A
/// conflicting state is intentionally retained: it is an engine bug that the
/// coverage contract tests must expose, not a precedence decision that may be
/// hidden at serialization time.
fn normalize_coverage(coverage: &mut Vec<Coverage>) {
    let mut normalized = std::collections::BTreeMap::<(String, String), Coverage>::new();
    let mut conflicts = Vec::new();
    for item in coverage.drain(..) {
        let key = (item.rule_id.clone(), item.target.clone());
        match normalized.get(&key) {
            Some(existing) if existing.status == item.status => {}
            Some(_) => conflicts.push(item),
            _ => {
                normalized.insert(key, item);
            }
        }
    }
    coverage.extend(normalized.into_values());
    coverage.extend(conflicts);
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
pub(crate) fn error(errors: &mut Vec<ToolError>, path: &Path, code: &str, message: &str) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    struct UnsupportedGuard;
    impl UnsupportedGuard {
        fn enable(path: PathBuf) -> Self {
            crate::containment::set_test_force_unsupported(Some(path));
            Self
        }
    }
    impl Drop for UnsupportedGuard {
        fn drop(&mut self) {
            crate::containment::set_test_force_unsupported(None);
        }
    }
    fn has_unchecked(coverage: &[Coverage], rule: RuleId, target: &str) -> bool {
        coverage.iter().any(|coverage| {
            coverage.rule_id == rule.as_str()
                && coverage.target == target
                && coverage.status == CoverageStatus::Unchecked
                && coverage.reason_code.as_deref() == Some("SAFE_READ_UNSUPPORTED")
        })
    }
    fn has_coverage(
        coverage: &[Coverage],
        rule: RuleId,
        target: &str,
        status: CoverageStatus,
    ) -> bool {
        coverage.iter().any(|coverage| {
            coverage.rule_id == rule.as_str()
                && coverage.target == target
                && coverage.status == status
                && coverage.reason_code.as_deref() == Some("SAFE_READ_UNSUPPORTED")
        })
    }
    fn assert_final(report: &Report, rule: RuleId, target: &str, blocked_rule: RuleId) {
        assert_eq!(report.exit_code, 2);
        assert!(!report.complete);
        assert!(
            report
                .errors
                .iter()
                .any(|error| error.code == "SAFE_READ_UNSUPPORTED")
        );
        assert!(report.plugins.iter().all(|plugin| {
            plugin
                .findings
                .iter()
                .all(|finding| !(finding.normative && finding.obligation == Obligation::Must))
        }));
        assert!(
            report
                .plugins
                .iter()
                .any(|plugin| has_unchecked(&plugin.coverage, rule, target))
        );
        assert!(report.plugins.iter().any(|plugin| {
            has_coverage(
                &plugin.coverage,
                blocked_rule,
                target,
                CoverageStatus::Blocked,
            )
        }));
        assert!(
            report
                .plugins
                .iter()
                .filter(|plugin| has_unchecked(&plugin.coverage, rule, target))
                .all(|plugin| !plugin.coverage.iter().any(|coverage| {
                    coverage.rule_id == rule.as_str()
                        && coverage.target == target
                        && coverage.status == CoverageStatus::Pass
                }))
        );
        let json = serde_json::to_value(report).unwrap();
        assert_eq!(json["exitCode"], 2);
        assert_eq!(json["complete"], false);
        assert!(
            json["errors"]
                .as_array()
                .unwrap()
                .iter()
                .any(|error| error["code"] == "SAFE_READ_UNSUPPORTED")
        );
        assert!(json["plugins"].as_array().unwrap().iter().any(|plugin| {
            plugin["coverage"]
                .as_array()
                .unwrap()
                .iter()
                .any(|coverage| {
                    coverage["ruleId"] == rule.as_str()
                        && coverage["target"] == target
                        && coverage["status"] == "unchecked"
                        && coverage["reasonCode"] == "SAFE_READ_UNSUPPORTED"
                })
        }));
        assert!(json["plugins"].as_array().unwrap().iter().any(|plugin| {
            plugin["coverage"]
                .as_array()
                .unwrap()
                .iter()
                .any(|coverage| {
                    coverage["ruleId"] == blocked_rule.as_str()
                        && coverage["target"] == target
                        && coverage["status"] == "blocked"
                        && coverage["reasonCode"] == "SAFE_READ_UNSUPPORTED"
                })
        }));
    }

    #[test]
    fn safe_read_unsupported_manifest_blocks_only_its_plugin() {
        let temp = TempDir::new().unwrap();
        let bad = temp.path().join("bad");
        let good = temp.path().join("good");
        fs::create_dir(&bad).unwrap();
        fs::create_dir(&good).unwrap();
        fs::write(
            bad.join("plugin.json"),
            "{ manifest sentinel is invalid JSON",
        )
        .unwrap();
        fs::write(
            good.join("plugin.json"),
            r#"{"$schema":"https://agent-plugins.org/schemas/1.0.0/plugin.schema.json","name":"good"}"#,
        )
        .unwrap();
        let _guard = UnsupportedGuard::enable(bad.join("plugin.json"));
        let report = lint_path(
            temp.path(),
            LintOptions {
                mode: InputMode::Collection,
                strict: false,
            },
        );
        assert_final(
            &report,
            RuleId::ManifestLocation,
            "plugin.json",
            RuleId::ManifestJson,
        );
        let good_plugin = report
            .plugins
            .iter()
            .find(|plugin| plugin.root == "good")
            .unwrap();
        assert!(good_plugin.coverage.iter().any(|coverage| {
            coverage.rule_id == RuleId::ManifestJson.as_str()
                && coverage.status == CoverageStatus::Pass
        }));
    }

    #[test]
    fn safe_read_unsupported_skill_blocks_its_fields_but_not_mcp() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("plugin.json"),
            r#"{"$schema":"https://agent-plugins.org/schemas/1.0.0/plugin.schema.json","name":"a"}"#,
        )
        .unwrap();
        let skill = temp.path().join("skills/a");
        fs::create_dir_all(&skill).unwrap();
        fs::write(
            skill.join("SKILL.md"),
            "---\nname: [invalid YAML sentinel\n---\n",
        )
        .unwrap();
        fs::write(
            temp.path().join("mcp.json"),
            r#"{"$schema":"https://agent-plugins.org/schemas/1.0.0/mcp.schema.json","mcpServers":{}}"#,
        )
        .unwrap();
        let _guard = UnsupportedGuard::enable(skill.join("SKILL.md"));
        let report = lint_path(temp.path(), LintOptions::default());
        assert_final(
            &report,
            RuleId::SkillConformance,
            "skills/a/SKILL.md",
            RuleId::AsFrontmatter,
        );
        assert!(report.plugins[0].coverage.iter().any(|coverage| {
            coverage.rule_id == RuleId::McpEnvelope.as_str()
                && coverage.status == CoverageStatus::Pass
        }));
    }

    #[test]
    fn safe_read_unsupported_mcp_blocks_its_envelope_but_not_skill() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("plugin.json"),
            r#"{"$schema":"https://agent-plugins.org/schemas/1.0.0/plugin.schema.json","name":"a"}"#,
        )
        .unwrap();
        let skill = temp.path().join("skills/a");
        fs::create_dir_all(&skill).unwrap();
        fs::write(
            skill.join("SKILL.md"),
            "---\nname: a\ndescription: valid\n---\nbody\n",
        )
        .unwrap();
        fs::write(
            temp.path().join("mcp.json"),
            "{ mcp sentinel is invalid JSON",
        )
        .unwrap();
        let _guard = UnsupportedGuard::enable(temp.path().join("mcp.json"));
        let report = lint_path(temp.path(), LintOptions::default());
        assert_final(
            &report,
            RuleId::McpEnvelope,
            "mcp.json",
            RuleId::McpServerVariant,
        );
        assert!(report.plugins[0].coverage.iter().any(|coverage| {
            coverage.rule_id == RuleId::AsFrontmatter.as_str()
                && coverage.target == "skills/a/SKILL.md"
                && coverage.status == CoverageStatus::Pass
        }));
    }
}
