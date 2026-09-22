use crate::{ReadPropertiesError, SkillIoError, find_skill_md, read_properties};
use std::path::{Path, PathBuf};
fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
/// 生成移植自 `skills-ref` 的 `<available_skills>` XML；所有文本节点均转义。
pub fn to_prompt(directories: &[PathBuf]) -> Result<String, ReadPropertiesError> {
    let mut lines = vec!["<available_skills>".to_owned()];
    for directory in directories {
        let directory = utf8_path(directory)?;
        let p = read_properties(Path::new(directory))?;
        lines.extend([
            "<skill>".to_owned(),
            "<name>".to_owned(),
            escape(&p.name),
            "</name>".to_owned(),
            "<description>".to_owned(),
            escape(&p.description),
            "</description>".to_owned(),
        ]);
        if let Some(path) = find_skill_md(Path::new(directory)) {
            lines.extend([
                "<location>".to_owned(),
                escape(utf8_path(&path)?),
                "</location>".to_owned(),
            ]);
        }
        lines.push("</skill>".to_owned());
    }
    lines.push("</available_skills>".to_owned());
    Ok(lines.join("\n"))
}

fn utf8_path(path: &Path) -> Result<&str, ReadPropertiesError> {
    path.to_str()
        .ok_or_else(|| ReadPropertiesError::Io(SkillIoError::NonUtf8Path(path.to_path_buf())))
}
