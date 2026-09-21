//! 离线、只读的 Agent Plugins 1.0.0 校验库。
mod lint;
mod manifest;
mod report;
mod rules;
pub mod vendor;
pub use lint::{LintOptions, lint_path};
pub use manifest::{ManifestValidation, invalid_json_manifest, validate_manifest};
pub use report::{
    Confidence, Coverage, CoverageStatus, Effect, Finding, InputMode, Obligation, PluginReport,
    Policy, Radius, Report, Scope, Subject, Summary, ToolError,
};
pub use rules::RuleId;
