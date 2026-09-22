# Plan 002: Close MCP coverage gaps when a server is skipped or empty

> **Executor instructions**: This plan is already landed. Do not re-apply it.
> Drift against `05d65d4` in the in-scope files is this fix.
>
> **Drift check**: `git diff --stat 05d65d4..HEAD -- crates/agent-plugin-lint/src/mcp.rs crates/agent-plugin-lint/tests/mcp_s4.rs`

## Status

- **Priority**: P1
- **Effort**: M
- **Risk**: LOW
- **Depends on**: none
- **Category**: bug
- **Planned at**: commit `05d65d4`, 2026-09-22

## Why this matters

`mcp::scan` returned as soon as one server check failed. Later checks for that
server were never recorded, so `ensure_registry_coverage` filled them with
`unchecked` / `RULE_NOT_EVALUATED`. That reason is reserved for rules that have
no target-level scanner evidence (`lib.rs`). It is the wrong status when the
scanner did run and then stopped:

- `command: "../bin/server"` failed `AP-MCP-COMMAND` and left cwd / reserved-env unrecorded.
- `http://api.example/x` failed `AP-MCP-HTTPS` without recording that the URL form itself was acceptable, and without blocking header checks.
- An accepted `mcpServers: {}` looked like the command/URL rules were unimplemented, unlike an absent `mcp.json`, which is already `not-applicable` / `MCP_ABSENT`.

Exit codes do not change: blocked and not-applicable coverage are not strict failures, and the existing MUST findings still exit 1. Consumers of coverage stop mistaking a skipped check for a missing implementation.

## Current state

Evidence: `crates/agent-plugin-lint/src/mcp.rs:326` (empty `mcpServers`), `crates/agent-plugin-lint/src/mcp.rs:635` (`URL_VALID` before the HTTPS skip), `crates/agent-plugin-lint/src/mcp.rs:1128` (`record_absent`), `mcp.rs:486`, `mcp.rs:819`, `mcp.rs:877`, `mcp.rs:887`, tests from `crates/agent-plugin-lint/tests/mcp_s4.rs:259`.

- `crates/agent-plugin-lint/src/mcp.rs`
  - `envelope` calls `mark_no_servers` when the server map is empty (`mcp.rs` around the `servers.is_empty()` branch).
  - `record_absent` writes a coverage row only when that rule and target are not already present, so a Fail/Pass/Unchecked row is never overwritten.
  - `record_stdio_remote_absence` marks URL/HTTPS/headers `not-applicable` / `STDIO_NO_REMOTE` once a stdio variant is valid.
  - `record_remote_stdio_absence` marks command, cwd, env, and relative-form `not-applicable` / `REMOTE_NO_STDIO` once a remote variant is valid.
  - `block_stdio_remainder` / `block_cwd` / `block_headers` record `blocked` for checks that a failure return skips.
  - `UrlCheck::HttpsRequired` records `AP-MCP-URL` pass / `URL_VALID` before the existing `AP-MCP-HTTPS` skip.
- Tests in `crates/agent-plugin-lint/tests/mcp_s4.rs`:
  - `empty_server_map_is_not_applicable_rather_than_unevaluated`
  - `rejected_stdio_command_blocks_the_checks_that_did_not_run`
  - `reserved_env_blocks_cwd_instead_of_leaving_it_unevaluated`
  - `https_failure_records_the_url_form_and_blocks_headers`
  - `unresolved_dot_command_records_command_coverage_without_a_must` — `./missing` records `AP-MCP-COMMAND` unchecked / `PATH_UNRESOLVED` at `mcp.rs:486`. No new MUST finding.
  - `invalid_cwd_form_blocks_server_escape` — cwd `../x` keeps the `AP-MCP-CWD-FORM` exit 1 and blocks `AP-PATH-SERVER-ESCAPE` at `mcp.rs:819`.
  - `unresolved_cwd_records_cwd_form` and `escaped_cwd_symlink_records_cwd_form_as_blocked` — `./missing` cwd is unchecked / `PATH_UNRESOLVED` (`mcp.rs:877`); a symlink cwd outside the root blocks `AP-MCP-CWD-FORM` / `SERVER_OUTSIDE_ROOT` (`mcp.rs:887`).

