use crate::{Coverage, CoverageStatus, Finding, Radius, RuleId, Scope};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
pub const SCHEMA: &str = "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json";
/// S2a only executes manifest rules; future RuleId variants are not implicitly covered here.
const MANIFEST_RULES: [RuleId; 15] = [
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
];
pub struct ManifestValidation {
    pub findings: Vec<Finding>,
    pub coverage: Vec<Coverage>,
    pub rejected: bool,
    pub name: Option<String>,
    pub declared_spec: Option<String>,
    pub duplicate_json_keys: Option<bool>,
    pub has_license: bool,
    pub has_extensions: bool,
}

pub fn validate_manifest(value: &Value) -> ManifestValidation {
    let mut state = State::default();
    let Some(manifest) = value.as_object() else {
        state.fail(
            RuleId::ManifestJson,
            &[],
            "MANIFEST_NOT_OBJECT",
            "plugin.json must be a top-level object",
        );
        state.block_all_except(RuleId::ManifestJson, "ROOT_NOT_OBJECT");
        return state.finish();
    };
    state.has_license = manifest.contains_key("license");
    state.has_extensions = manifest.contains_key("extensions");
    state.pass(RuleId::ManifestJson);
    validate_unknown_fields(manifest, &mut state);
    let schema_valid = required(manifest, "$schema", &mut state);
    let name_valid = required(manifest, "name", &mut state);
    if schema_valid {
        let schema = manifest["$schema"].as_str().expect("required string");
        if schema == SCHEMA {
            state.declared_spec = Some("1.0.0".into());
            state.pass(RuleId::ManifestSchemaId);
        } else {
            state.fail(
                RuleId::ManifestSchemaId,
                &["$schema"],
                "SCHEMA_ID",
                "unsupported schema identifier",
            );
        }
    } else {
        state.block(RuleId::ManifestSchemaId, "REQUIRED_FIELD_INVALID");
    }
    if name_valid {
        validate_name(
            manifest["name"].as_str().expect("required string"),
            &mut state,
        );
    } else {
        for rule in [
            RuleId::NameLength,
            RuleId::NameCharset,
            RuleId::NameEnds,
            RuleId::NameRepetition,
        ] {
            state.block(rule, "REQUIRED_FIELD_INVALID");
        }
    }
    validate_metadata(manifest, &mut state);
    validate_author(manifest, &mut state);
    validate_extensions(manifest, &mut state);
    validate_version(manifest, &mut state);
    state.finish()
}
pub(crate) fn validate_manifest_document(
    value: &Value,
    duplicate_json_keys: bool,
) -> ManifestValidation {
    let mut validation = validate_manifest(value);
    validation.duplicate_json_keys = Some(duplicate_json_keys);
    validation
}
pub fn invalid_json_manifest() -> ManifestValidation {
    let mut state = State::default();
    state.fail(
        RuleId::ManifestJson,
        &[],
        "JSON_INVALID",
        "plugin.json is not valid JSON",
    );
    state.block_all_except(RuleId::ManifestJson, "JSON_INVALID");
    state.finish()
}
#[derive(Default)]
struct State {
    findings: Vec<Finding>,
    coverage: BTreeMap<RuleId, (CoverageStatus, Option<String>)>,
    rejected: bool,
    name: Option<String>,
    declared_spec: Option<String>,
    has_license: bool,
    has_extensions: bool,
}
impl State {
    fn pass(&mut self, rule: RuleId) {
        self.set(rule, CoverageStatus::Pass, None);
    }
    fn not_applicable(&mut self, rule: RuleId, reason: &str) {
        self.set(rule, CoverageStatus::NotApplicable, Some(reason));
    }
    fn block(&mut self, rule: RuleId, reason: &str) {
        self.set(rule, CoverageStatus::Blocked, Some(reason));
    }
    fn fail(&mut self, rule: RuleId, segments: &[&str], code: &str, message: &str) {
        let metadata = rule.metadata();
        if metadata.radius == Radius::Fatal {
            self.rejected = true;
        }
        self.findings.push(Finding {
            rule_id: rule,
            spec: metadata.spec.iter().map(|item| (*item).into()).collect(),
            radius: metadata.radius,
            normative: metadata.normative,
            obligation: metadata.obligation,
            subject: metadata.subject,
            confidence: metadata.confidence,
            path: "plugin.json".into(),
            pointer: (!segments.is_empty()).then(|| pointer(segments)),
            line: None,
            column: None,
            scope: Scope::Plugin,
            effect: metadata.effect,
            evidence_code: code.into(),
            message: message.into(),
            hint: None,
        });
        self.set(rule, CoverageStatus::Fail, Some(code));
    }
    fn set(&mut self, rule: RuleId, status: CoverageStatus, reason: Option<&str>) {
        if self
            .coverage
            .get(&rule)
            .is_some_and(|(current, _)| *current == CoverageStatus::Fail)
            && status != CoverageStatus::Fail
        {
            return;
        }
        self.coverage
            .insert(rule, (status, reason.map(str::to_owned)));
    }
    fn block_all_except(&mut self, except: RuleId, reason: &str) {
        for rule in MANIFEST_RULES {
            if rule != except {
                self.block(rule, reason);
            }
        }
    }
    fn finish(mut self) -> ManifestValidation {
        self.findings.sort_by_key(|finding| {
            (
                finding.path.clone(),
                finding.pointer.clone(),
                finding.rule_id.as_str(),
            )
        });
        let mut coverage: Vec<_> = MANIFEST_RULES
            .into_iter()
            .map(|rule| {
                let (status, reason_code) = self
                    .coverage
                    .remove(&rule)
                    .unwrap_or((CoverageStatus::Unchecked, Some("NOT_EXECUTED".into())));
                Coverage {
                    rule_id: rule.as_str().into(),
                    target: "plugin.json".into(),
                    status,
                    reason_code,
                }
            })
            .collect();
        coverage.sort_by(|left, right| {
            (&left.rule_id, &left.target, left.status.as_str()).cmp(&(
                &right.rule_id,
                &right.target,
                right.status.as_str(),
            ))
        });
        ManifestValidation {
            findings: self.findings,
            coverage,
            rejected: self.rejected,
            name: self.name,
            declared_spec: self.declared_spec,
            duplicate_json_keys: None,
            has_license: self.has_license,
            has_extensions: self.has_extensions,
        }
    }
}
fn validate_unknown_fields(manifest: &Map<String, Value>, state: &mut State) {
    let allowed: BTreeSet<&str> = [
        "$schema",
        "name",
        "version",
        "description",
        "author",
        "homepage",
        "repository",
        "license",
        "keywords",
        "extensions",
    ]
    .into_iter()
    .collect();
    let mut found = false;
    for key in manifest.keys() {
        if !allowed.contains(key.as_str()) {
            found = true;
            state.fail(
                RuleId::ManifestUnknownField,
                &[key],
                "UNKNOWN_FIELD",
                "unknown top-level field will be ignored",
            );
        }
    }
    if !found {
        state.pass(RuleId::ManifestUnknownField);
    }
}
fn required(manifest: &Map<String, Value>, key: &str, state: &mut State) -> bool {
    match manifest.get(key) {
        Some(Value::String(value)) if !value.is_empty() => {
            state.pass(RuleId::ManifestRequired);
            true
        }
        _ => {
            state.fail(
                RuleId::ManifestRequired,
                &[key],
                "REQUIRED_FIELD",
                "required field is missing, has the wrong type, or is empty",
            );
            false
        }
    }
}
fn validate_name(name: &str, state: &mut State) {
    let mut valid = true;
    if name.chars().count() > 64 {
        valid = false;
        state.fail(
            RuleId::NameLength,
            &["name"],
            "NAME_LENGTH",
            "plugin name length is out of range",
        );
    } else {
        state.pass(RuleId::NameLength);
    }
    if !name.bytes().all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'.')
    }) {
        valid = false;
        state.fail(
            RuleId::NameCharset,
            &["name"],
            "NAME_CHARSET",
            "plugin name contains characters that are not allowed",
        );
    } else {
        state.pass(RuleId::NameCharset);
    }
    if !name
        .as_bytes()
        .first()
        .is_some_and(u8::is_ascii_alphanumeric)
        || !name
            .as_bytes()
            .last()
            .is_some_and(u8::is_ascii_alphanumeric)
    {
        valid = false;
        state.fail(
            RuleId::NameEnds,
            &["name"],
            "NAME_ENDS",
            "plugin name must start and end with a letter or digit",
        );
    } else {
        state.pass(RuleId::NameEnds);
    }
    if name.contains("--") || name.contains("..") {
        valid = false;
        state.fail(
            RuleId::NameRepetition,
            &["name"],
            "NAME_REPETITION",
            "plugin name must not contain repeated separators",
        );
    } else {
        state.pass(RuleId::NameRepetition);
    }
    if valid {
        state.name = Some(name.into());
    }
}
fn validate_metadata(manifest: &Map<String, Value>, state: &mut State) {
    let scalar_fields = [
        "version",
        "description",
        "homepage",
        "repository",
        "license",
    ];
    let invalid_field = scalar_fields
        .into_iter()
        .find(|field| manifest.contains_key(*field) && !manifest[*field].is_string());
    let keywords_invalid = manifest.get("keywords").is_some_and(|keywords| {
        !keywords
            .as_array()
            .is_some_and(|items| items.iter().all(Value::is_string))
    });
    if let Some(field) = invalid_field {
        state.fail(
            RuleId::ManifestMetadataType,
            &[field],
            "METADATA_TYPE",
            "metadata field has the wrong type",
        );
    } else if keywords_invalid {
        state.fail(
            RuleId::ManifestMetadataType,
            &["keywords"],
            "KEYWORDS_TYPE",
            "keywords must be an array of strings",
        );
    } else {
        state.pass(RuleId::ManifestMetadataType);
    }
}
fn validate_author(manifest: &Map<String, Value>, state: &mut State) {
    let Some(author) = manifest.get("author") else {
        state.not_applicable(RuleId::ManifestAuthor, "FIELD_ABSENT");
        return;
    };
    let Some(author) = author.as_object() else {
        state.fail(
            RuleId::ManifestAuthor,
            &["author"],
            "AUTHOR_TYPE",
            "author must be an object",
        );
        return;
    };
    let mut invalid = false;
    for (key, value) in author {
        if !["name", "email", "url"].contains(&key.as_str()) || !value.is_string() {
            invalid = true;
            state.fail(
                RuleId::ManifestAuthor,
                &["author", key],
                "AUTHOR_FIELD",
                "author field is invalid",
            );
        }
    }
    if !invalid {
        state.pass(RuleId::ManifestAuthor);
    }
}
fn validate_extensions(manifest: &Map<String, Value>, state: &mut State) {
    let Some(extensions) = manifest.get("extensions") else {
        state.not_applicable(RuleId::ExtensionsObject, "FIELD_ABSENT");
        state.not_applicable(RuleId::ExtensionNamespace, "FIELD_ABSENT");
        state.not_applicable(RuleId::ExtensionUnknown, "FIELD_ABSENT");
        state.not_applicable(RuleId::ExtensionFileLocation, "FIELD_ABSENT");
        return;
    };
    let Some(namespaces) = extensions.as_object() else {
        state.fail(
            RuleId::ExtensionsObject,
            &["extensions"],
            "EXTENSIONS_NOT_OBJECT",
            "extensions is not an object and will be ignored",
        );
        state.block(RuleId::ExtensionUnknown, "EXTENSIONS_IGNORED");
        state.block(RuleId::ExtensionNamespace, "EXTENSIONS_IGNORED");
        state.block(RuleId::ExtensionFileLocation, "EXTENSIONS_IGNORED");
        return;
    };
    state.pass(RuleId::ExtensionsObject);
    if namespaces.is_empty() {
        state.not_applicable(RuleId::ExtensionNamespace, "NO_NAMESPACES");
        state.not_applicable(RuleId::ExtensionUnknown, "NO_NAMESPACES");
        state.not_applicable(RuleId::ExtensionFileLocation, "NO_NAMESPACES");
    } else {
        let mut ambiguous = false;
        for namespace in namespaces.keys() {
            if namespace.is_empty() || namespace.contains('/') || namespace.contains('\\') {
                state.fail(
                    RuleId::ExtensionNamespace,
                    &["extensions", namespace],
                    "EXTENSION_NAMESPACE",
                    "extension namespace must be non-empty and must not contain path separators",
                );
            } else {
                ambiguous = true;
            }
        }
        if ambiguous {
            state.set(
                RuleId::ExtensionNamespace,
                CoverageStatus::Unchecked,
                Some("NAMESPACE_SYNTAX_UNSPECIFIED"),
            );
        }
        state.set(
            RuleId::ExtensionUnknown,
            CoverageStatus::Manual,
            Some("UNIMPLEMENTED_NAMESPACE"),
        );
        // Unknown extension directories are deliberately not enumerated: neither
        // manifest data nor a same-named directory implies the other exists.
        state.set(
            RuleId::ExtensionFileLocation,
            CoverageStatus::Manual,
            Some("UNIMPLEMENTED_NAMESPACE"),
        );
    }
}
fn validate_version(manifest: &Map<String, Value>, state: &mut State) {
    let Some(version) = manifest.get("version") else {
        state.not_applicable(RuleId::VersionSemver, "FIELD_ABSENT");
        return;
    };
    let Some(version) = version.as_str() else {
        state.block(RuleId::VersionSemver, "METADATA_TYPE_INVALID");
        return;
    };
    if semver::Version::parse(version).is_err() {
        state.fail(
            RuleId::VersionSemver,
            &["version"],
            "SEMVER",
            "version is not the recommended SemVer form",
        );
    } else {
        state.pass(RuleId::VersionSemver);
    }
}
fn pointer(segments: &[&str]) -> String {
    format!(
        "/{}",
        segments
            .iter()
            .map(|segment| segment.replace('~', "~0").replace('/', "~1"))
            .collect::<Vec<_>>()
            .join("/")
    )
}
