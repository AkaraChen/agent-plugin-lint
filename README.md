# agent-plugin-lint

离线、只读的 Agent Plugins 1.0.0 校验器。它区分整包拒绝、组件跳过、单一路径拒绝、人工/运行时检查与静态未检查，而不把“没有 finding”写成通过。

设计与规则见 [DESIGN.md](DESIGN.md) 和 [rules.md](rules.md)，验证记录见 [REVIEW.md](REVIEW.md)。

- `agent-skills-lint`：SKILL.md 解析与结构化判定，吸收 AkaraChen/skills-ref 的实现；原仓库保持不变。
- `agent-plugin-lint`：manifest、发现、包含关系、MCP 与扩展编排；提供 `ap-lint`。

```sh
cargo run -p agent-plugin-lint -- /path/to/plugin --format text
cargo run -p agent-plugin-lint -- /path/to/plugins --mode collection --json
```

`--json`（或 `--format json`）输出稳定排序的单一 JSON 文档。退出码为：`0` 没有本工具可确定的包侧 MUST 失败（strict 下也没有 advisory 或所选静态 `unchecked`）；`1` 为规范/strict 结果；`2` 为参数、I/O 或解析器表示能力错误。`complete: true` 只表示本次工具运行没有操作错误，**不表示 91 条规则均已通过**；请读取每条 `coverage.status` 与 `reasonCode`。

完整规则索引中的 `RULE_NOT_EVALUATED` 表示该规则尚无目标级检查证据：它既不说明目标不存在，也不是通过。strict 只选择具有实际目标的静态检查，不提升这类索引占位；它会提升实际目标上的静态 `unchecked`（例如 URL 形态歧义）。后续新增目标级检查可在规则集版本演进时选择并记录其状态。

当前静态范围覆盖 manifest、固定发现、SKILL.md、MCP 配置、路径边界、JSON 语法/重复键和部分 URL/环境检查。客户端、发布者、网络连接、运行时环境与未知 extension namespace 分别明确标为 `manual` 或 `runtime`；已遇到但尚未由静态代码实现的字段会标为 `unchecked`，在 strict 下失败。不会执行被检 command/skill/MCP，不联网、不请求 schema，也没有 `--fix`。

已在 Linux 上验证；其他操作系统尚未验证。内嵌的官方 schema 在 crate 内随包分发，运行时不依赖仓库检出或网络。

运行时不联网，不下载 schema，不执行 skill/MCP，不修改被检查包。schema 已固定在 research/schemas/；构建获取 Cargo 依赖不属于运行时网络。

代码采用 MIT。官方 schema 原字节采用 Apache-2.0；crate 包内的许可证与来源见 `crates/agent-plugin-lint/THIRD_PARTY.md`。skills-ref 的移植范围和迁移说明见 crate 文档及 [research/PROVENANCE.md](research/PROVENANCE.md)。
