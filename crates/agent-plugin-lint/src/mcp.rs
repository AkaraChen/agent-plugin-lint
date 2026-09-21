use crate::containment::{self, ReadError, Resolution};
use crate::expansion;
use crate::json::{self, Parsed};
use crate::lint::{add_finding, add_finding_pointer, error};
use crate::{Coverage, CoverageStatus, PluginReport, RuleId, Scope, ToolError};
use http::{HeaderName, HeaderValue};
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::net::IpAddr;
use std::path::Path;
use url::Url;

const MCP_SCHEMA: &str = "https://agent-plugins.org/schemas/1.0.0/mcp.schema.json";

pub(crate) fn scan(root: &Path, plugin: &mut PluginReport, errors: &mut Vec<ToolError>) {
    let path = root.join("mcp.json");
    match std::fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => coverage(
            plugin,
            RuleId::McpEnvelope,
            "mcp.json",
            CoverageStatus::NotApplicable,
            "MCP_ABSENT",
        ),
        Err(_) => {
            error(
                errors,
                &path,
                "MCP_METADATA_IO",
                "cannot read mcp.json metadata",
            );
            coverage(
                plugin,
                RuleId::McpEnvelope,
                "mcp.json",
                CoverageStatus::Unchecked,
                "MCP_METADATA_IO",
            );
        }
        Ok(_) => match containment::resolve(root, &path) {
            Ok(Resolution::Outside) => add_finding(
                plugin,
                RuleId::PathFixedEscape,
                "mcp.json".into(),
                Scope::ComponentType("mcp".into()),
                "MCP_OUTSIDE_ROOT",
                "mcp.json is outside the package root; MCP is disabled",
            ),
            Ok(Resolution::Unresolved) => add_finding(
                plugin,
                RuleId::DiscoveryKind,
                "mcp.json".into(),
                Scope::ComponentType("mcp".into()),
                "MCP_UNRESOLVED",
                "mcp.json does not resolve to a regular file; MCP is disabled",
            ),
            Err(_) => error(errors, &path, "MCP_CANONICALIZE", "cannot resolve mcp.json"),
            Ok(Resolution::Inside(actual)) => match containment::regular(&actual) {
                Ok(false) => add_finding(
                    plugin,
                    RuleId::DiscoveryKind,
                    "mcp.json".into(),
                    Scope::ComponentType("mcp".into()),
                    "MCP_WRONG_KIND",
                    "mcp.json must be a regular file; MCP is disabled",
                ),
                Err(_) => {
                    error(
                        errors,
                        &path,
                        "MCP_METADATA_IO",
                        "cannot read mcp.json metadata",
                    );
                    coverage(
                        plugin,
                        RuleId::McpEnvelope,
                        "mcp.json",
                        CoverageStatus::Unchecked,
                        "MCP_METADATA_IO",
                    );
                }
                Ok(true) => {
                    // These are fixed-location checks.  A valid file remains
                    // contained even if its JSON envelope is later rejected.
                    coverage(
                        plugin,
                        RuleId::DiscoveryKind,
                        "mcp.json",
                        CoverageStatus::Pass,
                        "MCP_REGULAR_FILE",
                    );
                    coverage(
                        plugin,
                        RuleId::PathFixedEscape,
                        "mcp.json",
                        CoverageStatus::Pass,
                        "MCP_FIXED_FILE_CONTAINED",
                    );
                    match containment::read_safe(root, &path) {
                        Ok(source) => match json::parse(&source) {
                            Parsed::Value {
                                value,
                                duplicate_keys,
                            } => {
                                duplicate_coverage(plugin, duplicate_keys);
                                envelope(root, plugin, errors, value)
                            }
                            Parsed::Syntax => disable(
                                plugin,
                                RuleId::McpEnvelope,
                                None,
                                "MCP_JSON",
                                "mcp.json is not valid JSON; MCP is disabled",
                            ),
                            Parsed::Representation => {
                                error(
                                    errors,
                                    &path,
                                    "MCP_JSON_REPRESENTATION",
                                    "mcp.json exceeds this parser's representation limits (numeric range or nesting depth)",
                                );
                                representation_block(plugin);
                            }
                        },
                        Err(ReadError::InputChanged) => {
                            error(
                                errors,
                                &path,
                                "INPUT_CHANGED",
                                "mcp.json changed while it was being read",
                            );
                            coverage(
                                plugin,
                                RuleId::McpEnvelope,
                                "mcp.json",
                                CoverageStatus::Unchecked,
                                "INPUT_CHANGED",
                            );
                        }
                        Err(ReadError::Unsafe) => {
                            error(
                                errors,
                                &path,
                                "MCP_READ",
                                "mcp.json is not a regular file that can be read safely",
                            );
                            coverage(
                                plugin,
                                RuleId::McpEnvelope,
                                "mcp.json",
                                CoverageStatus::Unchecked,
                                "MCP_READ",
                            );
                        }
                        Err(ReadError::Unsupported) => {
                            error(
                                errors,
                                &path,
                                "SAFE_READ_UNSUPPORTED",
                                "this platform cannot safely read mcp.json",
                            );
                            safe_read_unsupported_block(plugin);
                        }
                        Err(ReadError::Io(_)) => {
                            error(errors, &path, "MCP_READ", "cannot read mcp.json");
                            coverage(
                                plugin,
                                RuleId::McpEnvelope,
                                "mcp.json",
                                CoverageStatus::Unchecked,
                                "MCP_READ",
                            );
                        }
                    }
                }
            },
        },
    }
}

