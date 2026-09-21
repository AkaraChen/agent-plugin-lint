use crate::report::{Confidence, Effect, Obligation, Radius, Subject};
use serde::{Deserialize, Serialize, Serializer};
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RuleId {
    ManifestLocation,
    ManifestJson,
    ManifestUnknownField,
    ManifestRequired,
    ManifestSchemaId,
    ManifestMetadataType,
    ManifestAuthor,
    NameLength,
    NameCharset,
    NameEnds,
    NameRepetition,
    VersionSemver,
    ExtensionsObject,
    ExtensionNamespace,
    ExtensionUnknown,
    ExtensionFileLocation,
    PathManifestEscape,
    PathFixedEscape,
    PathSkillEscape,
    PathResourceEscape,
    DiscoveryKind,
    AdviceUnresolvedPath,
    AdviceSkillsUnchecked,
    AsFrontmatter,
    AsName,
    AsDescription,
    AsOptionalFields,
    AsSizeGuidance,
    SkillConformance,
    McpEnvelope,
    McpSchemaId,
    McpVersionMatch,
    McpServerVariant,
    McpCommand,
    McpCwdForm,
    PathServerEscape,
    McpUrl,
    McpHttps,
    McpHeaders,
    McpReservedEnv,
    AdviceAmbiguousCommand,
    AdviceEnvCase,
    AdvicePossibleSecret,
    AdviceDuplicateJsonKey,
}

#[derive(Debug, Clone, Copy)]
pub struct RuleMetadata {
    pub spec: &'static [&'static str],
    pub radius: Radius,
    pub normative: bool,
    pub obligation: Obligation,
    pub subject: Subject,
    pub confidence: Confidence,
    pub effect: Effect,
}

/// 1.0.0 原规则表的公开索引。它用于让调用方对照 coverage；枚举并不表示
/// 每条规则都由本工具作了静态判定。
#[derive(Debug, Clone, Copy)]
pub struct RegisteredRule {
    pub id: &'static str,
    pub subject: Subject,
}

/// 规范原表的可序列化元数据。`spec_table` 是固定来源引用，不是工具猜测的
/// 规则说明；它与 `rules.md` 的 1.0.0 原表逐行对应。
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct RuleRegistryMetadata {
    pub id: String,
    pub radius: Radius,
    pub normative: bool,
    pub obligation: Obligation,
    pub subject: Subject,
    #[serde(rename = "specTable")]
    pub spec_table: String,
}

/// 返回完整的 1.0.0 规则 ID 注册表，顺序固定为规范原表顺序。
pub fn rule_registry() -> &'static [RegisteredRule] {
    RULE_REGISTRY
}

/// 返回完整 91 条规则的原表元数据，供调用者同 coverage 进行公开对照。
pub fn rule_registry_metadata() -> &'static [RuleRegistryMetadata] {
    static METADATA: OnceLock<Vec<RuleRegistryMetadata>> = OnceLock::new();
    METADATA
        .get_or_init(|| {
            serde_json::from_str(include_str!("rule_registry_1_0_0.json"))
                .expect("embedded 1.0.0 rule registry is valid")
        })
        .as_slice()
}

pub(crate) fn registry_subject(id: &str) -> Subject {
    RULE_REGISTRY
        .iter()
        .find(|rule| rule.id == id)
        .map(|rule| rule.subject)
        .unwrap_or(Subject::Tool)
}

