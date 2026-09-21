[English](README.md) · **中文**

<h1 align="center">ap-lint</h1>

<p align="center">
	<a href="https://github.com/AkaraChen/agent-plugin-lint/actions/workflows/ci.yml"><img src="https://github.com/AkaraChen/agent-plugin-lint/actions/workflows/ci.yml/badge.svg" alt="CI status"></a>
	<a href="#许可证"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT license"></a>
</p>

> [Agent Plugins](https://agent-plugins.org) 的 linter —— 它告诉你到底哪里会坏

任何 linter 都能告诉你「这个包不合法」。那是最容易的一半。

难的是：「不合法」并不是一件事。有的不合法会让客户端**整个拒收这个包**；有的不合法只会让**某一个 skill 悄悄消失** —— 插件照样装上，不报任何错，而你交付的某项能力就是不在那里了。你会在几周后从用户嘴里知道。

`ap-lint` 把后一种报得和前一种一样响。每条 finding 都带**爆炸半径** —— 合规客户端实际会怎么处理它。所以你永远知道自己搞坏的是整个包，还是包里的某一个组件。

**已在 Linux 上验证。macOS 与 Windows 尚未验证。** 见[平台支持](#平台支持)。

## 特性

- **给的是爆炸半径，不是 pass/fail。** 每条 finding 都说明客户端会怎么处理：拒收整个插件、跳过这一个 skill 或 MCP server、忽略某个字段，或只是提个建议。
- **抓得到「静默消失」。** skill 目录一旦逃出包根并不会报错 —— 规范要求客户端跳过它继续加载。`ap-lint` 会把它指名报出来。
- **从不猜。** 静态判不了的一律标 `unchecked`、`manual` 或 `runtime`，**绝不标成通过**。没有目标级检查的规则标 `RULE_NOT_EVALUATED`，那不是绿灯。
- **只读。** 不改你的包，不执行包里的任何东西，不启动你的 MCP server。
- **离线。** 运行时零网络，官方 schema 直接编进二进制。也**没有 `--fix`** —— 会动手改你包的 linter 是另一个工具，风险模型完全不同。
- **机器可读。** 一个顺序稳定的 JSON 文档给 CI，内容与人读输出一致，没有「只存在于文本里」的信息。
- **查的是整块可移植地板**，不只是 manifest：`plugin.json`、组件发现、每一个 `SKILL.md`、`mcp.json`、包内路径包含关系，以及那些「带不走」的宿主私有目录。
- **证据优先于自信。** 每条规则都映射到规范的具体章节，CI 在它声称支持的每个平台上跑全套。

## 安装

还没有正式发布，直接从仓库装：

```sh
cargo install --git https://github.com/AkaraChen/agent-plugin-lint agent-plugin-lint
```

或者 clone 下来自己构建：

```sh
git clone https://github.com/AkaraChen/agent-plugin-lint
cd agent-plugin-lint
cargo build --release
```

## 使用

```sh
ap-lint path/to/plugin              # 单个插件
ap-lint path/to/plugins             # 一个装着多个插件的目录
ap-lint path/to/plugins --json      # 机器可读报告
ap-lint path/to/plugins --strict    # 把 advisory 和未检查项也算失败
```

参数：

| 参数 | 含义 |
| --- | --- |
| `--mode auto\|plugin\|collection` | 怎么理解传入的路径。`auto` 在歧义时**拒绝猜**。 |
| `--spec 1.0.0` | 按哪个 Agent Plugins 版本检查。默认跟随包自己声明的版本。 |
| `--json`、`--format text\|json` | 输出格式。 |
| `--strict` | 把 advisory 与未检查项也当作失败。 |

真实输出长这样：

```console
$ ap-lint plugins
插件根：frontend（状态：accepted）
AP-PATH-RESOURCE-ESCAPE skills/e2e-testing/references/docker.md §4.1 RESOURCE_OUTSIDE_ROOT [frontend]
    资源路径位于包根之外，访问时会被拒绝
插件根：review（状态：accepted）
AP-PATH-RESOURCE-ESCAPE skills/github-pr/references/viewed-state.md §4.1 RESOURCE_OUTSIDE_ROOT [review]
AP-PATH-SKILL-ESCAPE    skills/guided-review §4.1、§7.1 SKILL_OUTSIDE_ROOT [review]
    skill 位于包根之外，已跳过该 skill
退出码：1
```

前两条是资源路径逃逸（访问时被拒）；第三条是**整个 skill** 逃逸 —— 客户端会跳过这个 skill，插件其余部分照常加载。这就是最难自己发现的那类问题。

## 退出码

| 码 | 含义 |
| --- | --- |
| `0` | 本工具能确定的包侧 MUST 全部通过。`--strict` 下还包括没有 advisory、没有未检查的静态项。 |
| `1` | 规范失败 —— 或 `--strict` 失败。细节都在报告里。 |
| `2` | 工具没能跑完：参数、I/O，或它无法表示的输入。这**不是**对包的结论。 |

## 它不做什么

把边界说清楚，比把特性列表写长更重要。

- 它不当客户端。不会加载你的插件、不会跑你的 MCP server、不会测网络。
- 除了规范自己那条「不得把凭证写进 `headers` / `env`」，它不做安全审计。
- 它不检查宿主私有目录、marketplace 索引，或任何宿主自己定义的东西。
- 它不修你的包。

遇到过但验不了的东西，会在报告里标成 `manual` 或 `runtime`，而不是悄悄丢掉。

## 建立在证据上

规则、它们的爆炸半径，以及背后的理由，都写在仓库里：

- [`rules.md`](rules.md) —— 规则索引，每条映射到它所执行的规范章节。
- [`DESIGN.md`](DESIGN.md) —— 设计，包括那些**故意留着没定**的部分。
- [`REVIEW.md`](REVIEW.md) —— 独立验证记录：跑了什么、什么被退回重做、什么仍未验证。
- [`research/PROVENANCE.md`](research/PROVENANCE.md) —— 所有依赖的规范正文与 schema 的校验和与来源。

未验证就写未验证。这整件事的意义就在这里。

## 平台支持

| 平台 | 状态 |
| --- | --- |
| Linux | 已验证 |
| macOS | 未验证 |
| Windows | 未验证 |

**只有 CI 在某个平台上跑绿了，README 才会声称支持它。**

## 相关

- [`agent-skills-lint`](crates/agent-skills-lint) —— 本工具所基于的 SKILL.md 解析与校验库。它是 [`AkaraChen/skills-ref`](https://github.com/AkaraChen/skills-ref) 的接班，迁移说明见 [MIGRATION.md](crates/agent-skills-lint/MIGRATION.md)。
- [Agent Plugins 规范](https://agent-plugins.org) —— 被检查的包格式本身。
- [Agent Skills 规范](https://agentskills.io) —— `SKILL.md` 的格式。
- [Model Context Protocol](https://modelcontextprotocol.io) —— `mcp.json` 指向的那些 server。

## 许可证

代码 MIT。crate 内嵌的官方规范 schema 为 Apache-2.0，逐文件署名见 [`crates/agent-plugin-lint/THIRD_PARTY.md`](crates/agent-plugin-lint/THIRD_PARTY.md)。