fn representation_block(plugin: &mut PluginReport) {
    blocked_unread_envelope(plugin, "JSON_REPRESENTATION");
}

fn safe_read_unsupported_block(plugin: &mut PluginReport) {
    blocked_unread_envelope(plugin, "SAFE_READ_UNSUPPORTED");
}

fn blocked_unread_envelope(plugin: &mut PluginReport, reason: &str) {
    coverage(
        plugin,
        RuleId::McpEnvelope,
        "mcp.json",
        CoverageStatus::Unchecked,
        reason,
    );
    coverage(
        plugin,
        RuleId::AdviceDuplicateJsonKey,
        "mcp.json",
        CoverageStatus::Unchecked,
        reason,
    );
    for rule in [
        RuleId::McpSchemaId,
        RuleId::McpVersionMatch,
        RuleId::McpServerVariant,
        RuleId::McpCommand,
        RuleId::McpCwdForm,
        RuleId::PathServerEscape,
        RuleId::McpUrl,
        RuleId::McpHttps,
        RuleId::McpHeaders,
        RuleId::McpReservedEnv,
    ] {
        coverage(plugin, rule, "mcp.json", CoverageStatus::Blocked, reason);
    }
}

fn duplicate_coverage(plugin: &mut PluginReport, duplicate_keys: bool) {
    coverage(
        plugin,
        RuleId::AdviceDuplicateJsonKey,
        "mcp.json",
        if duplicate_keys {
            CoverageStatus::Fail
        } else {
            CoverageStatus::Pass
        },
        if duplicate_keys {
            "DUPLICATE_JSON_KEY"
        } else {
            "NO_DUPLICATE_JSON_KEY"
        },
    );
    if duplicate_keys {
        add_finding(
            plugin,
            RuleId::AdviceDuplicateJsonKey,
            "mcp.json".into(),
            Scope::ComponentType("mcp".into()),
            "DUPLICATE_JSON_KEY",
            "JSON object has duplicate keys; later checks use the last value",
        );
    }
}

