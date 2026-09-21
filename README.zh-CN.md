[English](README.md) · **中文**

<h1 align="center">ap-lint</h1>

<p align="center">
	<a href="https://github.com/AkaraChen/agent-plugin-lint/actions/workflows/ci.yml"><img src="https://github.com/AkaraChen/agent-plugin-lint/actions/workflows/ci.yml/badge.svg" alt="CI status"></a>
	<a href="#许可证"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT license"></a>
</p>

用于 [Agent Plugins](https://agent-plugins.org) 的命令行 linter，可检查多种问题。

检查范围：

- `plugin.json`：必填字段、字段类型、插件名称和规范版本。
- `skills/`：skill 发现，以及 `SKILL.md` 的 frontmatter、名称和描述。
- `mcp.json`：server 配置、命令、工作目录、URL、headers 和保留环境变量。
- 包内路径：解析后的路径及符号链接目标是否留在插件根目录内。

报告包含问题位置、rule ID、规范章节和影响范围：整个插件、组件、字段或路径。JSON 输出还记录需要人工或运行时验证的检查项。各项规则见 [rules.md](rules.md)。

工具读取本地文件，运行时离线，不执行插件代码。

Linux、macOS、Windows 上的 `cargo test` 和 `cargo clippy` 均已在 CI 中通过。

## 安装

需要 Rust 和 Cargo。

```sh
git clone https://github.com/AkaraChen/agent-plugin-lint
cd agent-plugin-lint
cargo install --path crates/agent-plugin-lint --locked
```

## 使用

```sh
ap-lint path/to/plugin
ap-lint path/to/plugins --mode collection
ap-lint path/to/plugin --json
ap-lint path/to/plugin --strict
```

`--strict` 还会因 advisory 和具体目标上未检查的静态项返回失败。全部参数见 `ap-lint --help`。

诊断与 CLI help 是英文。

扫描 [eric-way](https://github.com/AkaraChen/eric-way) 的 `frontend` 插件时，输出节选如下：

```text
plugin
  root: .
  status: accepted

finding
  rule: AP-PATH-RESOURCE-ESCAPE
  path: skills/e2e-testing/references/docker.md
  spec: §4.1
  radius: ignored
  effect: deny-path
  scope: path skills/e2e-testing/references/docker.md
  evidence: RESOURCE_OUTSIDE_ROOT
  message: access to this resource path is denied because it is outside the package root
```

这条符号链接指向插件根目录之外。按 §4.1，客户端必须拒绝访问该资源路径。这次扫描的退出码为 `1`。

## 退出码

| 码 | 含义 |
| --- | --- |
| `0` | 未发现包侧 MUST 违规，也未触发 strict 失败条件。 |
| `1` | 包侧 MUST 违规，包含字段被忽略或路径访问被拒；或触发 strict 失败条件。 |
| `2` | 因参数、I/O 或输入表示能力错误，扫描未能完成。 |

## 许可证

[MIT](LICENSE)。内嵌的官方 schema 使用 Apache-2.0，见[署名说明](crates/agent-plugin-lint/THIRD_PARTY.md)。
