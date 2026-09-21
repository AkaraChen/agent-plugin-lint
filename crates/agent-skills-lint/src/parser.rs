use crate::SkillProperties;
use serde::de::{self, Deserialize, Deserializer, MapAccess, Visitor};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error;

/// YAML 标量和 mapping 的严格中间表示，避免 JSON 投影改变 YAML 键类型。
#[derive(Debug, Clone, PartialEq)]
pub enum YamlValue {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Sequence(Vec<YamlValue>),
    Mapping(BTreeMap<String, YamlValue>),
}
impl YamlValue {
    /// 如果该值是 YAML 字符串，返回其解码后的文本。
    pub fn as_str(&self) -> Option<&str> {
        self.string()
    }

    pub(crate) fn string(&self) -> Option<&str> {
        if let Self::String(v) = self {
            Some(v)
        } else {
            None
        }
    }
    pub(crate) fn mapping(&self) -> Option<&BTreeMap<String, YamlValue>> {
        if let Self::Mapping(v) = self {
            Some(v)
        } else {
            None
        }
    }
}
struct StringKey(String);
impl<'de> Deserialize<'de> for StringKey {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = StringKey;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("字符串 YAML mapping key")
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<StringKey, E> {
                Ok(StringKey(v.to_owned()))
            }
            fn visit_string<E: de::Error>(self, v: String) -> Result<StringKey, E> {
                Ok(StringKey(v))
            }
        }
        d.deserialize_any(V)
    }
}
impl<'de> Deserialize<'de> for YamlValue {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = YamlValue;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("YAML 值")
            }
            fn visit_unit<E: de::Error>(self) -> Result<YamlValue, E> {
                Ok(YamlValue::Null)
            }
            fn visit_none<E: de::Error>(self) -> Result<YamlValue, E> {
                Ok(YamlValue::Null)
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> Result<YamlValue, E> {
                Ok(YamlValue::Bool(v))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<YamlValue, E> {
                Ok(YamlValue::Number(v.to_string()))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<YamlValue, E> {
                Ok(YamlValue::Number(v.to_string()))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> Result<YamlValue, E> {
                Ok(YamlValue::Number(v.to_string()))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<YamlValue, E> {
                Ok(YamlValue::String(v.to_owned()))
            }
            fn visit_string<E: de::Error>(self, v: String) -> Result<YamlValue, E> {
                Ok(YamlValue::String(v))
            }
            fn visit_seq<A: de::SeqAccess<'de>>(self, mut s: A) -> Result<YamlValue, A::Error> {
                let mut v = Vec::new();
                while let Some(x) = s.next_element()? {
                    v.push(x);
                }
                Ok(YamlValue::Sequence(v))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut m: A) -> Result<YamlValue, A::Error> {
                let mut v = BTreeMap::new();
                while let Some((StringKey(k), x)) = m.next_entry()? {
                    v.insert(k, x);
                }
                Ok(YamlValue::Mapping(v))
            }
        }
        d.deserialize_any(V)
    }
}
/// 已解析 frontmatter 和未改动的 Markdown body。
#[derive(Debug, Clone, PartialEq)]
pub struct Frontmatter {
    pub fields: BTreeMap<String, YamlValue>,
    pub body: String,
}
/// 内容错误；不表示文件系统 I/O 错误。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ParseError {
    #[error("缺少 YAML frontmatter")]
    MissingFrontmatter,
    #[error("YAML frontmatter 未结束")]
    UnclosedFrontmatter,
    #[error("YAML frontmatter 语法无效")]
    InvalidYaml,
    #[error("YAML frontmatter 根节点不是 mapping")]
    NotMapping,
    #[error("YAML 合法但无法安全检查")]
    UnsupportedYaml,
}
/// 目录 API 的 I/O 错误。
#[derive(Debug, Error)]
pub enum SkillIoError {
    #[error("路径不存在: {0}")]
    NotFound(PathBuf),
    #[error("不是目录: {0}")]
    NotDirectory(PathBuf),
    #[error("目录名不是有效 UTF-8: {0}")]
    NonUtf8Directory(PathBuf),
    #[error("缺少 SKILL.md: {0}")]
    MissingSkillFile(PathBuf),
    #[error("读取 {path} 失败: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}
/// `read_properties` 明确分离的 I/O 或内容错误。
#[derive(Debug, Error)]
pub enum ReadPropertiesError {
    #[error(transparent)]
    Io(#[from] SkillIoError),
    #[error(transparent)]
    Content(#[from] ParseError),
}
/// 解析完整源码；只把独立的 `---` 行识别为分隔符。
pub fn parse_frontmatter(source: &str) -> Result<Frontmatter, ParseError> {
    let mut offset = 0;
    let mut lines = source.split_inclusive('\n');
    let Some(first) = lines.next() else {
        return Err(ParseError::MissingFrontmatter);
    };
    if !delimiter(first) {
        return Err(ParseError::MissingFrontmatter);
    }
    offset += first.len();
    let start = offset;
    let mut end = None;
    for line in lines {
        let here = offset;
        offset += line.len();
        if delimiter(line) {
            end = Some((here, offset));
            break;
        }
    }
    let Some((yaml_end, body_start)) = end else {
        return Err(ParseError::UnclosedFrontmatter);
    };
    let yaml = &source[start..yaml_end];
    let value = parse_yaml(yaml)?;
    let YamlValue::Mapping(fields) = value else {
        return Err(ParseError::NotMapping);
    };
    Ok(Frontmatter {
        fields,
        body: source[body_start..].to_owned(),
    })
}
fn delimiter(line: &str) -> bool {
    line.trim_end_matches(['\r', '\n']) == "---"
}
fn parse_yaml(source: &str) -> Result<YamlValue, ParseError> {
    let mut options = serde_saphyr::Options::default();
    options.strict_booleans = true;
    serde_saphyr::from_str_with_options(source, options).map_err(classify)
}
fn classify(error: serde_saphyr::Error) -> ParseError {
    match error {
        serde_saphyr::Error::WithSnippet { error, .. } => classify(*error),
        serde_saphyr::Error::Eof { .. }
        | serde_saphyr::Error::ExternalMessage { .. }
        | serde_saphyr::Error::Unexpected { .. }
        | serde_saphyr::Error::UnexpectedSequenceEnd { .. }
        | serde_saphyr::Error::UnexpectedMappingEnd { .. }
        | serde_saphyr::Error::ContainerEndMismatch { .. }
        | serde_saphyr::Error::UnknownAnchor { .. } => ParseError::InvalidYaml,
        serde_saphyr::Error::DuplicateMappingKey { .. }
        | serde_saphyr::Error::SerdeInvalidType { .. } => ParseError::UnsupportedYaml,
        serde_saphyr::Error::SerdeInvalidValue { .. } => ParseError::InvalidYaml,
        _ => ParseError::UnsupportedYaml,
    }
}
/// 精确枚举目录项，避免 case-insensitive 文件系统把异名文件当作 `SKILL.md`。
pub fn find_skill_md(dir: &Path) -> Option<PathBuf> {
    find_skill_md_io(dir).ok().flatten()
}
fn find_skill_md_io(dir: &Path) -> Result<Option<PathBuf>, std::io::Error> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_name() == "SKILL.md" {
            let path = entry.path();
            return fs::metadata(&path).map(|metadata| metadata.is_file().then_some(path));
        }
    }
    Ok(None)
}
/// 从目录读取属性，错误保持 I/O/内容边界。
pub fn read_properties(dir: &Path) -> Result<SkillProperties, ReadPropertiesError> {
    let source = read_source(dir)?;
    Ok(read_properties_from_source(&source)?)
}
/// 从源码提取属性，不做 I/O。
pub fn read_properties_from_source(source: &str) -> Result<SkillProperties, ParseError> {
    let fm = parse_frontmatter(source)?;
    let required = |key: &str| {
        fm.fields
            .get(key)
            .and_then(YamlValue::string)
            .map(str::to_owned)
            .ok_or(ParseError::UnsupportedYaml)
    };
    let string = |key: &str| {
        fm.fields
            .get(key)
            .and_then(YamlValue::string)
            .map(str::to_owned)
    };
    for field in ["license", "compatibility", "allowed-tools"] {
        if fm.fields.contains_key(field) && string(field).is_none() {
            return Err(ParseError::UnsupportedYaml);
        }
    }
    if let Some(metadata) = fm.fields.get("metadata") {
        let Some(mapping) = metadata.mapping() else {
            return Err(ParseError::UnsupportedYaml);
        };
        if mapping.values().any(|value| value.string().is_none()) {
            return Err(ParseError::UnsupportedYaml);
        }
    }
    Ok(SkillProperties {
        name: required("name")?,
        description: required("description")?,
        license: string("license"),
        compatibility: string("compatibility"),
        allowed_tools: string("allowed-tools"),
        metadata: fm
            .fields
            .get("metadata")
            .and_then(YamlValue::mapping)
            .map(|m| {
                m.iter()
                    .filter_map(|(k, v)| v.string().map(|v| (k.clone(), v.to_owned())))
                    .collect()
            }),
    })
}
pub(crate) fn read_source(dir: &Path) -> Result<String, SkillIoError> {
    if !dir.exists() {
        return Err(SkillIoError::NotFound(dir.to_path_buf()));
    }
    if !dir.is_dir() {
        return Err(SkillIoError::NotDirectory(dir.to_path_buf()));
    }
    let path = find_skill_md_io(dir)
        .map_err(|source| SkillIoError::Read {
            path: dir.to_path_buf(),
            source,
        })?
        .ok_or_else(|| SkillIoError::MissingSkillFile(dir.to_path_buf()))?;
    fs::read_to_string(&path).map_err(|source| SkillIoError::Read { path, source })
}