D5 fixtures still require `http://api.example/x` to fail `AP-MCP-HTTPS` / `MCP_HTTPS` at exit 1. The extra URL pass row uses a different rule id on the same target.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Focused tests | `cargo test -p agent-plugin-lint --locked --offline --test mcp_s4 -- empty_server_map` | pass (repeat for the other three test names) |
| D5 regression | `cargo test -p agent-plugin-lint --locked --offline --test d5_matrix` | pass |
| Lint | `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 |

## Scope

**In scope**
- `crates/agent-plugin-lint/src/mcp.rs`
- `crates/agent-plugin-lint/tests/mcp_s4.rs`

**Out of scope**
- `ensure_registry_coverage` and the `RULE_NOT_EVALUATED` exception in `finish_report`. Placeholders remain for rules that still have no target row (a package with no `mcp.json` must keep `AP-PATH-RELATIVE-FORM` / `RULE_NOT_EVALUATED` on target `plugin`; `tests/s5a.rs` `registry_placeholders_are_honest_unchecked_but_do_not_make_strict_fail`).
- Inventing MUST findings for headers or cwd after the server is already invalid.
- `scripts/review/*`. They are external oracles; do not edit them to match a weaker result.

## Git workflow

Do not commit or push.

## Steps

### Step 1: Record absence and blocked tails

Add `record_absent` and the helpers named in Current state. Call them from:

- empty `mcpServers` after a valid envelope
- stdio returns for a bad relative command, outside command, command I/O, and reserved env
- remote returns for an invalid URL (block HTTPS and headers) and for HTTPS-required (pass the URL form, block headers)

Do not call `record_absent` when a row for that rule and target already exists.

**Verify**: the four `mcp_s4` tests and `cargo test -p agent-plugin-lint --locked --offline --test d5_matrix` pass.

## Test plan

Each new test calls `lint_path` on a temp plugin (the same helper as the existing `mcp_s4` tests):

- empty map: exit 0; `AP-MCP-COMMAND`, `AP-MCP-URL`, `AP-MCP-HEADERS`, `AP-PATH-RELATIVE-FORM` are `not-applicable` / `NO_SERVERS` on target `mcp.json`
- `../bin/server`: exit 1; `AP-MCP-COMMAND` finding remains; cwd and reserved-env rows are `blocked`; URL is `STDIO_NO_REMOTE`
- `PLUGIN_ROOT` env: exit 1; cwd is `blocked` / `RESERVED_ENV`
- `http://api.example/x` plus a token-like header: exit 1; `AP-MCP-HTTPS` finding; `AP-MCP-URL` pass / `URL_VALID`; headers `blocked` / `MCP_HTTPS`; no `AP-ADVICE-POSSIBLE-SECRET` (the server was not accepted); command `REMOTE_NO_STDIO`

## Done criteria

- [x] the four new tests pass
- [x] `d5_matrix` passes, including `http-public`
- [x] a plugin with no `mcp.json` still has `AP-PATH-RELATIVE-FORM` / `RULE_NOT_EVALUATED` under strict and still exits 0
- [x] `record_absent` does not overwrite an existing row

## STOP conditions

- A D5 fixture's expected rule, target, status, or reason disappears.
- Strict exit changes for `http://127.1`, `${PLUGIN_DATA}` cwd, or a minimal plugin with no `mcp.json`.
- The fix seems to require editing `finish_report`'s strict predicate.

## Maintenance notes

New per-server checks must either record their own row on every return path or go through `record_absent` / `block_stdio_remainder`. A bare `return` after a finding recreates the placeholder bug.
