# astra 独立验证记录

本记录区分源码阅读、实际执行与未验证范围；实现 agent 的总结不作为通过凭证。

## A：立项与证据

- 仓库 https://github.com/AkaraChen/agent-plugin-lint 已创建并克隆；`gh repo view --json visibility,licenseInfo,url` 验证 PUBLIC + MIT。
- 本机实际 `cargo 1.97.1`、`rustc 1.97.1`；不沿用背景材料中的版本数字。
- 骨架执行 `cargo test --workspace`、`cargo clippy --workspace --all-targets -- -D warnings` 通过；当时是 **0 测试**，仅证明骨架编译，不作为规则验收。
- 两份 schema 按 `SHA1("blob " + 字节数 + NUL + 正文)` 与索引核对通过；原文及 research 既有文件复制前后摘要相同。
- 91 个规则 ID 唯一；AP §1–11 中 74 个带要求关键词的源行都有规则索引映射；这不是 74 项执行测试。
- 已检查 AP §4.1 的最窄失败边界、§5.2/§11.3 两个非 fatal 例外、§7.1 直接发现、§7.2.2 envelope/entry 隔离、§8/§8.1 扩展处理。实现验证分片记录如下。

## 真实语料版本

`eric-way` 默认 main `147fe6b840aab1f183cfd22d9f5cb61a14c35d63` 已无 plugins 目录。本次新克隆检出设计审计快照 `37d007ca28263d182aab6a5cc62d1bbda246fdfb`；子模块 guided-review 锁定 `05a5924108a6a81ce926c23f053a7d74528d5a62`。没有改语料内容。

使用 Python pathlib 严格 resolve、按目录边界比较，已独立核实 11 plugin、24 直接 skill 候选、0 mcp.json、3 外链：

| 路径 | §4.1 要求的后果 |
|---|---|
| plugins/frontend/skills/e2e-testing/references/docker.md | ignored / deny-path，访问时拒绝 |
| plugins/review/skills/github-pr/references/viewed-state.md | ignored / deny-path，访问时拒绝 |
| plugins/review/skills/guided-review | component / skip-skill，结合 §7.1 跳过该 skill |

此段是文件系统事实核实；引擎结果另记，不将 Python 脚本充当引擎验收。

## 尚未验证

引擎各切片待 review。Windows/macOS、实际宿主加载、安装器产物、MCP 启动/连接/认证/握手、PLUGIN_DATA 生命周期、秘密真实性、域名控制权、完整客户端符合性、发布安装尚未验证。

## S1 首轮：退回

astra 独立执行 workspace test（10 个测试通过）和 clippy（无警告）后，另作 6 个黑盒库 API 回归，**6 个失败**：解析失败后字段 coverage=pass；metadata 数字键被字符串化；属性提取丢弃非法 metadata 值；行数预算漏算 frontmatter；SKILL.md 内部链接被拒；非 UTF-8 目录名被替为空串制造 mismatch。已将完整复现交回 terra。依据 AP §7.1 引用的 AS 格式/metadata/Progressive disclosure，以及 §7.1 的 resolves to regular file；覆盖状态与 IO 分类是本工具正确性契约。此轮不接受 S1，不进入 S2。

S1 第二轮：astra 再跑 20 个集成测试和 clippy 通过；六个旧复现已修复。新增复现仍有两失败：布尔 metadata 键被字符串化；frontmatter unchecked 后字段仍 pass。独立最小依赖实验确认 `deserialize_any` 可严格区分数值/布尔键与带引号或 `!!str` 的字符串键；退回 terra 移除手写事件补丁并修正 coverage。仍未接受 S1。

S1 第三轮接受：astra 独立执行 `cargo test --workspace`（23 个集成测试通过）、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo fmt --all -- --check`、`git diff --check`。另将最初未改动的 6 个 probes 和后续 2 个失败 probes + 1 个依赖实验重新加入临时测试并执行，9/9 通过后移除临时文件。已核对 `deserialize_any` 严格键类型、解析未完成时 dependent coverage=blocked、全文件行数、合法链接、非 UTF-8 路径与原类型错误修复。常规 Unicode/NFKC、未知字段、重复/非字符串 YAML 键仍明确 unchecked；token 预算 manual，位置不猜测。AP §7.1 引用 AS 格式要求；范围不包含完整宿主加载。

## S2a：manifest 纯库

两次旧执行会话提前停止后，换新 terra 会话继续同片。astra 独立执行 34 个集成测试和 clippy 通过；核对 §5.2 两个非 fatal 例外、§5.3 required 抑制、§5.4 类型与禁止格式拒绝、§5.5 名称、§10.2 SemVer 建议、§8.1 未知扩展不定罪。两份内嵌 schema 与研究快照字节、固定 SHA-256 均通过。独立 11 个 probes 有 9 pass、2 fail（obligation 大小写和 rule ID 字典序），已退回修复输出契约，尚未接受此片。

S2a 修订接受：astra 重跑 workspace 的 36 个集成测试、clippy、fmt，均通过；独立 11 probes 原样复验全过。输出 obligation 大写、finding 按序列化 rule ID 字典序排序、manifest 专用规则集合隔离已核实。两份内嵌 schema 再次与研究快照逐字节及 SHA-256 核对一致。此片仅纯库，没有声称 CLI/文件系统/组件检查已执行。

## S2b 首轮：退回

astra 实际执行 46 个测试、clippy、fmt、build，以及 13 个 CLI 用例和 15 个文件系统初版用例，均通过。进一步探针复现：canonicalize 权限错误误作 manifest fatal（应操作错误 2）；参数错误会扫描无关 cwd 并留下残余 summary；manifest 早退缺 blocked coverage；非 UTF-8 CLI 路径被替换；collection root 未按设计输出相对路径。源码发现重复 mode/spec 被静默覆盖。初版重复 mode 用例碰巧因空集合报 2，已强化为有效包 + ARGUMENT 错误断言，避免把测试自身的弱断言当证据。规范门禁依据 §5.1–5.2，JSON/IO/输出路径是工具契约。退回 terra，不接受 S2b。

S2b 第二轮：astra 实际执行 50 个测试、clippy、fmt、build、13 个 CLI + 15 个强化文件系统 + 6 个复现断言全过。发现 IO 分支 coverage 仍误记 location=fail，要求最后修正；没有把无 finding 等同于覆盖状态准确。

S2b 第三轮接受：astra 独立执行 51 个测试、clippy、fmt、build 与强化后的 6 个复现断言，全过；IO coverage 不再伪称规范失败。CLI 的0/1/2、参数/JSON错误、相对root、目录别名、内外manifest链接、FIFO、权限与非UTF8错误已验。读取时文件变化二次核验及组件读取属于S3，尚未验；大小写不敏感文件系统未实机验证，仅检查精确枚举实现。