const RULE_REGISTRY: &[RegisteredRule] = &[
    // package rules are the default; client and publisher obligations are
    // deliberately represented as manual coverage, never static pass/fail.
    RegisteredRule {
        id: "AP-MANIFEST-LOCATION",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-MANIFEST-JSON",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-MANIFEST-UNKNOWN-FIELD",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-MANIFEST-REQUIRED",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-MANIFEST-SCHEMA-ID",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-MANIFEST-METADATA-TYPE",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-MANIFEST-AUTHOR",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-NAME-LENGTH",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-NAME-CHARSET",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-NAME-ENDS",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-NAME-REPETITION",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-METADATA-NO-FORMAT-REJECTION",
        subject: Subject::Client,
    },
    RegisteredRule {
        id: "AP-VERSION-SEMVER",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-LICENSE-SPDX",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-CLIENT-MANIFEST-FIRST",
        subject: Subject::Client,
    },
    RegisteredRule {
        id: "AP-CLIENT-MANIFEST-REPORT",
        subject: Subject::Client,
    },
    RegisteredRule {
        id: "AP-LOCAL-SCHEMA-SELECTION",
        subject: Subject::Client,
    },
    RegisteredRule {
        id: "AP-PATH-MANIFEST-ESCAPE",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-PATH-FIXED-ESCAPE",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-PATH-SKILL-ESCAPE",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-PATH-SERVER-ESCAPE",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-PATH-RESOURCE-ESCAPE",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-PATH-RELATIVE-FORM",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-PATH-OPAQUE-VALUES",
        subject: Subject::Client,
    },
    RegisteredRule {
        id: "AP-PATH-NARROWEST-BOUNDARY",
        subject: Subject::Client,
    },
    RegisteredRule {
        id: "AP-DISCOVERY-FIXED",
        subject: Subject::Client,
    },
    RegisteredRule {
        id: "AP-DISCOVERY-MISSING",
        subject: Subject::Client,
    },
    RegisteredRule {
        id: "AP-DISCOVERY-KIND",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-DISCOVERY-SKILL-EXACT",
        subject: Subject::Client,
    },
    RegisteredRule {
        id: "AP-DISCOVERY-UNSUPPORTED",
        subject: Subject::Client,
    },
    RegisteredRule {
        id: "AP-ADVICE-UNDISCOVERED-SKILL",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-ADVICE-UNRESOLVED-PATH",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-SKILL-CONFORMANCE",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AS-FRONTMATTER",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AS-NAME",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AS-DESCRIPTION",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AS-OPTIONAL-FIELDS",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AS-DESCRIPTION-QUALITY",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AS-SIZE-GUIDANCE",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-CLIENT-SKILL-REPORT",
        subject: Subject::Client,
    },
    RegisteredRule {
        id: "AP-ADVICE-SKILLS-UNCHECKED",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-MCP-ENVELOPE",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-MCP-SCHEMA-ID",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-MCP-VERSION-MATCH",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-MCP-SERVER-VARIANT",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-MCP-COMMAND",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-MCP-BUNDLED-COMMAND",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-MCP-PATH-DEPENDENCE",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-MCP-CWD-FORM",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-MCP-URL",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-MCP-HTTPS",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-MCP-HEADERS",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-MCP-HEADER-SECRETS",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-MCP-ENV-SECRETS",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-MCP-RESERVED-ENV",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-ADVICE-POSSIBLE-SECRET",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-ADVICE-AMBIGUOUS-COMMAND",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-ADVICE-ENV-CASE",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-CLIENT-COMMAND-RESOLUTION",
        subject: Subject::Client,
    },
    RegisteredRule {
        id: "AP-CLIENT-CWD",
        subject: Subject::Client,
    },
    RegisteredRule {
        id: "AP-CLIENT-REMOTE-LITERALS",
        subject: Subject::Client,
    },
    RegisteredRule {
        id: "AP-CLIENT-HEADER-PRECEDENCE",
        subject: Subject::Client,
    },
    RegisteredRule {
        id: "AP-CLIENT-HEADER-REDIRECT",
        subject: Subject::Client,
    },
    RegisteredRule {
        id: "AP-CLIENT-TRANSPORT-MINIMUM",
        subject: Subject::Client,
    },
    RegisteredRule {
        id: "AP-CLIENT-TRANSPORT-BOTH",
        subject: Subject::Client,
    },
    RegisteredRule {
        id: "AP-CLIENT-TRANSPORT-INITIAL",
        subject: Subject::Client,
    },
    RegisteredRule {
        id: "AP-CLIENT-MCP-CONFIG-BOUNDARY",
        subject: Subject::Client,
    },
    RegisteredRule {
        id: "AP-CLIENT-MCP-ENTRY-BOUNDARY",
        subject: Subject::Client,
    },
    RegisteredRule {
        id: "AP-CLIENT-MCP-UNSUPPORTED",
        subject: Subject::Client,
    },
    RegisteredRule {
        id: "AP-CLIENT-MCP-CONNECTION",
        subject: Subject::Client,
    },
    RegisteredRule {
        id: "AP-CLIENT-MCP-REPORT",
        subject: Subject::Client,
    },
    RegisteredRule {
        id: "AP-EXTENSIONS-OBJECT",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-EXTENSION-NAMESPACE",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-EXTENSION-VALUE",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-EXTENSION-UNKNOWN",
        subject: Subject::Client,
    },
    RegisteredRule {
        id: "AP-EXTENSION-FILE-LOCATION",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-EXTENSION-CLIENT-DISCOVERY",
        subject: Subject::Client,
    },
    RegisteredRule {
        id: "AP-EXTENSION-DOMAIN-CONTROL",
        subject: Subject::Publisher,
    },
    RegisteredRule {
        id: "AP-EXTENSION-STABILITY",
        subject: Subject::Publisher,
    },
    RegisteredRule {
        id: "AP-CLIENT-ENV-ROOTS",
        subject: Subject::Client,
    },
    RegisteredRule {
        id: "AP-CLIENT-DATA-LIFECYCLE",
        subject: Subject::Client,
    },
    RegisteredRule {
        id: "AP-CLIENT-ENV-OVERLAY",
        subject: Subject::Client,
    },
    RegisteredRule {
        id: "AP-ENV-BASE-DEPENDENCE",
        subject: Subject::Package,
    },
    RegisteredRule {
        id: "AP-EXPANSION-SINGLE-PASS",
        subject: Subject::Client,
    },
    RegisteredRule {
        id: "AP-EXPANSION-LITERAL",
        subject: Subject::Client,
    },
    RegisteredRule {
        id: "AP-RELEASE-SCHEMA-PAIR",
        subject: Subject::Publisher,
    },
    RegisteredRule {
        id: "AP-RELEASE-SCHEMA-IMMUTABLE",
        subject: Subject::Publisher,
    },
    RegisteredRule {
        id: "AP-CLIENT-CONFORMANCE",
        subject: Subject::Client,
    },
    RegisteredRule {
        id: "AP-CLIENT-FAILURE-ISOLATION",
        subject: Subject::Client,
    },
    RegisteredRule {
        id: "AP-CLIENT-FAILURE-REPORT",
        subject: Subject::Client,
    },
    RegisteredRule {
        id: "AP-ADVICE-DUPLICATE-JSON-KEY",
        subject: Subject::Package,
    },
];

