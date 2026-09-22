use crate::Report;

pub fn render(report: &Report) -> String {
    let mut document = serde_json::to_string(report).expect("report serializes");
    document.push('\n');
    document
}
