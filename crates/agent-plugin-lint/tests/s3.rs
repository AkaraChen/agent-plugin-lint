use agent_plugin_lint::{InputMode, LintOptions, lint_path};
use serde_json::json;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

const SCHEMA: &str = "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json";
fn options() -> LintOptions {
    LintOptions {
        mode: InputMode::Plugin,
        strict: false,
    }
}
fn manifest(root: &Path) {
    fs::write(
        root.join("plugin.json"),
        serde_json::to_vec(&json!({"$schema":SCHEMA,"name":"a"})).unwrap(),
    )
    .unwrap();
}
fn skill(dir: &Path, name: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: test\n---\n"),
    )
    .unwrap();
}

#[cfg(unix)]
#[test]
fn containment_matrix_uses_resolved_not_lexical_paths() {
    use std::os::unix::fs::symlink;
    let t = TempDir::new().unwrap();
    let root = t.path().join("p");
    fs::create_dir(&root).unwrap();
    manifest(&root);
    skill(&root.join("skills/a"), "a");
    let outside = t.path().join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("x"), "x").unwrap();
    symlink(&outside, root.join("skills/a/ref")).unwrap();
    let r = lint_path(&root, options());
    assert!(
        r.plugins[0]
            .findings
            .iter()
            .any(|f| f.rule_id.as_str() == "AP-PATH-RESOURCE-ESCAPE" && f.path == "skills/a/ref")
    );
    // A root symlink remains a legal boundary, and a loop inside it is not recursed forever.
    symlink(".", root.join("skills/a/loop")).unwrap();
    let alias = t.path().join("alias");
    symlink(&root, &alias).unwrap();
    assert_eq!(lint_path(&alias, options()).exit_code, 1); // only the deliberately external resource
}

#[cfg(unix)]
#[test]
fn fixed_escape_and_skill_escape_have_distinct_boundaries() {
    use std::os::unix::fs::symlink;
    let t = TempDir::new().unwrap();
    let outside = t.path().join("out");
    skill(&outside.join("s"), "s");
    let fixed = t.path().join("fixed");
    fs::create_dir(&fixed).unwrap();
    manifest(&fixed);
    symlink(&outside, fixed.join("skills")).unwrap();
    let r = lint_path(&fixed, options());
    assert!(r.plugins[0].findings.iter().any(
        |f| f.rule_id.as_str() == "AP-PATH-FIXED-ESCAPE" && f.effect.as_str() == "disable-type"
    ));
    let file = t.path().join("file");
    fs::create_dir(&file).unwrap();
    manifest(&file);
    skill(&file.join("skills/s"), "s");
    fs::remove_file(file.join("skills/s/SKILL.md")).unwrap();
    symlink(outside.join("s/SKILL.md"), file.join("skills/s/SKILL.md")).unwrap();
    let r = lint_path(&file, options());
    assert!(
        r.plugins[0]
            .findings
            .iter()
            .any(|f| f.rule_id.as_str() == "AP-PATH-SKILL-ESCAPE"
                && f.effect.as_str() == "skip-skill")
    );
}

#[cfg(unix)]
#[test]
fn permission_failures_are_tool_errors_not_conformance_findings() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let t = TempDir::new().unwrap();
    let root = t.path().join("skill-locked");
    fs::create_dir(&root).unwrap();
    manifest(&root);
    skill(&root.join("skills/a"), "a");
    fs::set_permissions(
        root.join("skills/a/SKILL.md"),
        fs::Permissions::from_mode(0o000),
    )
    .unwrap();
    let r = lint_path(&root, options());
    assert_eq!(r.exit_code, 2);
    assert!(!r.complete);
    assert!(!r.errors.is_empty());
    assert!(
        !r.plugins[0]
            .findings
            .iter()
            .any(|f| f.path == "skills/a/SKILL.md")
    );
    fs::set_permissions(
        root.join("skills/a/SKILL.md"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();

    let root = t.path().join("mcp-locked");
    fs::create_dir(&root).unwrap();
    manifest(&root);
    let locked = t.path().join("locked-mcp");
    fs::create_dir(&locked).unwrap();
    fs::write(locked.join("file"), "{}").unwrap();
    symlink("../locked-mcp/file", root.join("mcp.json")).unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    let r = lint_path(&root, options());
    assert_eq!(r.exit_code, 2);
    assert!(!r.complete);
    assert!(!r.errors.is_empty());
    assert!(r.plugins[0].findings.is_empty());
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o700)).unwrap();

    let root = t.path().join("resource-locked");
    fs::create_dir(&root).unwrap();
    manifest(&root);
    let s = root.join("skills/a");
    skill(&s, "a");
    let locked = t.path().join("locked-resource");
    fs::create_dir(&locked).unwrap();
    fs::write(locked.join("file"), "x").unwrap();
    symlink("../../../locked-resource/file", s.join("ref")).unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    let r = lint_path(&root, options());
    assert_eq!(r.exit_code, 2);
    assert!(!r.complete);
    assert!(!r.errors.is_empty());
    assert!(
        !r.plugins[0]
            .findings
            .iter()
            .any(|f| f.path == "skills/a/ref")
    );
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o700)).unwrap();
}

