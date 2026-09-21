use crate::report::{Confidence, Effect, Obligation, Radius, Subject};
use serde::{Serialize, Serializer};

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
    ExtensionUnknown,
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
            Self::ExtensionUnknown => "AP-EXTENSION-UNKNOWN",
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
            Self::ExtensionUnknown => RuleMetadata {
                spec: &["§8.1", "§11.1"],
                radius: Advisory,
                normative: true,
                obligation: Must,
                subject: Subject::Client,
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
}
