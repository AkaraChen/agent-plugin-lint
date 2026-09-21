use agent_skills_lint::*;

#[test]
fn parse_failure_cannot_pass_fields() {
    let r = validate_source("invalid", "a");
    assert!(
        !r.coverage
            .iter()
            .any(|c| c.rule == SkillRule::Name && c.status == SkillCoverageStatus::Pass)
    );
    assert!(r.has_unchecked());
}
#[test]
fn numeric_metadata_key_cannot_pass() {
    let r = validate_source(
        "---\nname: a\ndescription: ok\nmetadata:\n  12: text\n---\n",
        "a",
    );
    assert!(r.has_errors() || r.has_unchecked(), "{r:?}");
}
#[test]
fn non_string_metadata_key_matrix_never_passes() {
    for key in ["12", "true", "null", "0x10", "*key"] {
        let prefix = if key == "*key" {
            "key: &key metadata\n"
        } else {
            ""
        };
        let r = validate_source(
            &format!("---\nname: a\ndescription: ok\n{prefix}metadata:\n  {key}: text\n---\n"),
            "a",
        );
        assert!(r.has_errors() || r.has_unchecked(), "{key}: {r:?}");
    }
}
#[test]
fn quoted_and_explicit_string_metadata_keys_remain_valid() {
    for key in ["\"12\"", "!!str 12"] {
        let r = validate_source(
            &format!("---\nname: a\ndescription: ok\nmetadata:\n  {key}: text\n---\n"),
            "a",
        );
        assert!(!r.has_errors() && !r.has_unchecked(), "{key}: {r:?}");
    }
}
#[test]
fn unsupported_frontmatter_blocks_dependent_fields() {
    let r = validate_source("---\nname: a\nname: b\ndescription: ok\n---\n", "a");
    assert!(
        !r.coverage
            .iter()
            .any(|c| c.rule == SkillRule::Name && c.status == SkillCoverageStatus::Pass),
        "{r:?}"
    );
    assert!(r.has_unchecked());
}
#[test]
fn full_file_line_budget() {
    let text = format!("---\nname: a\ndescription: ok\n---\n{}", "x\n".repeat(496));
    let r = validate_source(&text, "a");
    assert!(
        r.issues
            .iter()
            .any(|i| i.kind == SkillIssueKind::TooManyLines),
        "{r:?}"
    );
}
#[test]
fn properties_cannot_drop_bad_metadata_value() {
    assert!(
        read_properties_from_source(
            "---\nname: a\ndescription: ok\nmetadata:\n  version: 12\n---\n"
        )
        .is_err()
    );
}
#[cfg(unix)]
#[test]
fn linked_skill_file_is_valid() {
    let t = tempfile::tempdir().unwrap();
    std::fs::write(
        t.path().join("actual.md"),
        "---\nname: a\ndescription: ok\n---\n",
    )
    .unwrap();
    std::os::unix::fs::symlink("actual.md", t.path().join("SKILL.md")).unwrap();
    assert!(read_properties(t.path()).is_ok());
}
#[cfg(unix)]
#[test]
fn invalid_utf8_name_not_content_error() {
    use std::os::unix::ffi::OsStringExt;
    let t = tempfile::tempdir().unwrap();
    let d = t.path().join(std::ffi::OsString::from_vec(vec![0xff]));
    std::fs::create_dir(&d).unwrap();
    std::fs::write(d.join("SKILL.md"), "---\nname: a\ndescription: ok\n---\n").unwrap();
    assert!(validate_directory(&d).is_err());
}
