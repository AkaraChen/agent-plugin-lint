# Plan 003: Reject non-UTF-8 paths in the skills prompt

> **Executor instructions**: This plan is already landed. Do not re-apply it.
>
> **Drift check**: `git diff --stat 05d65d4..HEAD -- crates/agent-skills-lint/src/prompt.rs crates/agent-skills-lint/src/parser.rs crates/agent-skills-lint/tests/migration_matrix.rs`

## Status

- **Priority**: P2
- **Effort**: S
- **Risk**: LOW
- **Depends on**: none
- **Category**: bug
- **Planned at**: commit `05d65d4`, 2026-09-22

## Why this matters

`to_prompt` used `Path::display`, which substitutes a replacement character for
non-UTF-8 components. DESIGN §2 forbids quietly merging distinct filenames with
a lossy conversion. The linter itself already returns `PATH_ENCODING` instead.
The prompt helper is not on the lint path, but it is a public function of
`agent-skills-lint` and it embeds the path in XML. Two different byte paths
could have rendered as the same location text.

## Current state

Evidence: `crates/agent-skills-lint/src/parser.rs:140`, `crates/agent-skills-lint/src/prompt.rs:15`, `crates/agent-skills-lint/src/prompt.rs:39`, test `crates/agent-skills-lint/tests/migration_matrix.rs:126`.

- `crates/agent-skills-lint/src/parser.rs` — `SkillIoError::NonUtf8Path(PathBuf)` with message `path is not valid UTF-8`. The path is not interpolated into `Display`, so the message itself is not lossy.
- `crates/agent-skills-lint/src/prompt.rs` — `utf8_path` requires `Path::to_str`. `to_prompt` checks the directory before `read_properties`, and checks `find_skill_md`'s path before `escape`.

```rust
fn utf8_path(path: &Path) -> Result<&str, ReadPropertiesError> {
    path.to_str()
        .ok_or_else(|| ReadPropertiesError::Io(SkillIoError::NonUtf8Path(path.to_path_buf())))
}
```

- `crates/agent-skills-lint/tests/migration_matrix.rs` — `prompt_rejects_non_utf8_paths_without_lossy_text` calls `to_prompt` with `OsString::from_vec(b"bad\\xff")` and expects `ReadPropertiesError::Io(SkillIoError::NonUtf8Path(_))` whose path does not decode as UTF-8. No filesystem entry is required, so the test runs on macOS where creating such a name returns `EILSEQ`.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Test | `cargo test -p agent-skills-lint --locked --offline --test migration_matrix prompt_rejects_non_utf8` | pass |
| Existing prompt test | `cargo test -p agent-skills-lint --locked --offline --test migration_matrix directory_missing` | pass |

## Scope

**In scope**
- `crates/agent-skills-lint/src/prompt.rs`
- `crates/agent-skills-lint/src/parser.rs` (`SkillIoError` only)
- `crates/agent-skills-lint/tests/migration_matrix.rs`

**Out of scope**
- `validate_directory`'s existing `NonUtf8Directory` check.
- Lint reports. Plugin scanning already uses `to_str` and `PATH_ENCODING`.
- XML content escaping. `escape` already covers `& < > " '`.

## Git workflow

Do not commit or push.

## Steps

### Step 1: Fail before rendering

Add `NonUtf8Path`. Route both the input directory and the discovered `SKILL.md` path through `utf8_path` inside `to_prompt`. Check the directory before reading it.

**Verify**: `cargo test -p agent-skills-lint --locked --offline --test migration_matrix prompt_rejects_non_utf8` → pass.

## Test plan

The new test calls the public `to_prompt`. The existing `directory_missing_not_directory_case_and_multiple_prompt_matrix` still checks a UTF-8 two-skill prompt and the `&lt;` escape.

## Done criteria

- [x] `prompt_rejects_non_utf8_paths_without_lossy_text` passes
- [x] `to_prompt` no longer calls `Path::display` or `to_string_lossy`
- [x] `cargo test -p agent-skills-lint --locked --offline --test migration_matrix` passes

## STOP conditions

- `SkillIoError` is exhaustively matched in a downstream crate that this repo cannot update. It is not; only this crate matches specific variants.
- A UTF-8 skill directory that previously prompted now returns `NonUtf8Path`.

## Maintenance notes

Any new path interpolated into the prompt XML has to go through `utf8_path`. Do not put the raw `PathBuf` into the `thiserror` display string; `Path`'s display is lossy.
