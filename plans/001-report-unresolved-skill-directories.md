# Plan 001: Report unresolved direct skill directories

> **Executor instructions**: This plan is already landed in the worktree. Do not
> re-apply it. The drift check below will show `lint.rs` and `s3.rs` changed
> since `05d65d4`; that diff is this fix. Status is DONE.
>
> **Drift check (run first)**: `git diff --stat 05d65d4..HEAD -- crates/agent-plugin-lint/src/lint.rs crates/agent-plugin-lint/tests/s3.rs`
> If those paths differ from the excerpts below, stop.

## Status

- **Priority**: P1
- **Effort**: S
- **Risk**: LOW
- **Depends on**: none
- **Category**: bug
- **Planned at**: commit `05d65d4`, 2026-09-22

## Why this matters

`scan_skills` used to `continue` when a direct child of `skills/` did not resolve
(`NotFound`, `NotADirectory`, or Unix `ELOOP`). A dangling symlink or a symlink
loop was omitted from findings and coverage, so a package whose only broken
skill entry exited 0 with an empty finding list. Resource symlinks inside a
real skill already emit `AP-ADVICE-UNRESOLVED-PATH`. A direct candidate is the
same situation: it is not an escape (D5 / DESIGN §6: a broken link is not
automatically outside the root) and it is not checked.

## Current state

Evidence: `crates/agent-plugin-lint/src/lint.rs:969` and `crates/agent-plugin-lint/tests/s3.rs:297`.

- `crates/agent-plugin-lint/src/lint.rs` — plugin scan. The unresolved arm records an advisory and does not read the target:

```rust
Ok(crate::containment::Resolution::Unresolved) => {
    add_finding(
        plugin,
        RuleId::AdviceUnresolvedPath,
        logical.clone(),
        Scope::Path(logical),
        "SKILL_DIR_UNRESOLVED",
        "skill directory cannot be resolved; this path was not checked",
    );
    continue;
}
```

`AdviceUnresolvedPath` is advisory, non-normative, obligation none (`rules.rs`).
Default exit stays 0. `--strict` fails because the finding radius is advisory.
A sibling skill that does resolve is still validated.

- Regression: `crates/agent-plugin-lint/tests/s3.rs` `unresolved_skill_directory_is_advisory_not_a_silent_pass` builds a real tree, calls `lint_path`, and checks both a dangling symlink `skills/ghost` and a self-loop `skills/loop`.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Tests | `cargo test -p agent-plugin-lint --locked --offline --test s3 unresolved_skill_directory` | exit 0, the test passes |
| Lint | `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 |

## Scope

**In scope**
- `crates/agent-plugin-lint/src/lint.rs`
- `crates/agent-plugin-lint/tests/s3.rs`

**Out of scope**
- Treating the unresolved directory as `AP-PATH-SKILL-ESCAPE` or any other normative MUST.
- Blocking AS-* rows for that candidate. The existing unresolved `SKILL.md` arm also records only the advisory finding.
- Windows symlink fixtures. The test is `cfg(unix)`, matching `s3.rs`.

## Git workflow

Do not commit or push. The goal forbids both.

## Steps

### Step 1: Record the advisory

In the `Resolution::Unresolved` arm of the direct-child `resolve` match inside `scan_skills`, call `add_finding` as in Current state, then `continue` so the arm does not have to produce a `PathBuf`.

**Verify**: `cargo test -p agent-plugin-lint --locked --offline --test s3 unresolved_skill_directory` → pass.

## Test plan

`unresolved_skill_directory_is_advisory_not_a_silent_pass` calls `lint_path` (the library entry used by the CLI):

- dangling `skills/ghost` and self-loop `skills/loop` produce `AP-ADVICE-UNRESOLVED-PATH` / `SKILL_DIR_UNRESOLVED`
- neither is `AP-PATH-SKILL-ESCAPE`
- `skills/s/SKILL.md` still has `AS-NAME` pass
- default exit 0, strict exit 1

## Done criteria

- [x] `cargo test -p agent-plugin-lint --locked --offline --test s3 unresolved_skill_directory` passes
- [x] default exit for that fixture is 0 and strict exit is 1
- [x] no normative escape finding is added for the unresolved directory

## STOP conditions

- `AdviceUnresolvedPath` metadata is no longer advisory / non-normative. Do not retarget the finding onto a MUST rule.
- The arm cannot `continue` because a later use of `directory` was added. Report that instead of skipping the candidate silently.

## Maintenance notes

A future scanner that starts reading past an unresolved directory must keep the advisory whenever `canonicalize` returns not-found, not-a-directory, or `ELOOP`. Do not collapse that into `SKILL_CANONICALIZE` (that code is the `Err` arm and is exit 2).