fn envelope(root: &Path, plugin: &mut PluginReport, errors: &mut Vec<ToolError>, value: Value) {
    let Some(object) = value.as_object() else {
        return disable(
            plugin,
            RuleId::McpEnvelope,
            None,
            "MCP_NOT_OBJECT",
            "mcp.json must be a top-level object; MCP is disabled",
        );
    };
    if object.len() != 2
        || !object.contains_key("$schema")
        || !object.contains_key("mcpServers")
        || !object["mcpServers"].is_object()
        || !object["$schema"].is_string()
    {
        return disable(
            plugin,
            RuleId::McpEnvelope,
            None,
            "MCP_ENVELOPE",
            "mcp.json top-level fields do not match the specification; MCP is disabled",
        );
    }
    let schema = object["$schema"].as_str().expect("string checked");
    if schema != MCP_SCHEMA {
        let rule = if recognized_mcp_schema(schema) {
            RuleId::McpVersionMatch
        } else {
            RuleId::McpSchemaId
        };
        return disable(
            plugin,
            rule,
            Some("/$schema"),
            if rule == RuleId::McpSchemaId {
                "MCP_SCHEMA_ID"
            } else {
                "MCP_VERSION_MISMATCH"
            },
            if rule == RuleId::McpSchemaId {
                "mcp.json schema identifier is unsupported; MCP is disabled"
            } else {
                "mcp.json specification version is unsupported or does not match the manifest; MCP is disabled"
            },
        );
    }
    if plugin.declared_spec.as_deref() != Some("1.0.0") {
        return disable(
            plugin,
            RuleId::McpVersionMatch,
            Some("/$schema"),
            "MCP_VERSION_MISMATCH",
            "mcp.json specification version does not match the manifest; MCP is disabled",
        );
    }
    coverage(
        plugin,
        RuleId::McpEnvelope,
        "mcp.json",
        CoverageStatus::Pass,
        "MCP_ENVELOPE_VALID",
    );
    coverage(
        plugin,
        RuleId::McpSchemaId,
        "mcp.json#/$schema",
        CoverageStatus::Pass,
        "MCP_SCHEMA_SUPPORTED",
    );
    coverage(
        plugin,
        RuleId::McpVersionMatch,
        "mcp.json#/$schema",
        CoverageStatus::Pass,
        "MCP_VERSION_MATCH",
    );
    for (name, entry) in object["mcpServers"].as_object().expect("object checked") {
        server(root, plugin, errors, name, entry);
    }
}

fn server(
    root: &Path,
    plugin: &mut PluginReport,
    errors: &mut Vec<ToolError>,
    name: &str,
    entry: &Value,
) {
    let pointer = format!("/mcpServers/{}", escape(name));
    let Some(object) = entry.as_object() else {
        return skip(
            plugin,
            RuleId::McpServerVariant,
            name,
            Some(&pointer),
            "MCP_SERVER_NOT_OBJECT",
            "MCP server configuration must be an object",
        );
    };
    let Some(kind) = object.get("type").and_then(Value::as_str) else {
        return skip(
            plugin,
            RuleId::McpServerVariant,
            name,
            Some(&pointer),
            "MCP_SERVER_TYPE",
            "MCP server is missing a valid type",
        );
    };
    match kind {
        "stdio" => stdio(root, plugin, errors, name, &pointer, object),
        "streamable-http" | "sse" => remote(plugin, name, &pointer, object),
        _ => skip(
            plugin,
            RuleId::McpServerVariant,
            name,
            Some(&pointer),
            "MCP_SERVER_TYPE",
            "MCP server type is unsupported",
        ),
    }
}

