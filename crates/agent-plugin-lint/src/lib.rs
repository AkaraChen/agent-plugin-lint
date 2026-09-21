//! 离线、只读的 Agent Plugins 1.0.0 校验库。
mod manifest;
mod report;
mod rules;
pub mod vendor;
pub use manifest::{ManifestValidation, validate_manifest};
pub use report::{
    Confidence, Coverage, CoverageStatus, Effect, Finding, Obligation, Radius, Scope, Subject,
};
pub use rules::RuleId;