#[cfg(unix)]
#[test]
fn unread_skill_prevents_package_skill_aggregate_pass() {
    use std::os::unix::fs::PermissionsExt;
    let t = TempDir::new().unwrap();
    let root = t.path().join("mixed-readable");
    fs::create_dir(&root).unwrap();
    manifest(&root);
    skill(&root.join("skills/good"), "good");
    skill(&root.join("skills/locked"), "locked");
    let locked = root.join("skills/locked/SKILL.md");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    let r = lint_path(&root, options());
    assert_eq!(r.exit_code, 2);
    let plugin = &r.plugins[0];
    assert!(
        plugin
            .coverage
            .iter()
            .any(|c| c.rule_id == "AP-SKILL-CONFORMANCE"
                && c.target == "skills/locked/SKILL.md"
                && c.status == agent_plugin_lint::CoverageStatus::Unchecked)
    );
    assert!(
        plugin
            .coverage
            .iter()
            .any(|c| c.rule_id == "AP-SKILL-CONFORMANCE"
                && c.target == "skills"
                && c.status == agent_plugin_lint::CoverageStatus::Unchecked)
    );
    assert_eq!(
        plugin
            .findings
            .iter()
            .filter(|f| f.rule_id.as_str() == "AP-ADVICE-SKILLS-UNCHECKED")
            .count(),
        1
    );
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o600)).unwrap();
}

#[test]
fn skill_coverage_keeps_library_status_and_narrow_aggregate() {
    let t = TempDir::new().unwrap();
    let root = t.path().join("large");
    fs::create_dir(&root).unwrap();
    manifest(&root);
    let body = "x\n".repeat(496);
    skill(&root.join("skills/s"), "s");
    fs::write(
        root.join("skills/s/SKILL.md"),
        format!("---\nname: s\ndescription: test\n---\n{body}"),
    )
    .unwrap();
    let r = lint_path(&root, options());
    let plugin = &r.plugins[0];
    assert_eq!(r.exit_code, 0);
    assert!(
        plugin
            .findings
            .iter()
            .any(|f| f.rule_id.as_str() == "AS-SIZE-GUIDANCE"
                && f.radius == agent_plugin_lint::Radius::Advisory)
    );
    assert!(
        plugin
            .coverage
            .iter()
            .any(|c| c.rule_id == "AS-SIZE-GUIDANCE"
                && c.status == agent_plugin_lint::CoverageStatus::Manual)
    );
    assert!(
        !plugin
            .coverage
            .iter()
            .any(|c| c.rule_id == "AP-SKILL-CONFORMANCE"
                && c.status == agent_plugin_lint::CoverageStatus::Fail)
    );

    let root = t.path().join("mixed");
    fs::create_dir(&root).unwrap();
    manifest(&root);
    skill(&root.join("skills/bad"), "A--");
    fs::create_dir_all(root.join("skills/unknown")).unwrap();
    fs::write(
        root.join("skills/unknown/SKILL.md"),
        "---\nname: unknown\ndescription: test\ncustom: yes\n---\n",
    )
    .unwrap();
    let r = lint_path(&root, options());
    let plugin = &r.plugins[0];
    assert_eq!(r.exit_code, 1);
    assert!(
        plugin
            .findings
            .iter()
            .any(|f| f.rule_id.as_str() == "AS-NAME")
    );
    assert_eq!(
        plugin
            .findings
            .iter()
            .filter(|f| f.rule_id.as_str() == "AP-ADVICE-SKILLS-UNCHECKED")
            .count(),
        1
    );
    assert!(
        plugin
            .coverage
            .iter()
            .any(|c| c.rule_id == "AP-SKILL-CONFORMANCE"
                && c.status == agent_plugin_lint::CoverageStatus::Fail)
    );

    let names: Vec<_> = plugin
        .coverage
        .iter()
        .filter(|c| c.rule_id == "AS-NAME" && c.target == "skills/bad/SKILL.md")
        .collect();
    assert_eq!(names.len(), 1);
}

