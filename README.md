# agent-plugin-lint

离线、只读的 Agent Plugins 1.0.0 校验器，解释问题会导致整包拒绝、组件跳过、单一路径访问被拒，还是仅需人工检查。

当前为分片开发中的 Rust workspace，尚无已发布版本。设计与规则见 [DESIGN.md](DESIGN.md) 和 [rules.md](rules.md)，验证记录见后续 REVIEW.md。

- `agent-skills-lint`：SKILL.md 解析与结构化判定，吸收 AkaraChen/skills-ref 的实现；原仓库保持不变。
- `agent-plugin-lint`：manifest、发现、包含关系、MCP 与扩展编排；提供 `ap-lint`。

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

运行时不联网，不下载 schema，不执行 skill/MCP，不修改被检查包。schema 已固定在 research/schemas/；构建获取 Cargo 依赖不属于运行时网络。

MIT。第三方来源和快照见 [research/PROVENANCE.md](research/PROVENANCE.md)。
