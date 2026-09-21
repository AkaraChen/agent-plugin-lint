use crate::RuleId;
use serde::Serialize;
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Radius {
    Fatal,
    Component,
    Ignored,
    Advisory,
}
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Effect {
    RejectPlugin,
    DisableType,
    SkipSkill,
    SkipServer,
    DenyPath,
    IgnoreField,
    Advise,
}
impl Effect {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RejectPlugin => "reject-plugin",
            Self::DisableType => "disable-type",
            Self::SkipSkill => "skip-skill",
            Self::SkipServer => "skip-server",
            Self::DenyPath => "deny-path",
            Self::IgnoreField => "ignore-field",
            Self::Advise => "advise",
        }
    }
}
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Obligation {
    Must,
    Should,
    Recommended,
    None,
}
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Subject {
    Package,
    Client,
    Publisher,
    Tool,
}
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    Certain,
    Heuristic,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    Plugin,
    ComponentType(String),
    Skill(String),
    Server(String),
    Path(String),
}
impl Serialize for Scope {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let (kind, id) = match self {
            Self::Plugin => ("plugin", None),
            Self::ComponentType(x) => ("component-type", Some(x)),
            Self::Skill(x) => ("skill", Some(x)),
            Self::Server(x) => ("server", Some(x)),
            Self::Path(x) => ("path", Some(x)),
        };
        let mut o = s.serialize_struct("Scope", 2)?;
        o.serialize_field("kind", kind)?;
        o.serialize_field("id", &id)?;
        o.end()
    }
}
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum CoverageStatus {
    Pass,
    Fail,
    NotApplicable,
    Blocked,
    Unchecked,
    Manual,
    Runtime,
}
impl CoverageStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::NotApplicable => "not-applicable",
            Self::Blocked => "blocked",
            Self::Unchecked => "unchecked",
            Self::Manual => "manual",
            Self::Runtime => "runtime",
        }
    }
}
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Coverage {
    #[serde(rename = "ruleId")]
    pub rule_id: String,
    pub target: String,
    pub status: CoverageStatus,
    #[serde(rename = "reasonCode")]
    pub reason_code: Option<String>,
}
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Finding {
    #[serde(rename = "ruleId")]
    pub rule_id: RuleId,
    pub spec: Vec<String>,
    pub radius: Radius,
    pub normative: bool,
    pub obligation: Obligation,
    pub subject: Subject,
    pub confidence: Confidence,
    pub path: String,
    pub pointer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<usize>,
    pub scope: Scope,
    pub effect: Effect,
    #[serde(rename = "evidenceCode")]
    pub evidence_code: String,
    pub message: String,
    pub hint: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum InputMode {
    Auto,
    Plugin,
    Collection,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Policy {
    pub strict: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ToolError {
    pub path: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Summary {
    pub fatal: usize,
    pub component: usize,
    pub ignored: usize,
    pub advisory: usize,
    pub errors: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PluginReport {
    pub root: String,
    pub name: Option<String>,
    #[serde(rename = "declaredSpec")]
    pub declared_spec: Option<String>,
    pub status: String,
    pub findings: Vec<Finding>,
    pub coverage: Vec<Coverage>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Report {
    #[serde(rename = "schemaVersion")]
    pub schema_version: u32,
    #[serde(rename = "toolVersion")]
    pub tool_version: String,
    #[serde(rename = "rulesetVersion")]
    pub ruleset_version: String,
    #[serde(rename = "specVersion")]
    pub spec_version: String,
    pub input: String,
    pub mode: InputMode,
    pub policy: Policy,
    pub complete: bool,
    pub plugins: Vec<PluginReport>,
    pub errors: Vec<ToolError>,
    pub summary: Summary,
    #[serde(rename = "exitCode")]
    pub exit_code: i32,
}