impl RuleId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ManifestLocation => "AP-MANIFEST-LOCATION",
            Self::ManifestJson => "AP-MANIFEST-JSON",
            Self::ManifestUnknownField => "AP-MANIFEST-UNKNOWN-FIELD",
            Self::ManifestRequired => "AP-MANIFEST-REQUIRED",
            Self::ManifestSchemaId => "AP-MANIFEST-SCHEMA-ID",
            Self::ManifestMetadataType => "AP-MANIFEST-METADATA-TYPE",
            Self::ManifestAuthor => "AP-MANIFEST-AUTHOR",
            Self::NameLength => "AP-NAME-LENGTH",
            Self::NameCharset => "AP-NAME-CHARSET",
            Self::NameEnds => "AP-NAME-ENDS",
            Self::NameRepetition => "AP-NAME-REPETITION",
            Self::VersionSemver => "AP-VERSION-SEMVER",
            Self::ExtensionsObject => "AP-EXTENSIONS-OBJECT",
            Self::ExtensionNamespace => "AP-EXTENSION-NAMESPACE",
            Self::ExtensionUnknown => "AP-EXTENSION-UNKNOWN",
            Self::ExtensionFileLocation => "AP-EXTENSION-FILE-LOCATION",
            Self::PathManifestEscape => "AP-PATH-MANIFEST-ESCAPE",
            Self::PathFixedEscape => "AP-PATH-FIXED-ESCAPE",
            Self::PathSkillEscape => "AP-PATH-SKILL-ESCAPE",
            Self::PathResourceEscape => "AP-PATH-RESOURCE-ESCAPE",
            Self::DiscoveryKind => "AP-DISCOVERY-KIND",
            Self::AdviceUnresolvedPath => "AP-ADVICE-UNRESOLVED-PATH",
            Self::AdviceSkillsUnchecked => "AP-ADVICE-SKILLS-UNCHECKED",
            Self::AsFrontmatter => "AS-FRONTMATTER",
            Self::AsName => "AS-NAME",
            Self::AsDescription => "AS-DESCRIPTION",
            Self::AsOptionalFields => "AS-OPTIONAL-FIELDS",
            Self::AsSizeGuidance => "AS-SIZE-GUIDANCE",
            Self::SkillConformance => "AP-SKILL-CONFORMANCE",
            Self::McpEnvelope => "AP-MCP-ENVELOPE",
            Self::McpSchemaId => "AP-MCP-SCHEMA-ID",
            Self::McpVersionMatch => "AP-MCP-VERSION-MATCH",
            Self::McpServerVariant => "AP-MCP-SERVER-VARIANT",
            Self::McpCommand => "AP-MCP-COMMAND",
            Self::McpCwdForm => "AP-MCP-CWD-FORM",
            Self::PathServerEscape => "AP-PATH-SERVER-ESCAPE",
            Self::McpUrl => "AP-MCP-URL",
            Self::McpHttps => "AP-MCP-HTTPS",
            Self::McpHeaders => "AP-MCP-HEADERS",
            Self::McpReservedEnv => "AP-MCP-RESERVED-ENV",
            Self::AdviceAmbiguousCommand => "AP-ADVICE-AMBIGUOUS-COMMAND",
            Self::AdviceEnvCase => "AP-ADVICE-ENV-CASE",
            Self::AdvicePossibleSecret => "AP-ADVICE-POSSIBLE-SECRET",
            Self::AdviceDuplicateJsonKey => "AP-ADVICE-DUPLICATE-JSON-KEY",
        }
    }
    pub fn metadata(self) -> RuleMetadata {
        use Effect::{Advise, DenyPath, DisableType, IgnoreField, RejectPlugin, SkipSkill};
        use Obligation::{Must, Should};
        use Radius::{Advisory, Component, Fatal, Ignored};
        let package = Subject::Package;
        let certain = Confidence::Certain;
        match self {
            Self::ManifestLocation => RuleMetadata {
                spec: &["§4.1", "§5.1"],
                radius: Fatal,
                normative: true,
                obligation: Must,
                subject: package,
                confidence: certain,
                effect: RejectPlugin,
            },
            Self::ManifestJson => RuleMetadata {
                spec: &["§5.2"],
                radius: Fatal,
                normative: true,
                obligation: Must,
                subject: package,
                confidence: certain,
                effect: RejectPlugin,
            },
            Self::ManifestUnknownField => RuleMetadata {
                spec: &["§5.2", "§11.3"],
                radius: Ignored,
                normative: true,
                obligation: Must,
                subject: package,
                confidence: certain,
                effect: IgnoreField,
            },
            Self::ManifestRequired => RuleMetadata {
                spec: &["§5.3"],
                radius: Fatal,
                normative: true,
                obligation: Must,
                subject: package,
                confidence: certain,
                effect: RejectPlugin,
            },
            Self::ManifestSchemaId => RuleMetadata {
                spec: &["§5.2"],
                radius: Fatal,
                normative: true,
                obligation: Must,
                subject: package,
                confidence: certain,
                effect: RejectPlugin,
            },
            Self::ManifestMetadataType => RuleMetadata {
                spec: &["§5.2", "§5.4"],
                radius: Fatal,
                normative: true,
                obligation: Must,
                subject: package,
                confidence: certain,
                effect: RejectPlugin,
            },
            Self::ManifestAuthor => RuleMetadata {
                spec: &["§5.4"],
                radius: Fatal,
                normative: true,
                obligation: Must,
                subject: package,
                confidence: certain,
                effect: RejectPlugin,
            },
            Self::NameLength | Self::NameCharset | Self::NameEnds | Self::NameRepetition => {
                RuleMetadata {
                    spec: &["§5.5"],
                    radius: Fatal,
                    normative: true,
                    obligation: Must,
                    subject: package,
                    confidence: certain,
                    effect: RejectPlugin,
                }
            }
            Self::VersionSemver => RuleMetadata {
                spec: &["§5.4", "§10.2"],
                radius: Advisory,
                normative: true,
                obligation: Should,
                subject: package,
                confidence: certain,
                effect: Advise,
            },
            Self::ExtensionsObject => RuleMetadata {
                spec: &["§5.2", "§8.1", "§11.3"],
                radius: Ignored,
                normative: true,
                obligation: Must,
                subject: package,
                confidence: certain,
                effect: IgnoreField,
            },
            Self::ExtensionNamespace => RuleMetadata {
                spec: &["§8", "§8.1"],
                radius: Fatal,
                normative: true,
                obligation: Must,
                subject: package,
                confidence: certain,
                effect: RejectPlugin,
            },
            Self::ExtensionUnknown => RuleMetadata {
                spec: &["§8.1", "§11.1"],
                radius: Advisory,
                normative: true,
                obligation: Must,
                subject: Subject::Client,
                confidence: certain,
                effect: Advise,
            },
            Self::ExtensionFileLocation => RuleMetadata {
                spec: &["§8", "§8.2"],
                radius: Advisory,
                normative: true,
                obligation: Must,
                subject: package,
                confidence: certain,
                effect: Advise,
            },
            Self::PathManifestEscape => RuleMetadata {
                spec: &["§4.1"],
                radius: Fatal,
                normative: true,
                obligation: Must,
                subject: package,
                confidence: certain,
                effect: RejectPlugin,
            },
            Self::PathFixedEscape | Self::DiscoveryKind => RuleMetadata {
                spec: &["§4.1", "§6.2"],
                radius: Component,
                normative: true,
                obligation: Must,
                subject: package,
                confidence: certain,
                effect: DisableType,
            },
            Self::PathSkillEscape => RuleMetadata {
                spec: &["§4.1", "§7.1"],
                radius: Component,
                normative: true,
                obligation: Must,
                subject: package,
                confidence: certain,
                effect: SkipSkill,
            },
            Self::PathResourceEscape => RuleMetadata {
                spec: &["§4.1"],
                radius: Ignored,
                normative: true,
                obligation: Must,
                subject: package,
                confidence: certain,
                effect: DenyPath,
            },
            Self::AsFrontmatter | Self::AsName | Self::AsDescription | Self::AsOptionalFields => {
                RuleMetadata {
                    spec: &["§7.1"],
                    radius: Component,
                    normative: true,
                    obligation: Must,
                    subject: package,
                    confidence: certain,
                    effect: SkipSkill,
                }
            }
            Self::AsSizeGuidance => RuleMetadata {
                spec: &["§7.1"],
                radius: Advisory,
                normative: true,
                obligation: Obligation::Recommended,
                subject: package,
                confidence: certain,
                effect: Advise,
            },
            Self::AdviceUnresolvedPath | Self::AdviceSkillsUnchecked => RuleMetadata {
                spec: &["§4.1", "§7.1"],
                radius: Advisory,
                normative: false,
                obligation: Obligation::None,
                subject: package,
                confidence: certain,
                effect: Advise,
            },
            Self::SkillConformance => RuleMetadata {
                spec: &["§7.1"],
                radius: Component,
                normative: true,
                obligation: Must,
                subject: package,
                confidence: certain,
                effect: SkipSkill,
            },
            Self::McpEnvelope => RuleMetadata {
                spec: &["§7.2.1", "§7.2.2"],
                radius: Component,
                normative: true,
                obligation: Must,
                subject: package,
                confidence: certain,
                effect: DisableType,
            },
            Self::McpSchemaId | Self::McpVersionMatch => RuleMetadata {
                spec: &["§7.2.2", "§10.1"],
                radius: Component,
                normative: true,
                obligation: Must,
                subject: package,
                confidence: certain,
                effect: DisableType,
            },
            Self::PathServerEscape => RuleMetadata {
                spec: &["§4.1", "§7.2.1", "§7.2.2"],
                radius: Component,
                normative: true,
                obligation: Must,
                subject: package,
                confidence: certain,
                effect: Effect::SkipServer,
            },
            Self::McpServerVariant
            | Self::McpCommand
            | Self::McpCwdForm
            | Self::McpUrl
            | Self::McpHttps
            | Self::McpHeaders
            | Self::McpReservedEnv => RuleMetadata {
                spec: &["§7.2.1", "§7.2.2"],
                radius: Component,
                normative: true,
                obligation: Must,
                subject: package,
                confidence: certain,
                effect: Effect::SkipServer,
            },
            Self::AdviceAmbiguousCommand | Self::AdviceEnvCase | Self::AdvicePossibleSecret => {
                RuleMetadata {
                    spec: &["§7.2.1", "§9.2"],
                    radius: Advisory,
                    normative: false,
                    obligation: Obligation::None,
                    subject: package,
                    confidence: certain,
                    effect: Advise,
                }
            }
            Self::AdviceDuplicateJsonKey => RuleMetadata {
                spec: &[],
                radius: Advisory,
                normative: false,
                obligation: Obligation::None,
                subject: package,
                confidence: certain,
                effect: Advise,
            },
        }
    }
}
impl Serialize for RuleId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn component_rule_metadata_has_narrow_boundaries() {
        let skill = RuleId::SkillConformance.metadata();
        assert_eq!(skill.spec, ["§7.1"]);
        assert_eq!(skill.radius, Radius::Component);
        assert_eq!(skill.effect, Effect::SkipSkill);
        let mcp = RuleId::McpEnvelope.metadata();
        assert_eq!(mcp.spec, ["§7.2.1", "§7.2.2"]);
        assert_eq!(mcp.radius, Radius::Component);
        assert_eq!(mcp.effect, Effect::DisableType);
    }

    #[test]
    fn public_registry_matches_fixed_91_row_metadata() {
        assert_eq!(rule_registry().len(), 91);
        assert_eq!(rule_registry_metadata().len(), 91);
        assert!(
            rule_registry_metadata()
                .iter()
                .all(|metadata| { rule_registry().iter().any(|rule| rule.id == metadata.id) })
        );
    }
}
