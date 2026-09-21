use agent_skills_lint::{
    IssueLevel, ParseError, ReadPropertiesError, SkillIssueKind, SkillRule, parse_frontmatter,
    read_properties, to_prompt, validate_source,
};
use tempfile::TempDir;

fn source(name: &str, description: &str) -> String {
    format!("---\nname: {name}\ndescription: {description}\n---\n# body\n")
}

#[test]
fn metadata_is_typed_and_accepted() {
    let report = validate_source(
        "---\nname: skill\ndescription: useful\nmetadata:\n  author: example\n  version: \"1.0\"\n---\n",
        "skill",
    );
    assert!(!report.has_errors());
    assert_eq!(
        report.properties.unwrap().metadata.unwrap()["author"],
        "example"
    );
}

#[test]
fn character_limits_count_decoded_characters() {
    let ok = validate_source(&source("skill", &"汉".repeat(1024)), "skill");
    assert!(!ok.has_errors());
    let too_long = validate_source(&source("skill", &"汉".repeat(1025)), "skill");
    let issue = too_long
        .issues
        .iter()
        .find(|x| x.field.as_deref() == Some("description"))
        .unwrap();
    assert_eq!(issue.kind, SkillIssueKind::TooLong);
    assert_eq!(issue.actual, Some(1025));
}

#[test]
fn unicode_name_is_not_silently_normalized_or_rejected() {
    let report = validate_source(&source("café", "useful"), "cafe\u{301}");
    assert!(!report.has_errors());
    assert!(report.has_unchecked());
    assert!(
        report
            .issues
            .iter()
            .any(|x| x.kind == SkillIssueKind::AmbiguousUnicode)
    );
}

#[test]
fn unknown_field_is_unchecked_not_a_violation() {
    let report = validate_source(
        "---\nname: skill\ndescription: useful\nfuture-field: 1\n---\n",
        "skill",
    );
    assert!(!report.has_errors());
    assert!(
        report
            .issues
            .iter()
            .any(|x| x.kind == SkillIssueKind::UnknownField && x.level == IssueLevel::Unchecked)
    );
}

#[test]
fn yaml_12_boolean_words_are_strings() {
    let report = validate_source("---\nname: on\ndescription: yes\n---\n", "on");
    assert!(!report.has_errors(), "{:#?}", report.issues);
}

#[test]
fn malformed_and_non_mapping_yaml_are_distinct() {
    assert_eq!(
        parse_frontmatter("---\nname: [\n---\n").unwrap_err(),
        ParseError::InvalidYaml
    );
    assert_eq!(
        parse_frontmatter("---\n- name\n---\n").unwrap_err(),
        ParseError::NotMapping
    );
}

#[test]
fn duplicate_or_non_string_yaml_keys_are_not_conversion_whitened() {
    for source in [
        "---\nname: skill\nname: other\ndescription: useful\n---\n",
        "---\n12: value\nname: skill\ndescription: useful\n---\n",
    ] {
        let report = validate_source(source, "skill");
        assert!(!report.has_errors());
        assert!(report.has_unchecked(), "{source:?}: {report:#?}");
    }
}

#[test]
fn only_standalone_delimiters_close_frontmatter_and_body_is_preserved() {
    let source =
        "---\r\nname: skill\r\ndescription: |\r\n  value with --- inside\r\n---\r\nbody\r\n";
    let frontmatter = parse_frontmatter(source).unwrap();
    assert_eq!(
        frontmatter.fields["description"].as_str(),
        Some("value with --- inside\n")
    );
    assert_eq!(frontmatter.body, "body\r\n");
}

#[test]
fn directory_content_and_io_errors_are_separate_and_prompt_escapes() {
    let temp = TempDir::new().unwrap();
    let missing = read_properties(temp.path()).unwrap_err();
    assert!(matches!(missing, ReadPropertiesError::Io(_)));
    let dir = temp.path().join("skill");
    std::fs::create_dir(&dir).unwrap();
    std::fs::write(
        dir.join("SKILL.md"),
        "---\nname: skill\ndescription: [\n---\n",
    )
    .unwrap();
    assert!(matches!(
        read_properties(&dir).unwrap_err(),
        ReadPropertiesError::Content(ParseError::InvalidYaml)
    ));
    std::fs::write(dir.join("SKILL.md"), source("skill", "<tag> & \"quote\"")).unwrap();
    let xml = to_prompt(&[dir]).unwrap();
    assert!(xml.contains("&lt;tag&gt; &amp; &quot;quote&quot;"));
}

#[test]
fn wrong_field_types_are_structured() {
    let report = validate_source(
        "---\nname: 12\ndescription: useful\nmetadata:\n  author: 1\n---\n",
        "skill",
    );
    assert!(
        report
            .issues
            .iter()
            .any(|x| x.rule == SkillRule::Name && x.kind == SkillIssueKind::WrongType)
    );
    assert!(
        report
            .issues
            .iter()
            .any(|x| x.field.as_deref() == Some("metadata.author")
                && x.kind == SkillIssueKind::WrongType)
    );
}