fn stdio(
    root: &Path,
    plugin: &mut PluginReport,
    errors: &mut Vec<ToolError>,
    name: &str,
    pointer: &str,
    o: &Map<String, Value>,
) {
    if !closed(o, &["type", "command", "args", "env", "cwd"])
        || !o.get("command").is_some_and(Value::is_string)
        || !strings(o.get("args"))
        || !string_map(o.get("env"))
        || !o.get("cwd").is_none_or(Value::is_string)
    {
        return skip(
            plugin,
            RuleId::McpServerVariant,
            name,
            Some(pointer),
            "MCP_STDIO_VARIANT",
            "stdio server fields do not match the closed variant",
        );
    }
    coverage(
        plugin,
        RuleId::McpServerVariant,
        &format!("mcp.json#{pointer}"),
        CoverageStatus::Pass,
        "STDIO_VARIANT_VALID",
    );
    let command = o["command"].as_str().expect("checked");
    let command_target = target(pointer, "command");
    if command.is_empty()
        || command.starts_with('/')
        || command.starts_with("../")
        || (!command.starts_with("./") && command.contains('/'))
    {
        coverage_id(
            plugin,
            "AP-PATH-RELATIVE-FORM",
            &command_target,
            CoverageStatus::Fail,
            "COMMAND_RELATIVE_FORM",
        );
        return skip(
            plugin,
            RuleId::McpCommand,
            name,
            Some(&format!("{pointer}/command")),
            "MCP_COMMAND_PATH",
            "command must not be an absolute path or a parent-directory path",
        );
    }
    if command.starts_with("./") {
        coverage_id(
            plugin,
            "AP-PATH-RELATIVE-FORM",
            &command_target,
            CoverageStatus::Pass,
            "COMMAND_DOT_SLASH_FORM",
        );
        match inside(
            root,
            root.join(command),
            plugin,
            errors,
            name,
            pointer,
            "command",
        ) {
            PathCheck::Outside => return,
            PathCheck::Io => return,
            PathCheck::Unresolved => {}
            PathCheck::Inside => coverage(
                plugin,
                RuleId::McpCommand,
                &command_target,
                CoverageStatus::Pass,
                "COMMAND_CONTAINED",
            ),
        }
    } else if !safe_bare_command(command) {
        add_finding_pointer(
            plugin,
            RuleId::AdviceAmbiguousCommand,
            "mcp.json".into(),
            Scope::Server(name.into()),
            Some(format!("{pointer}/command")),
            "AMBIGUOUS_COMMAND",
            "a bare command's single-token meaning cannot be determined statically",
        );
        coverage(
            plugin,
            RuleId::McpCommand,
            &command_target,
            CoverageStatus::Unchecked,
            "AMBIGUOUS_COMMAND",
        );
    } else {
        coverage_id(
            plugin,
            "AP-PATH-RELATIVE-FORM",
            &command_target,
            CoverageStatus::NotApplicable,
            "COMMAND_BARE_NAME",
        );
        coverage(
            plugin,
            RuleId::McpCommand,
            &command_target,
            CoverageStatus::Pass,
            "COMMAND_BARE",
        );
    }
    if let Some(env) = o.get("env").and_then(Value::as_object)
        && !env_checks(plugin, name, pointer, env)
    {
        return;
    }
    if o.get("env").is_none() {
        coverage(
            plugin,
            RuleId::McpReservedEnv,
            &target(pointer, "env"),
            CoverageStatus::NotApplicable,
            "ENV_ABSENT",
        );
    } else {
        coverage(
            plugin,
            RuleId::McpReservedEnv,
            &target(pointer, "env"),
            CoverageStatus::Pass,
            "ENV_RESERVED_VALID",
        );
    }
    if let Some(cwd) = o.get("cwd").and_then(Value::as_str) {
        cwd_check(root, plugin, errors, name, pointer, cwd);
    } else {
        coverage_id(
            plugin,
            "AP-PATH-RELATIVE-FORM",
            &target(pointer, "cwd"),
            CoverageStatus::NotApplicable,
            "CWD_DEFAULT_ROOT",
        );
        coverage(
            plugin,
            RuleId::McpCwdForm,
            &target(pointer, "cwd"),
            CoverageStatus::Pass,
            "CWD_DEFAULT_ROOT",
        );
    }
}

