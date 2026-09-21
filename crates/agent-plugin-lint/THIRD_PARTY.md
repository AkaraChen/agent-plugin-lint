# 随 crate 分发的第三方材料

`schemas/plugin.schema.json` 与 `schemas/mcp.schema.json` 是 Agent Plugins
项目的官方 1.0.0 schema 原字节快照，来源为
`https://github.com/agentplugins/agent-plugins-spec`，采用 Apache License
2.0。完整 Apache-2.0 许可证文本随此 crate 的 `schemas/Apache-2.0.txt`
分发；本 crate 自有代码采用 `LICENSE` 中的 MIT。

schema 仅在本地读取；运行时不会请求该 URL 或下载其他 schema。
