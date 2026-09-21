# astra 独立验证记录

本记录区分源码阅读、实际执行与未验证范围；实现 agent 的总结不作为通过凭证。

## A：立项与证据

- 仓库 https://github.com/AkaraChen/agent-plugin-lint 已创建并克隆；`gh repo view --json visibility,licenseInfo,url` 验证 PUBLIC + MIT。
- 本机实际 `cargo 1.97.1`、`rustc 1.97.1`；不沿用背景材料中的版本数字。
- 骨架执行 `cargo test --workspace`、`cargo clippy --workspace --all-targets -- -D warnings` 通过；当时是 **0 测试**，仅证明骨架编译，不作为规则验收。
- 两份 schema 按 `SHA1("blob " + 字节数 + NUL + 正文)` 与索引核对通过；原文及 research 既有文件复制前后摘要相同。
- 91 个规则 ID 唯一；AP §1–11 中 74 个带要求关键词的源行都有规则索引映射；这不是 74 项执行测试。
- 已检查 AP §4.1 的最窄失败边界、§5.2/§11.3 两个非 fatal 例外、§7.1 直接发现、§7.2.2 envelope/entry 隔离、§8/§8.1 扩展处理。实现验证分片记录如下。

## 真实语料版本

`eric-way` 默认 main `147fe6b840aab1f183cfd22d9f5cb61a14c35d63` 已无 plugins 目录。本次新克隆检出设计审计快照 `37d007ca28263d182aab6a5cc62d1bbda246fdfb`；子模块 guided-review 锁定 `05a5924108a6a81ce926c23f053a7d74528d5a62`。没有改语料内容。

使用 Python pathlib 严格 resolve、按目录边界比较，已独立核实 11 plugin、24 直接 skill 候选、0 mcp.json、3 外链：

| 路径 | §4.1 要求的后果 |
|---|---|
| plugins/frontend/skills/e2e-testing/references/docker.md | ignored / deny-path，访问时拒绝 |
| plugins/review/skills/github-pr/references/viewed-state.md | ignored / deny-path，访问时拒绝 |
| plugins/review/skills/guided-review | component / skip-skill，结合 §7.1 跳过该 skill |

此段是文件系统事实核实；引擎结果另记，不将 Python 脚本充当引擎验收。

## 尚未验证

引擎各切片待 review。Windows/macOS、实际宿主加载、安装器产物、MCP 启动/连接/认证/握手、PLUGIN_DATA 生命周期、秘密真实性、域名控制权、完整客户端符合性、发布安装尚未验证。