fn remote(plugin: &mut PluginReport, name: &str, pointer: &str, o: &Map<String, Value>) {
    if !closed(o, &["type", "url", "headers"])
        || !o.get("url").is_some_and(Value::is_string)
        || !string_map(o.get("headers"))
    {
        return skip(
            plugin,
            RuleId::McpServerVariant,
            name,
            Some(pointer),
            "MCP_REMOTE_VARIANT",
            "remote MCP server fields do not match the closed variant",
        );
    }
    coverage(
        plugin,
        RuleId::McpServerVariant,
        &format!("mcp.json#{pointer}"),
        CoverageStatus::Pass,
        "REMOTE_VARIANT_VALID",
    );
    let url = o["url"].as_str().expect("checked");
    match valid_url(url) {
        UrlCheck::Invalid => {
            return skip(
                plugin,
                RuleId::McpUrl,
                name,
                Some(&format!("{pointer}/url")),
                "MCP_URL",
                "MCP URL must be an absolute HTTP(S) URL without userinfo or a fragment",
            );
        }
        UrlCheck::Ambiguous => {
            coverage(
                plugin,
                RuleId::McpUrl,
                &target(pointer, "url"),
                CoverageStatus::Unchecked,
                "AMBIGUOUS_URL_HOST",
            );
            coverage(
                plugin,
                RuleId::McpHttps,
                &target(pointer, "url"),
                CoverageStatus::Unchecked,
                "AMBIGUOUS_URL_HOST",
            );
        }
        UrlCheck::HttpsRequired => {
            return skip(
                plugin,
                RuleId::McpHttps,
                name,
                Some(&format!("{pointer}/url")),
                "MCP_HTTPS",
                "a non-loopback MCP URL must use HTTPS",
            );
        }
        UrlCheck::Ok => {
            coverage(
                plugin,
                RuleId::McpUrl,
                &target(pointer, "url"),
                CoverageStatus::Pass,
                "URL_VALID",
            );
            coverage(
                plugin,
                RuleId::McpHttps,
                &target(pointer, "url"),
                CoverageStatus::Pass,
                "HTTPS_OR_LOOPBACK",
            );
        }
    }
    if let Some(headers) = o.get("headers").and_then(Value::as_object) {
        let mut seen = BTreeSet::new();
        for (key, value) in headers {
            if HeaderName::from_bytes(key.as_bytes()).is_err()
                || HeaderValue::from_str(value.as_str().expect("checked")).is_err()
                || !seen.insert(key.to_ascii_lowercase())
            {
                return skip(
                    plugin,
                    RuleId::McpHeaders,
                    name,
                    Some(&format!("{pointer}/headers/{}", escape(key))),
                    "MCP_HEADERS",
                    "MCP headers contain an invalid field or a case-insensitive duplicate",
                );
            }
            if possible_secret(key, value.as_str().expect("checked")) {
                add_finding_pointer(
                    plugin,
                    RuleId::AdvicePossibleSecret,
                    "mcp.json".into(),
                    Scope::Server(name.into()),
                    Some(format!("{pointer}/headers/{}", escape(key))),
                    "POSSIBLE_SECRET",
                    "header name may indicate a secret; confirm it manually. This report does not include the value",
                );
                coverage(
                    plugin,
                    RuleId::AdvicePossibleSecret,
                    name,
                    CoverageStatus::Manual,
                    "POSSIBLE_SECRET",
                );
            }
        }
        coverage(
            plugin,
            RuleId::McpHeaders,
            &target(pointer, "headers"),
            CoverageStatus::Pass,
            "HEADERS_VALID",
        );
    } else {
        coverage(
            plugin,
            RuleId::McpHeaders,
            &target(pointer, "headers"),
            CoverageStatus::NotApplicable,
            "HEADERS_ABSENT",
        );
    }
}

fn env_checks(
    plugin: &mut PluginReport,
    name: &str,
    pointer: &str,
    env: &Map<String, Value>,
) -> bool {
    let mut lower = BTreeSet::new();
    for (key, value) in env {
        if key == "PLUGIN_ROOT" || key == "PLUGIN_DATA" {
            skip(
                plugin,
                RuleId::McpReservedEnv,
                name,
                Some(&format!("{pointer}/env/{}", escape(key))),
                "RESERVED_ENV",
                "env must not override a reserved variable",
            );
            return false;
        }
        let non_ascii = !key.is_ascii();
        if non_ascii
            || !lower.insert(key.to_ascii_lowercase())
            || matches!(
                key.to_ascii_uppercase().as_str(),
                "PLUGIN_ROOT" | "PLUGIN_DATA"
            )
        {
            add_finding_pointer(
                plugin,
                RuleId::AdviceEnvCase,
                "mcp.json".into(),
                Scope::Server(name.into()),
                Some(format!("{pointer}/env/{}", escape(key))),
                if non_ascii {
                    "ENV_CASE_UNCHECKED"
                } else {
                    "ENV_CASE"
                },
                "env key case may conflict across platforms",
            );
        }
        if possible_secret(key, value.as_str().expect("checked")) {
            add_finding_pointer(
                plugin,
                RuleId::AdvicePossibleSecret,
                "mcp.json".into(),
                Scope::Server(name.into()),
                Some(format!("{pointer}/env/{}", escape(key))),
                "POSSIBLE_SECRET",
                "env name may indicate a secret; confirm it manually. This report does not include the value",
            );
            coverage(
                plugin,
                RuleId::AdvicePossibleSecret,
                name,
                CoverageStatus::Manual,
                "POSSIBLE_SECRET",
            );
        }
    }
    true
}

fn safe_bare_command(command: &str) -> bool {
    !command.is_empty()
        && command
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._+-".contains(&byte))
}

