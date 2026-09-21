# 协作约定

- DECISIONS.md 是历史已决事项的唯一真源；本轮用户授权优先。不得修改 /home/akrc/ap-lint-design、skills-ref 或 eric-way。
- astra 负责设计与 review；实现交由 Paseo 调度的 codex/gpt-5.6-terra，full-access。每片完成后停下等 astra 验收，禁止自行进入下一片。
- 按 DESIGN.md 和 rules.md 实现；规范判断回 research/1.0.0.md。中文文档与诊断，英文代码标识符及 rule ID。
- 运行时离线、只读，不执行被检代码，不请求 schema，无 --fix。不得把未知/未检查判通过或违规。
- 每片提供测试、diff 和限制；astra 独立执行 cargo test --workspace 和 cargo clippy --workspace --all-targets -- -D warnings。
- 实现 agent 不提交、不推送、不改设计决定；发现冲突回报 astra。测试可在临时目录构造语料。
