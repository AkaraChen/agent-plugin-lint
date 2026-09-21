//! 离线、只读的 Agent Plugins 1.0.0 校验库。
//!
//! 此 crate 不执行被检包、不联网、不下载 schema，也不提供修复功能。`Report`
//! 的 coverage 是逐规则、逐目标的实际检查记录；`complete` 仅指工具运行完成，
//! 不能解释为全部规则通过。
//! `RULE_NOT_EVALUATED` 是完整规则索引的诚实占位：没有目标级检查证据，不表示
//! 目标不存在或已通过；strict 不提升这一占位，但会提升实际扫描分支的静态
//! `unchecked`。未来扩展应在规则集版本演进时记录新的目标级状态。
//!
//! # 迁移说明
//!
//! `agent-skills-lint` 是从 skills-ref 的受限移植层，负责 SKILL.md 的解析与
//! 判定；本 crate 负责 Plugin manifest、发现、包含边界和 MCP 编排。原库未被
//! 修改，来源、快照与差异记录在仓库 `research/PROVENANCE.md`。公开 registry
//! 可用于将 1.0.0 原规则表与报告 coverage 对照，不能据此推断每条均有静态实现。
mod containment;
mod expansion;
mod json;
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
pub use rules::{
    RegisteredRule, RuleId, RuleRegistryMetadata, rule_registry, rule_registry_metadata,
};