fn cwd_check(
    root: &Path,
    plugin: &mut PluginReport,
    errors: &mut Vec<ToolError>,
    name: &str,
    pointer: &str,
    cwd: &str,
) {
    let root_form = cwd == "${PLUGIN_ROOT}" || cwd.starts_with("${PLUGIN_ROOT}/");
    let data_form = cwd == "${PLUGIN_DATA}" || cwd.starts_with("${PLUGIN_DATA}/");
    if !(cwd.starts_with("./") || root_form || data_form) {
        coverage_id(
            plugin,
            "AP-PATH-RELATIVE-FORM",
            &target(pointer, "cwd"),
            CoverageStatus::Fail,
            "CWD_RELATIVE_FORM",
        );
        skip(
            plugin,
            RuleId::McpCwdForm,
            name,
            Some(&format!("{pointer}/cwd")),
            "CWD_FORM",
            "cwd must be ./, ${PLUGIN_ROOT}, or ${PLUGIN_DATA}",
        );
        return;
    }
    coverage_id(
        plugin,
        "AP-PATH-RELATIVE-FORM",
        &target(pointer, "cwd"),
        CoverageStatus::Pass,
        "CWD_RELATIVE_FORM_VALID",
    );
    if data_form || cwd.contains("${PLUGIN_DATA}") {
        coverage(
            plugin,
            RuleId::McpCwdForm,
            &target(pointer, "cwd"),
            CoverageStatus::Pass,
            "CWD_DATA_FORM_VALID",
        );
        coverage(
            plugin,
            RuleId::PathServerEscape,
            &target(pointer, "cwd"),
            CoverageStatus::Runtime,
            "PLUGIN_DATA_RUNTIME",
        );
        return;
    }
    let Some(root_text) = root.to_str() else {
        error(
            errors,
            root,
            "PATH_ENCODING",
            "package root path is not valid UTF-8, so PLUGIN_ROOT cannot be expanded",
        );
        coverage(
            plugin,
            RuleId::PathServerEscape,
            &target(pointer, "cwd"),
            CoverageStatus::Unchecked,
            "ROOT_PATH_ENCODING",
        );
        return;
    };
    let expanded = expansion::once(cwd, root_text, "${PLUGIN_DATA}");
    let path = if cwd.starts_with("./") {
        root.join(expanded)
    } else {
        std::path::PathBuf::from(expanded)
    };
    match inside(root, path, plugin, errors, name, pointer, "cwd") {
        PathCheck::Inside => {}
        PathCheck::Unresolved => return,
        PathCheck::Outside | PathCheck::Io => return,
    }
    coverage(
        plugin,
        RuleId::McpCwdForm,
        &target(pointer, "cwd"),
        CoverageStatus::Pass,
        "CWD_CONTAINED",
    );
}

enum PathCheck {
    Inside,
    Outside,
    Unresolved,
    Io,
}

fn inside(
    root: &Path,
    path: std::path::PathBuf,
    plugin: &mut PluginReport,
    errors: &mut Vec<ToolError>,
    name: &str,
    pointer: &str,
    field: &str,
) -> PathCheck {
    match containment::resolve(root, &path) {
        Ok(Resolution::Inside(_)) => {
            coverage(
                plugin,
                RuleId::PathServerEscape,
                &target(pointer, field),
                CoverageStatus::Pass,
                "PATH_CONTAINED",
            );
            PathCheck::Inside
        }
        Ok(Resolution::Outside) => {
            skip(
                plugin,
                RuleId::PathServerEscape,
                name,
                Some(&format!("{pointer}/{field}")),
                "SERVER_OUTSIDE_ROOT",
                "MCP server path is outside the package root",
            );
            PathCheck::Outside
        }
        Ok(Resolution::Unresolved) => {
            coverage(
                plugin,
                RuleId::PathServerEscape,
                &target(pointer, field),
                CoverageStatus::Unchecked,
                "PATH_UNRESOLVED",
            );
            add_finding_pointer(
                plugin,
                RuleId::AdviceUnresolvedPath,
                "mcp.json".into(),
                Scope::Server(name.into()),
                Some(format!("{pointer}/{field}")),
                "PATH_UNRESOLVED",
                "MCP path cannot be resolved yet and was not treated as an escape",
            );
            PathCheck::Unresolved
        }
        Err(_) => {
            error(
                errors,
                &path,
                "MCP_PATH_IO",
                "cannot resolve the MCP server path",
            );
            coverage(
                plugin,
                RuleId::PathServerEscape,
                &target(pointer, field),
                CoverageStatus::Unchecked,
                "PATH_IO_UNRESOLVED",
            );
            PathCheck::Io
        }
    }
}

