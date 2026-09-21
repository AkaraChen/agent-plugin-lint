use serde::{Deserialize, Serialize};

/// Agent Skills 规则组；消费者应依赖此标识，不应解析人类文案。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SkillRule {
    Frontmatter,
    Name,
    Description,
    OptionalFields,
    SizeGuidance,
}

/// skill 校验结果的机器可读原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SkillIssueKind {
    MissingFrontmatter,
    UnclosedFrontmatter,
    InvalidYaml,
    NotMapping,
    MissingField,
    WrongType,
    Empty,
    TooLong,
    InvalidCharacters,
    InvalidHyphens,
    DirectoryMismatch,
    UnknownField,
    AmbiguousUnicode,
    UnsupportedYaml,
    TooManyLines,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueLevel {
    Error,
    Advisory,
    Unchecked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillIssue {
    pub rule: SkillRule,
    pub kind: SkillIssueKind,
    pub field: Option<String>,
    pub level: IssueLevel,
    pub location: Option<SourceLocation>,
    pub actual: Option<usize>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SkillCoverageStatus {
    Pass,
    Fail,
    Blocked,
    Unchecked,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillCoverage {
    pub rule: SkillRule,
    pub status: SkillCoverageStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillReport {
    pub properties: Option<crate::SkillProperties>,
    pub issues: Vec<SkillIssue>,
    pub coverage: Vec<SkillCoverage>,
}

impl SkillReport {
    /// 是否含有确定的错误。此值不代表全部检查都已通过。
    pub fn has_errors(&self) -> bool {
        self.issues
            .iter()
            .any(|issue| issue.level == IssueLevel::Error)
    }

    /// 是否含有未检查结果；调用方不得将其当作通过。
    pub fn has_unchecked(&self) -> bool {
        self.issues
            .iter()
            .any(|issue| issue.level == IssueLevel::Unchecked)
            || self.coverage.iter().any(|coverage| {
                matches!(
                    coverage.status,
                    SkillCoverageStatus::Unchecked | SkillCoverageStatus::Blocked
                )
            })
    }
}
