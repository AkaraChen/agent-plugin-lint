**English** · [中文](README.zh-CN.md)

<h1 align="center">ap-lint</h1>

<p align="center">
	<a href="https://github.com/AkaraChen/agent-plugin-lint/actions/workflows/ci.yml"><img src="https://github.com/AkaraChen/agent-plugin-lint/actions/workflows/ci.yml/badge.svg" alt="CI status"></a>
	<a href="#license"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT license"></a>
</p>

A command-line linter that helps [Agent Plugins 1.0.0](https://agent-plugins.org) authors find configuration errors and paths that escape the plugin root before distribution.

It checks:

- `plugin.json`: required fields, field types, plugin names and specification version.
- `skills/`: skill discovery and `SKILL.md` frontmatter, names and descriptions.
- `mcp.json`: server configuration, commands, working directories, URLs, headers and reserved environment variables.
- Package paths: whether resolved paths and symlink targets stay within the plugin root.

Reports include the location, rule ID, specification section and affected scope: the whole plugin, a component, a field or a path. JSON output also records checks that need manual or runtime verification. See [rules.md](rules.md) for individual rules.

The linter reads local files, runs offline and does not execute plugin code.

Platform verification: Linux verified; macOS and Windows not yet verified.

## Install

Requires Rust and Cargo.

```sh
git clone https://github.com/AkaraChen/agent-plugin-lint
cd agent-plugin-lint
cargo install --path crates/agent-plugin-lint --locked
```

## Usage

```sh
ap-lint path/to/plugin
ap-lint path/to/plugins --mode collection
ap-lint path/to/plugin --json
ap-lint path/to/plugin --strict
```

`--strict` also fails on advisories and unchecked static checks for concrete targets. Run `ap-lint --help` for all options.

Diagnostics and CLI help are currently in Chinese.

Excerpt from scanning the `frontend` plugin in [eric-way](https://github.com/AkaraChen/eric-way):

```text
插件根：.（状态：accepted）
AP-PATH-RESOURCE-ESCAPE skills/e2e-testing/references/docker.md §4.1 RESOURCE_OUTSIDE_ROOT [.] field=/ hint=无：资源路径位于包根之外，访问时会被拒绝
```

This symlink points outside the plugin root. Under §4.1, a client must deny access to this resource path. The scan exits with code `1`.

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | No detected package-side MUST violation or strict-policy failure. |
| `1` | A package-side MUST violation, including an ignored field or denied path; or a strict-policy failure. |
| `2` | The scan could not complete because of an argument, I/O or input-representation error. |

## License

[MIT](LICENSE). The bundled official schemas are Apache-2.0; see [attribution](crates/agent-plugin-lint/THIRD_PARTY.md).