enum UrlCheck {
    Ok,
    Invalid,
    Ambiguous,
    HttpsRequired,
}
fn valid_url(raw: &str) -> UrlCheck {
    if raw.contains('#') || raw.contains('\\') || raw.chars().any(char::is_whitespace) {
        return UrlCheck::Invalid;
    }
    let Some((scheme_raw, after_scheme)) = raw.split_once("://") else {
        return UrlCheck::Invalid;
    };
    if !scheme_raw.eq_ignore_ascii_case("http") && !scheme_raw.eq_ignore_ascii_case("https") {
        return UrlCheck::Invalid;
    }
    let authority = after_scheme.split(['/', '?', '#']).next().unwrap_or("");
    if authority.is_empty() || authority.contains('@') {
        return UrlCheck::Invalid;
    }
    let raw_host = raw_host(authority);
    if raw_host.is_empty() {
        return UrlCheck::Invalid;
    }
    // These spellings have historically been parsed differently by URL and OS
    // stacks. Preserve the raw ambiguity rather than trusting normalization.
    if ambiguous_raw_host(raw_host) {
        return UrlCheck::Ambiguous;
    }
    let Ok(url) = Url::parse(raw) else {
        return UrlCheck::Invalid;
    };
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || url.username() != ""
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return UrlCheck::Invalid;
    }
    let host = url.host_str().unwrap_or("").trim_matches(['[', ']']);
    if host
        .parse::<std::net::Ipv6Addr>()
        .is_ok_and(|ip| ip.to_ipv4_mapped().is_some())
    {
        return UrlCheck::Ambiguous;
    }
    if url.scheme().eq_ignore_ascii_case("https") {
        return UrlCheck::Ok;
    }
    if host.eq_ignore_ascii_case("localhost") {
        return UrlCheck::Ok;
    }
    match host.parse::<IpAddr>() {
        Ok(IpAddr::V4(v)) if v.is_loopback() => UrlCheck::Ok,
        Ok(IpAddr::V6(v)) if v.is_loopback() => UrlCheck::Ok,
        Ok(_) => UrlCheck::HttpsRequired,
        Err(_) => UrlCheck::HttpsRequired,
    }
}
fn raw_host(authority: &str) -> &str {
    if let Some(rest) = authority.strip_prefix('[') {
        rest.split(']').next().unwrap_or("")
    } else {
        authority.split(':').next().unwrap_or("")
    }
}
fn ambiguous_raw_host(host: &str) -> bool {
    if host.contains('%')
        || host.to_ascii_lowercase().starts_with("0x")
        || host.to_ascii_lowercase().contains("::ffff:")
    {
        return true;
    }
    if host.chars().all(|c| c.is_ascii_digit() || c == '.') {
        let parts: Vec<_> = host.split('.').collect();
        return parts.len() != 4
            || parts
                .iter()
                .any(|part| part.len() > 1 && part.starts_with('0'));
    }
    // An authority beginning with a decimal octet is plausibly an IPv4
    // spelling. If it is not the exact dotted-decimal grammar, do not let a
    // downstream URL parser reinterpret it as a DNS name or address.
    if host.contains('.')
        && host
            .split('.')
            .next()
            .is_some_and(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
    {
        return true;
    }
    false
}
fn recognized_mcp_schema(schema: &str) -> bool {
    let Some(version) = schema
        .strip_prefix("https://agent-plugins.org/schemas/")
        .and_then(|x| x.strip_suffix("/mcp.schema.json"))
    else {
        return false;
    };
    matches!(version, "1.0.0" | "1.1.0")
}
fn target(pointer: &str, field: &str) -> String {
    format!("mcp.json#{pointer}/{field}")
}
fn closed(o: &Map<String, Value>, allowed: &[&str]) -> bool {
    o.keys().all(|x| allowed.contains(&x.as_str()))
}
fn strings(v: Option<&Value>) -> bool {
    v.is_none_or(|x| x.as_array().is_some_and(|a| a.iter().all(Value::is_string)))
}
fn string_map(v: Option<&Value>) -> bool {
    v.is_none_or(|x| {
        x.as_object()
            .is_some_and(|o| o.values().all(Value::is_string))
    })
}
fn possible_secret(key: &str, value: &str) -> bool {
    !value.is_empty()
        && [
            "token",
            "secret",
            "password",
            "api_key",
            "authorization",
            "proxy-authorization",
        ]
        .iter()
        .any(|x| key.to_ascii_lowercase().contains(x))
}
fn escape(x: &str) -> String {
    x.replace('~', "~0").replace('/', "~1")
}
fn coverage(
    plugin: &mut PluginReport,
    rule: RuleId,
    target: &str,
    status: CoverageStatus,
    reason: &str,
) {
    plugin.coverage.push(Coverage {
        rule_id: rule.as_str().into(),
        target: target.into(),
        status,
        reason_code: Some(reason.into()),
    });
}
fn coverage_id(
    plugin: &mut PluginReport,
    rule_id: &str,
    target: &str,
    status: CoverageStatus,
    reason: &str,
) {
    plugin.coverage.push(Coverage {
        rule_id: rule_id.into(),
        target: target.into(),
        status,
        reason_code: Some(reason.into()),
    });
}
fn disable(
    plugin: &mut PluginReport,
    rule: RuleId,
    pointer: Option<&str>,
    code: &str,
    message: &str,
) {
    add_finding_pointer(
        plugin,
        rule,
        "mcp.json".into(),
        Scope::ComponentType("mcp".into()),
        pointer.map(str::to_owned),
        code,
        message,
    );
    coverage(
        plugin,
        RuleId::McpEnvelope,
        "mcp.json",
        CoverageStatus::Fail,
        code,
    );
    if rule != RuleId::McpEnvelope {
        coverage(
            plugin,
            rule,
            "mcp.json#/$schema",
            CoverageStatus::Fail,
            code,
        );
    }
    for id in [
        "AP-MCP-BUNDLED-COMMAND",
        "AP-MCP-PATH-DEPENDENCE",
        "AP-MCP-HEADER-SECRETS",
        "AP-MCP-ENV-SECRETS",
        "AP-MCP-COMMAND",
    ] {
        plugin.coverage.push(Coverage {
            rule_id: id.into(),
            target: "mcp.json".into(),
            status: CoverageStatus::Blocked,
            reason_code: Some(code.into()),
        });
    }
}
fn skip(
    plugin: &mut PluginReport,
    rule: RuleId,
    name: &str,
    pointer: Option<&str>,
    code: &str,
    message: &str,
) {
    add_finding_pointer(
        plugin,
        rule,
        "mcp.json".into(),
        Scope::Server(name.into()),
        pointer.map(str::to_owned),
        code,
        message,
    );
    let coverage_target = pointer.map_or_else(
        || format!("mcp.json#/mcpServers/{}", escape(name)),
        |p| format!("mcp.json#{p}"),
    );
    coverage(plugin, rule, &coverage_target, CoverageStatus::Fail, code);
    if rule == RuleId::McpServerVariant {
        for derived in [
            RuleId::McpCommand,
            RuleId::McpCwdForm,
            RuleId::PathServerEscape,
            RuleId::McpUrl,
            RuleId::McpHttps,
            RuleId::McpHeaders,
            RuleId::McpReservedEnv,
        ] {
            coverage(
                plugin,
                derived,
                &coverage_target,
                CoverageStatus::Blocked,
                "VARIANT_INVALID",
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{UrlCheck, escape, valid_url};

    #[test]
    fn url_boundaries_do_not_get_normalized_away() {
        assert!(matches!(valid_url("http://localhost:3000"), UrlCheck::Ok));
        assert!(matches!(valid_url("http://[::1]"), UrlCheck::Ok));
        assert!(matches!(valid_url("http://127.1"), UrlCheck::Ambiguous));
        assert!(matches!(
            valid_url("https://@example.com"),
            UrlCheck::Invalid
        ));
        assert!(matches!(
            valid_url("https://example.com#"),
            UrlCheck::Invalid
        ));
    }

    #[test]
    fn pointer_uses_rfc_6901_escaping() {
        assert_eq!(escape("a/b~c"), "a~1b~0c");
    }
}