#[test]
fn exact_skill_md_name_is_required() {
    let t = TempDir::new().unwrap();
    let root = t.path().join("case");
    fs::create_dir(&root).unwrap();
    manifest(&root);
    fs::create_dir_all(root.join("skills/s")).unwrap();
    fs::write(root.join("skills/s/skill.md"), "ignored").unwrap();
    let r = lint_path(&root, options());
    assert!(
        !r.plugins[0]
            .coverage
            .iter()
            .any(|c| c.target == "skills/s/SKILL.md")
    );
}

#[cfg(unix)]
#[test]
fn unresolved_skill_directory_is_advisory_not_a_silent_pass() {
    use std::os::unix::fs::symlink;
    let t = TempDir::new().unwrap();
    let root = t.path().join("p");
    fs::create_dir(&root).unwrap();
    manifest(&root);
    skill(&root.join("skills/s"), "s");
    symlink("missing-target", root.join("skills/ghost")).unwrap();
    symlink("loop", root.join("skills/loop")).unwrap();

    let report = lint_path(&root, options());
    assert_eq!(report.exit_code, 0, "{report:?}");
    let unresolved: Vec<_> = report.plugins[0]
        .findings
        .iter()
        .filter(|finding| {
            finding.rule_id.as_str() == "AP-ADVICE-UNRESOLVED-PATH"
                && finding.evidence_code == "SKILL_DIR_UNRESOLVED"
        })
        .map(|finding| finding.path.as_str())
        .collect();
    assert!(unresolved.contains(&"skills/ghost"), "{unresolved:?}");
    assert!(unresolved.contains(&"skills/loop"), "{unresolved:?}");
    assert!(
        report.plugins[0]
            .findings
            .iter()
            .all(|finding| finding.rule_id.as_str() != "AP-PATH-SKILL-ESCAPE")
    );
    assert!(report.plugins[0].coverage.iter().any(|coverage| {
        coverage.rule_id == "AS-NAME"
            && coverage.target == "skills/s/SKILL.md"
            && coverage.status == agent_plugin_lint::CoverageStatus::Pass
    }));

    let strict = lint_path(
        &root,
        LintOptions {
            mode: InputMode::Plugin,
            strict: true,
        },
    );
    assert_eq!(strict.exit_code, 1);
}

/// Reproducible real-corpus entry: `AP_LINT_CORPUS=/path/to/plugins cargo test -p agent-plugin-lint --test s3 corpus -- --ignored`.
#[ignore = "requires AP_LINT_CORPUS pointing at the pinned real corpus"]
#[test]
fn corpus_when_supplied_has_expected_discovery_shape() {
    let path =
        std::env::var("AP_LINT_CORPUS").expect("set AP_LINT_CORPUS to the pinned plugin corpus");
    let r = lint_path(
        Path::new(&path),
        LintOptions {
            mode: InputMode::Collection,
            strict: false,
        },
    );
    assert_eq!(r.plugins.len(), 11);
    let candidates: std::collections::BTreeSet<_> = r
        .plugins
        .iter()
        .flat_map(|p| &p.coverage)
        // Only AS coverage proves a skill was safely read and handed to the
        // library. AP blocked records deliberately include skipped skills.
        .filter(|c| c.target.ends_with("/SKILL.md") && c.rule_id.starts_with("AS-"))
        .map(|c| c.target.as_str())
        .collect();
    assert_eq!(candidates.len(), 23); // 24 candidates; one is skipped before safe read.
    let paths: Vec<_> = r
        .plugins
        .iter()
        .flat_map(|p| {
            p.findings.iter().map(move |f| {
                (
                    p.root.as_str(),
                    f.rule_id.as_str(),
                    f.path.as_str(),
                    serde_json::to_value(f.radius).unwrap(),
                    f.effect.as_str(),
                )
            })
        })
        .filter(|(_, id, _, _, _)| id.starts_with("AP-PATH-"))
        .collect();
    assert_eq!(
        paths,
        vec![
            (
                "frontend",
                "AP-PATH-RESOURCE-ESCAPE",
                "skills/e2e-testing/references/docker.md",
                json!("ignored"),
                "deny-path"
            ),
            (
                "review",
                "AP-PATH-RESOURCE-ESCAPE",
                "skills/github-pr/references/viewed-state.md",
                json!("ignored"),
                "deny-path"
            ),
            (
                "review",
                "AP-PATH-SKILL-ESCAPE",
                "skills/guided-review",
                json!("component"),
                "skip-skill"
            ),
        ]
    );
}
