//! 固定的官方 schema 快照在本 crate 的 `schemas/` 中随发布产物分发。
/// Agent Plugins 1.0.0 plugin schema 的 canonical ID。
pub const PLUGIN_SCHEMA_ID: &str = "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json";
/// Agent Plugins 1.0.0 MCP schema 的 canonical ID。
pub const MCP_SCHEMA_ID: &str = "https://agent-plugins.org/schemas/1.0.0/mcp.schema.json";
/// 原始 schema 字节随 crate 编入；运行时不请求网络。
pub const PLUGIN_SCHEMA: &str = include_str!("../schemas/plugin.schema.json");
/// 原始 schema 字节随 crate 编入；运行时不请求网络。
pub const MCP_SCHEMA: &str = include_str!("../schemas/mcp.schema.json");
