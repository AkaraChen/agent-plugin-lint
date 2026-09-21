//! 离线、只读的 Agent Plugins 1.0.0 校验库。
mod containment;
mod expansion;
mod lint;
mod manifest;
mod mcp;
mod report;
mod rules;
mod skills;
pub mod vendor;
pub use lint::{LintOptions, lint_path};
pub use manifest::{ManifestValidation, invalid_json_manifest, validate_manifest};
pub use report::{
    Confidence, Coverage, CoverageStatus, Effect, Finding, InputMode, Obligation, PluginReport,
    Policy, Radius, Report, Scope, Subject, Summary, ToolError,
};
pub use rules::RuleId;
#[doc(hidden)]
pub fn expand_once_for_test(source: &str, root: &str, data: &str) -> String {
    expansion::once(source, root, data)
}
