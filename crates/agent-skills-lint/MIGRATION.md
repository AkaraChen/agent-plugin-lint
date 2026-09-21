# skills-ref 迁移说明

本 crate 从 skills-ref 的解析/校验实现迁移而来，但公开接口已改为 Rust
结构化数据：`validate_source` 返回 `SkillReport`，其中的 `SkillIssue`、
`SkillRule`、`SkillCoverageStatus` 和 `IssueLevel` 用 enum 表示可机器读取的
规则、严重性和覆盖状态，而不是仅返回文本。`parser` 保留 YAML frontmatter
边界与原始 body；`validator` 只做离线数据判定。原 skills-ref 仓库未被修改。

完整来源、快照版本与行为差异见仓库 `research/PROVENANCE.md`。
