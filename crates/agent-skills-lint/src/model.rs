use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// skill frontmatter 中已支持字段的强类型投影。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillProperties {
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compatibility: Option<String>,
    #[serde(rename = "allowed-tools", skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<BTreeMap<String, String>>,
}
impl SkillProperties {
    /// 保留所有已支持字段的稳定 JSON 投影。
    pub fn to_dict(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("SkillProperties is serializable")
    }
}
