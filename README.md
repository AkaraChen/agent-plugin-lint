**English** · [中文](README.zh-CN.md)

<h1 align="center">ap-lint</h1>

<p align="center">
	<a href="https://github.com/AkaraChen/agent-plugin-lint/actions/workflows/ci.yml"><img src="https://github.com/AkaraChen/agent-plugin-lint/actions/workflows/ci.yml/badge.svg" alt="CI status"></a>
	<a href="#license"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT license"></a>
</p>

> Linter for [Agent Plugins](https://agent-plugins.org) — it tells you what will actually break

Any linter can tell you a package is invalid. That is the easy half.

The hard half is that "invalid" is not one thing. A package can be invalid in a way that makes a client **reject it outright** — or invalid in a way that makes a single skill **quietly disappear**. In the second case the plugin still installs, nothing prints an error, and one of the capabilities you shipped is simply not there. You find out weeks later, from a user.

`ap-lint` reports the second kind as loudly as the first. Every finding carries the **blast radius** a conformant client will apply, so you always know whether you broke the package or lost a component in it.

**Verified on Linux. macOS and Windows are not verified yet.** See [Platform support](#platform-support).

## Highlights

- **Blast radius, not pass/fail.** Every finding says what a client will actually do about it: reject the plugin, skip this one skill or MCP server, ignore a field, or merely advise.
- **Catches components that vanish silently.** A skill directory that escapes the package root does not error — clients are required to skip it and carry on. `ap-lint` says it out loud.
- **Never guesses.** What it cannot determine statically is reported as `unchecked`, `manual`, or `runtime` — never as a pass. A rule with no target-level check is marked `RULE_NOT_EVALUATED`, which is not a green light.
- **Read-only.** It never writes to the package, never executes anything inside it, and never starts your MCP server.
- **Offline.** No network at run time; the official schemas ship inside the binary. There is no `--fix`, by design — a linter that edits your package is a different tool with a different risk profile.
- **Machine-readable.** A single stably-ordered JSON document for CI, with the same findings the human output shows. No information is text-only.
- **Checks the whole portable floor,** not just the manifest: `plugin.json`, component discovery, every `SKILL.md`, `mcp.json`, package-path containment, and the host-specific leftovers that will not travel.
- **Evidence over confidence.** Every rule maps to a section of the specification, and CI runs the full suite on every platform it claims.

## Install

Until a release is published, install straight from the repository:

```sh
cargo install --git https://github.com/AkaraChen/agent-plugin-lint agent-plugin-lint
```

Or build a checkout:

```sh
git clone https://github.com/AkaraChen/agent-plugin-lint
cd agent-plugin-lint
cargo build --release
```

## Usage

```sh
ap-lint path/to/plugin              # a single plugin
ap-lint path/to/plugins             # a directory of plugins
ap-lint path/to/plugins --json      # machine-readable report
ap-lint path/to/plugins --strict    # also fail on advisories and unchecked rules
```

Flags:

| Flag | Meaning |
| --- | --- |
| `--mode auto\|plugin\|collection` | How to read the path. `auto` refuses to guess when it is ambiguous. |
| `--spec 1.0.0` | Agent Plugins version to check against. Defaults to the version the package declares. |
| `--json`, `--format text\|json` | Output format. |
| `--strict` | Treat advisories and unchecked checks as failures too. |

Diagnostics and CLI help are currently Chinese; English output is planned. The JSON keys, rule IDs, and exit codes are English and stable.

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | No package-side MUST failure this tool can determine. Under `--strict`, also no advisories and no unchecked static checks. |
| `1` | Specification failure — or a `--strict` failure. Details are in the report. |
| `2` | The tool could not complete: bad arguments, I/O, or input it cannot represent. This is not a verdict on the package. |

## What it does not do

Being explicit about the boundary matters more than a longer feature list.

- It does not act as a client. It will not load your plugin, run your MCP server, or test the network.
- It does not audit for secrets beyond the specification's own prohibition on embedding them in `headers` and `env`.
- It does not check client-specific directories, marketplace indexes, or anything a host defines on its own.
- It does not repair your package.

Things it encounters but cannot verify are labelled `manual` or `runtime` in the report rather than silently dropped.

## Built on evidence

The rules, their blast radii, and the reasons behind both are written down in the repository:

- [`rules.md`](rules.md) — the rule index, each one mapped to the specification section it enforces.
- [`DESIGN.md`](DESIGN.md) — the design, including what is deliberately left undecided.
- [`REVIEW.md`](REVIEW.md) — the independent verification log: what was executed, what failed and was sent back, and what remains unverified.
- [`research/PROVENANCE.md`](research/PROVENANCE.md) — checksums and sources for every specification text and schema the tool relies on.

Unverified is stated as unverified. That is the whole point of the exercise.

## Platform support

| Platform | Status |
| --- | --- |
| Linux | Verified |
| macOS | Not verified |
| Windows | Not verified |

The README will only claim a platform once CI is green on it.

## Related

- [`agent-skills-lint`](crates/agent-skills-lint) — the SKILL.md parser and validator this tool is built on. It succeeds [`AkaraChen/skills-ref`](https://github.com/AkaraChen/skills-ref); see its [migration notes](crates/agent-skills-lint/MIGRATION.md).
- [Agent Plugins specification](https://agent-plugins.org) — the package format being checked.
- [Agent Skills specification](https://agentskills.io) — the format of `SKILL.md`.
- [Model Context Protocol](https://modelcontextprotocol.io) — the servers `mcp.json` points at.

## License

MIT. The official specification schemas embedded in the crate are Apache-2.0, with per-file attribution in [`crates/agent-plugin-lint/THIRD_PARTY.md`](crates/agent-plugin-lint/THIRD_PARTY.md).
