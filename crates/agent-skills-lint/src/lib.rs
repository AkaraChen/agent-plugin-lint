//! 离线、只读的 Agent Skills 校验库。
//!
//! 本 crate 依据仓库内 `research/agent-skills-specification.md` 的快照校验
//! `SKILL.md`。解析和 [`validate_source`] 不访问文件系统；目录 API 仅是便利层，
//! 并把 I/O 错误与内容诊断分开。
//!
//! 最初的 parser、validator 与 XML prompt 语义移植自 `skills-ref`
//! (`6c89f06b034240093e4933e066feba3ee1ac0ffa`)，但使用维护中的
//! `serde-saphyr` 后端，并修复 `metadata` 与 UTF-8 字符数问题。

mod diagnostic;
mod model;
mod parser;
mod prompt;
mod validator;

pub use diagnostic::{
    IssueLevel, SkillCoverage, SkillCoverageStatus, SkillIssue, SkillIssueKind, SkillReport,
    SkillRule, SourceLocation,
};
pub use model::SkillProperties;
pub use parser::{
    Frontmatter, ParseError, ReadPropertiesError, SkillIoError, YamlValue, find_skill_md,
    parse_frontmatter, read_properties, read_properties_from_source,
};
pub use prompt::to_prompt;
pub use validator::{validate_directory, validate_source};
