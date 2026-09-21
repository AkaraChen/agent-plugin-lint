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
    SkillConformance,
    McpEnvelope,
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
            Self::SkillConformance => "AP-SKILL-CONFORMANCE",
            Self::McpEnvelope => "AP-MCP-ENVELOPE",
            Self::AdviceDuplicateJsonKey => "AP-ADVICE-DUPLICATE-JSON-KEY",
        }
    }
    pub fn metadata(self) -> RuleMetadata {
        use Effect::{Advise, IgnoreField, RejectPlugin};
        use Obligation::{Must, Should};
        use Radius::{Advisory, Fatal, Ignored};
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
            Self::SkillConformance | Self::McpEnvelope => RuleMetadata {
                spec: &["§6.2", "§7.1"],
                radius: Fatal,
                normative: true,
                obligation: Must,
                subject: package,
                confidence: certain,
                effect: RejectPlugin,
            },
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
