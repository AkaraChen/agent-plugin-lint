use agent_skills_lint::*;
use tempfile::tempdir;

fn report(yaml: &str) -> SkillReport {
    validate_source(&format!("---\n{yaml}\n---\nbody\n"), "skill")
}
fn has(r: &SkillReport, kind: SkillIssueKind, field: &str) -> bool {
    r.issues
        .iter()
        .any(|i| i.kind == kind && i.field.as_deref() == Some(field))
}

#[test]
fn required_and_ascii_name_matrix() {
    for (yaml, kind, field) in [
        ("description: ok", SkillIssueKind::MissingField, "name"),
        ("name: skill", SkillIssueKind::MissingField, "description"),
        ("name: ''\ndescription: ok", SkillIssueKind::Empty, "name"),
        (
            "name: Skill\ndescription: ok",
            SkillIssueKind::InvalidCharacters,
            "name",
        ),
        (
            "name: -skill\ndescription: ok",
            SkillIssueKind::InvalidHyphens,
            "name",
        ),
        (
            "name: skill--x\ndescription: ok",
            SkillIssueKind::InvalidHyphens,
            "name",
        ),
    ] {
        assert!(has(&report(yaml), kind, field), "{yaml}");
    }
}
#[test]
fn name_and_text_limits_matrix() {
    assert!(!has(
        &report(&format!("name: {}\ndescription: ok", "a".repeat(64))),
        SkillIssueKind::TooLong,
        "name"
    ));
    assert!(has(
        &report(&format!("name: {}\ndescription: ok", "a".repeat(65))),
        SkillIssueKind::TooLong,
        "name"
    ));
    assert!(
        !report(&format!(
            "name: skill\ndescription: {}\ncompatibility: {}",
            "汉".repeat(1024),
            "汉".repeat(500)
        ))
        .has_errors()
    );
    assert!(has(
        &report(&format!(
            "name: skill\ndescription: ok\ncompatibility: {}",
            "汉".repeat(501)
        )),
        SkillIssueKind::TooLong,
        "compatibility"
    ));
}
#[test]
fn optional_field_and_metadata_type_matrix() {
    for field in ["license", "compatibility", "allowed-tools"] {
        assert!(has(
            &report(&format!("name: skill\ndescription: ok\n{field}: 1")),
            SkillIssueKind::WrongType,
            field
        ));
    }
    assert!(has(
        &report("name: skill\ndescription: ok\nmetadata: 1"),
        SkillIssueKind::WrongType,
        "metadata"
    ));
    assert!(has(
        &report("name: skill\ndescription: ok\nmetadata:\n  version: 1"),
        SkillIssueKind::WrongType,
        "metadata.version"
    ));
}
#[test]
fn directory_missing_not_directory_case_and_multiple_prompt_matrix() {
    let temp = tempdir().unwrap();
    assert!(matches!(
        validate_directory(&temp.path().join("missing")),
        Err(SkillIoError::NotFound(_))
    ));
    let file = temp.path().join("file");
    std::fs::write(&file, "x").unwrap();
    assert!(matches!(
        validate_directory(&file),
        Err(SkillIoError::NotDirectory(_))
    ));
    let lower = temp.path().join("lower");
    std::fs::create_dir(&lower).unwrap();
    std::fs::write(lower.join("skill.md"), "x").unwrap();
    assert!(matches!(
        validate_directory(&lower),
        Err(SkillIoError::MissingSkillFile(_))
    ));
    assert_eq!(
        to_prompt(&[]).unwrap(),
        "<available_skills>\n</available_skills>"
    );
    for name in ["one", "two"] {
        let d = temp.path().join(name);
        std::fs::create_dir(&d).unwrap();
        std::fs::write(
            d.join("SKILL.md"),
            format!("---\nname: {name}\ndescription: {name}\n---\n"),
        )
        .unwrap();
    }
    let xml = to_prompt(&[temp.path().join("one"), temp.path().join("two")]).unwrap();
    assert_eq!(xml.matches("<skill>").count(), 2);
}

#[cfg(unix)]
#[test]
fn prompt_rejects_non_utf8_paths_without_lossy_text() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    use std::path::PathBuf;
    let path = PathBuf::from(OsString::from_vec(b"bad\xff".to_vec()));
    match to_prompt(&[path]).unwrap_err() {
        ReadPropertiesError::Io(SkillIoError::NonUtf8Path(path)) => {
            assert!(path.to_str().is_none());
        }
        other => panic!("expected a non-UTF-8 path error, got {other:?}"),
    }
}
