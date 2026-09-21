use crate::{ReadPropertiesError, find_skill_md, read_properties};
use std::path::PathBuf;
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
        let p = read_properties(directory)?;
        lines.extend([
            "<skill>".to_owned(),
            "<name>".to_owned(),
            escape(&p.name),
            "</name>".to_owned(),
            "<description>".to_owned(),
            escape(&p.description),
            "</description>".to_owned(),
        ]);
        if let Some(path) = find_skill_md(directory) {
            lines.extend([
                "<location>".to_owned(),
                escape(&path.display().to_string()),
                "</location>".to_owned(),
            ]);
        }
        lines.push("</skill>".to_owned());
    }
    lines.push("</available_skills>".to_owned());
    Ok(lines.join("\n"))
}
