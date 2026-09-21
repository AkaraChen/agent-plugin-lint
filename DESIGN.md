# agent-plugin-lint：Rust 设计

本轮设计由 astra 负责。已决事项以 [DECISIONS.md](DECISIONS.md) 为唯一真源，本轮明确授权补充：public + MIT、新仓库双 crate、terra 分片实现、旧仓库不归档。原设计的 91 条规则、规范行索引、失败半径和证据链在 rules.md 复用；原 Node 结构及外部报告导入协议废止。

## 1. 输入与边界

目标固定 AP 1.0.0；原文 [research/1.0.0.md](research/1.0.0.md) §1–11 权威，schema 有冲突时正文优先。两份 schema 已再次核对 git blob，原始字节存 research/schemas/，来源摘要见 research/PROVENANCE.md。1.1.0 草案不能自动视为兼容。Agent Skills 采用 research/agent-skills-specification.md 日期快照；其没有发布版本，不虚构冻结的上游版本。

只读、运行时零网络。Cargo 构建下载依赖与运行时分开。解析器使用维护中的 YAML 后端，禁止 serde_yaml 0.9。默认使用 serde、serde_json、thiserror；按实际需要引入 serde-saphyr、url、http、semver，提交 Cargo.lock。禁用任何网络 schema resolver。官方 schema 在 crates/agent-plugin-lint/schemas/ 内随 crate 分发，用 include_str!/include_bytes! 编入 plugin crate；研究快照仍留 research/schemas/ 并对照摘要；可以手写这两份固定 schema 的校验，不实现通用 schema 引擎，不把整份验证的 boolean 映射到整包 fatal。

## 2. Workspace 与模块

```text
crates/agent-skills-lint/              # lib，与 plugin 无反向依赖
  src/lib.rs                         # 稳定公共重导出
  src/model.rs                       # SkillDocument、SkillProperties
  src/parser.rs                      # 完整 YAML、行级 frontmatter 边界
  src/diagnostic.rs                   # SkillRule、SkillIssueKind、SkillIssue、SkillReport
  src/validator.rs                   # 纯数据判定
  src/prompt.rs                      # 移植原库 XML 功能；非校验依赖
crates/agent-plugin-lint/              # lib + ap-lint
  src/lib.rs                         # lint_path(path, options) -> Report
  src/report.rs                      # Finding/Scope/Effect/Coverage/ToolError
  src/rules.rs                       # RuleId 枚举及规则元数据
  src/manifest.rs                    # JSON 投影、白名单、name、类型
  src/discovery.rs                   # plugin/collection、固定位置、直接 skill
  src/containment.rs                 # 文件系统真实路径与读取门禁
  src/skills.rs                      # 安全读取后调用 skill 库，结构化薄适配
  src/mcp.rs                         # envelope -> 各 server；不启动服务
  src/extensions.rs                  # 对象/namespace；不解释未知 value
  src/expansion.rs                   # 单次精确占位符替换
  src/vendor.rs                      # include_str 与身份/字段对照
  src/main.rs                        # 参数、文本/JSON 输出、退出码
```

模块可以按复杂度合并，但职责不混淆。无需 async runtime、动态规则插件或全局可变状态。使用 BTreeMap/排序 Vec 保证确定输出；PathBuf 存内部路径，报告转成插件根下逻辑相对路径。不可用 to_string_lossy 悄悄合并不同文件名；不可表示的路径应给工具错误或明确未检查。

## 3. skill 库的接口与移植

核心 API：`validate_source(source: &str, directory_name: &str) -> SkillReport`。plugin 侧完成发现、包含检查和读取，仅把文本及逻辑目录名传入；不会让 skill 库重新打开越界路径。另保留直接目录校验、read_properties、to_prompt 的便利 API，IO 用独立 Result 错误，不能冒充 YAML 不合法。保留旧实现来源说明及 XML 转义测试，不承诺旧包 ABI/API 完全兼容。

```rust
pub enum SkillRule { Frontmatter, Name, Description, OptionalFields, SizeGuidance }
pub enum SkillIssueKind {
    MissingFrontmatter, UnclosedFrontmatter, InvalidYaml, NotMapping,
    MissingField, WrongType, Empty, TooLong, InvalidCharacters,
    InvalidHyphens, DirectoryMismatch, UnknownField, AmbiguousUnicode,
    UnsupportedYaml, TooManyLines,
}
pub struct SkillIssue {
    pub rule: SkillRule,
    pub kind: SkillIssueKind,
    pub field: Option<String>,
    pub level: IssueLevel,             // Error / Advisory / Unchecked
    pub location: Option<SourceLocation>,
    pub actual: Option<usize>,
    pub limit: Option<usize>,
}
pub struct SkillReport {
    pub properties: Option<SkillProperties>,
    pub issues: Vec<SkillIssue>,
    pub coverage: Vec<SkillCoverage>,
}
```

枚举变体允许在实现中细化，不能退回 `Vec<String>` 或解析诊断文案。rule 映射到已有 AS-*，细节用枚举 kind；错误消息不回显 YAML 源行/值。位置只填解析器能证明的位置，否则 None；field 始终可定位。

修正原库两条实错：metadata 允许且保留；name/description/compatibility 用解码后 `chars().count()`。检查字段真实类型，不 trim 后掩盖原文字符，不把整数隐式转字符串。metadata 是字符串键值映射。compatibility 如提供需 1–500 字符。frontmatter 分隔符必须是独立行，不能 split("---") 截断合法标量；保留 Markdown body。CRLF、块标量、引号内 --- 都有回归。

D6 保守 profile：ASCII 名字确定检查；非 ASCII 长度仍检查，Unicode 字符集/NFKC 分歧标 unchecked，不做静默 NFKC。目录完全相同时匹配通过；非 ASCII 且仅规范化差异时不擅自拒绝。未知 frontmatter 字段没有正文闭合性禁令，标未检查，不照抄旧库白名单定罪。YAML 后端不支持的合法语法/复杂键/重复键歧义不得直接当格式违规；明确语法错误才 InvalidYaml，未知限制标 UnsupportedYaml。500 行建议可单独检查；token 无锁定 tokenizer 记 manual，不估算。描述质量人工检查。

## 4. 报告、半径与政策

复用四半径：Fatal / Component / Ignored / Advisory，序列化小写。正交信息包括 normative、obligation(MUST/SHOULD/RECOMMENDED/NONE)、subject(Package/Client/Publisher/Tool)、confidence(Certain/Heuristic)。

`Scope` 用带数据 enum：Plugin、ComponentType(Skills/Mcp)、Skill(String)、Server(String)、Path(String)；输出仍为 `{kind,id}`。`Effect` 为 RejectPlugin/DisableType/SkipSkill/SkipServer/DenyPath/IgnoreField/Advise，输出 kebab-case。资源逃逸是 ignored + deny-path，不等于可以读它。

Finding 字段：ruleId、spec 数组、radius、normative、obligation、subject、confidence、path、pointer、scope、effect、evidenceCode、message、hint，可选 line/column。规范引用使用 § 对应编号，不凭规则索引猜。JSON pointer 必须 RFC6901 转义。消息中文，不输出 env/header/args 原值、源行、秘密摘要。

Report 字段：schemaVersion=1、toolVersion、rulesetVersion、specVersion=1.0.0、input、mode、policy、complete、plugins、errors、summary、exitCode。PluginReport 含 root/name/declaredSpec/status/findings/coverage。Coverage 枚举 Pass/Fail/NotApplicable/Blocked/Unchecked/Manual/Runtime；必须区分检查未实现、父级阻断和不适用。errors 是操作错误，不能计入规范半径。错误优先退出 2，complete=false。summary 按 finding 数计，不把组件数混进去。

排序：plugin root；finding 按 path/pointer/ruleId/scope/effect；coverage 按 ruleId/target/status；errors 按 path/code。无时间戳。同一输入、平台、版本 JSON 两次运行逐字节相同。稳定消费者依赖枚举、ID、证据码，不能解析人类消息。规范解释变化升 rulesetVersion。

D4 尚待飞鸢正式裁决，本轮可审阅默认：确定的包侧 normative MUST 全部使退出 1，包含 ignored；只有 advisory 返回 0。`--strict` 额外拦截 advisory 与选定静态规则的 unchecked；manual/runtime/not-applicable/blocked 不因本身导致失败。这只是 CI 政策，不提升半径。无找到包、IO/参数/歧义返回 2。

## 5. manifest 与目录发现

manifest 先安全读取，再严格 JSON 和顶层对象。未知顶层键逐个 ignored；非对象 extensions ignored；其他确定违规 fatal 后不读组件（§5.2、§11.3）。metadata 只检查正文类型，不因 URL/email/SPDX/版本格式拒包（§5.4）；SemVer 可给 advisory。内层 author 是闭合集合。plugin name 和 skill name 是两种规则，plugin 可以有点。

`ap-lint <path> [--mode auto|plugin|collection] [--spec 1.0.0] [--json|--format text|json] [--strict]`。无 --fix、--host、SARIF、外部 skills-report。未知参数返回 2。--spec 不覆写 manifest 声明。self-test 可后置，开发用 cargo test 做真实验证，不宣称空自检有效。

auto 先枚举精确 plugin.json（坏 JSON、目录、断链也算入口）；有则单包。否则只看直接子目录，全部有 manifest 才 collection；混合或全无标记返回歧义 2。显式 collection 把所有直接子目录视为包，包括无 manifest 子包；零包报 2。不同逻辑根若解析到同一目标，发现歧义。只扫 eric-way/plugins，不从仓库根递归找包或解析 marketplace。

固定 skills/ 和 mcp.json 缺省正常；存在但类型错只禁对应类型（§6.2）。skill 只发现直接目录中的精确 SKILL.md 普通文件，资源审计递归不等于递归发现（§7.1）。辅助目录可给非规范未发现提示。未知宿主目录不解释。

## 6. 文件系统包含与安全读取

`ResolvedRoot` 保存真实 PathBuf；`PathRole` 区分 Manifest/FixedSkills/FixedMcp/Skill/ServerCommand/ServerCwd/Resource。`Containment` 为 Inside/Outside/Unresolved，后者带原因。外部路径只读取判定所需元数据，不读正文或枚举外部目录。

保留 `link/..` 原始路径交内核 `std::fs::canonicalize`，禁止先 components 折叠或词法 normalize；Rust canonicalize 在本平台实测 link/.. 与内核打开结果一致后方可依赖。比较 canonical Path 的 `starts_with`（组件级），禁止字符串前缀。根可为 symlink，根目标定义包边界。不存在/断链/循环不能一律称逃逸。

边界按 §4.1 最窄处理：manifest→fatal；fixed→禁类型；skill 目录或 SKILL.md→跳单 skill；command/cwd→跳 server；其他资源→拒路径。外链 skill 目录先判真实目标，再只做确定候选所需的 SKILL.md 元数据检查；不得进入其资源树，避免对一个逃逸重复报多个问题。未构成 skill 的目录外链只能资源边界，不凭外链本身捏造被发现 skill。

资源在安全 skill 子树内递归检查文件系统链接，目录真实身份集防循环；同一链接入口先检查包含再去重。断链资源给 unresolved，固定断链给 kind；权限错误给 IO。安全读取可在打开前后重核路径/文件身份，变化返回 InputChanged；仍不是原子快照或沙箱。CLI 对恶意设备/FIFO 不阻塞读，只读已确认普通文件。Windows/junction/挂载/竞态无法在 Linux 测试替代，明确限制。

## 7. MCP 与 extensions

MCP envelope 仅检查顶层；每 entry 独立闭合 union（Stdio/StreamableHttp/Sse）。坏 entry 不扩大到整类，坏 envelope/版本才禁 MCP，skills 继续（§7.2.2、§10.1）。不限制 server key 的 plugin 命名规则。

command 不执行、不展开。./ 可包含空格；裸名含空白或 shell 标点欠定，advisory + unchecked。明确绝对/../ 路径违规。cwd 仅三种正文形式；ROOT 可求真实包含；DATA 未知或混合未知占位符只记 runtime，禁止假造目录或凭 .. 定罪。args/env 始终 opaque。

expansion 对原串一次扫描，每个精确 ROOT/DATA 占位符替换一次；replacement 不再扫描；未知字面保留（§9.2）。command/url/header/env key 不扩展。只禁精确保留 env key；大小写等价依平台，提示而不定罪。

URL 用 url crate 配合原串检查，防宽松修复掩盖空 fragment/userinfo/反斜杠/非绝对形式。无 DNS。HTTP 明确 localhost、标准 127/8、::1 合法；IPv4 缩写、mapped IPv6 等欠定为 unchecked，不假装都安全或都违规。headers 用 http::HeaderName/HeaderValue，跨大小写重复为 entry 错；值不做替换，不因单独 Authorization 键就断言秘密。秘密候选只能非规范 advisory，真实性 manual。

extensions 非对象 ignored；unknown namespace value 整体不检查（§8.1），coverage manual/UNIMPLEMENTED_NAMESPACE。namespace 仅明确空/路径分隔符非法才 fatal；schema 未规定正则，其他语法边缘 unchecked。不要求 manifest/同名目录互相存在（§8、§8.2），不递归私有扩展。

## 8. 切片与 review 门槛

| 切片 | terra 实现范围 | astra 必须独立验证 |
|---|---|---|
| S1 | skill 库完整移植、结构化诊断、两 bug、YAML 后端 | 原有 parser/validator/prompt 测试的语义迁移；400 汉字、1024/1025、metadata、字符 name/compatibility、frontmatter/类型/未知字段 |
| S2a | report/rules/vendor/manifest 纯库 | cargo test/clippy；§5.2 两例外、author、name、metadata 禁误报 |
| S2b | CLI、文件读取门禁、报告汇总 | cargo test/clippy；退出码、参数、发现入口、JSON 稳定 |
| S3 | discovery/containment/skill 编排 | cargo test/clippy；§4.1 全边界、link/..、循环、内部链接、真实 3 条 symlink |
| S4 | MCP、expansion | cargo test/clippy；三 variant 矩阵、envelope/entry 隔离、URL/header/cwd/env、不回显值 |
| S5 | extensions、收口文档/覆盖 | cargo test/clippy；§8 未知 value 不检及独立存在；真实语料、离线/只读检查、完整验证记录 |

每片由 astra 验收后才能继续；缺陷退回同一 terra agent 修复。astra 负责提交与推送。每条已选包侧静态规则需要正反例或显式未实现状态；client/publisher/runtime 不能靠假包 fixture 宣称验证。91 条索引不是 91 条可静态证明的规则。

真实靶子 HEAD 见 PROVENANCE。期望 11 plugin、24 skill 候选、0 MCP；两 references 逃逸 ignored/deny-path，guided-review 逃逸 component/skip-skill，共三条确定路径逃逸。若新 HEAD 语料变化先如实记录，不修改它来迎合预期。最终报告区分源码树、安装产物、MCP 静态配置与真实连接；未启动服务不能称连接验过。

## 9. 待飞鸢裁决与非目标

D2′ 结构化接口由本设计落地，发布稳定性仍需 review。D3 unknown extension value 的作者侧合规裁决；D4 默认 ignored MUST 退出 1；D5 token/loopback/env 大小写接受域；D6 无版本 AS 快照及 Unicode/NFKC 裁决；D7 宿主矩阵与二进制/crates/skill 发行范围；D8 支持平台承诺。当前保守实现不将这些待决解释伪称已拍板。

本轮范围包含可运行引擎与 public 源码仓库，不含 crates.io 发布、宿主矩阵、SARIF、--fix、旧库归档、改 eric-way 或安装测试。Linux 实测范围以 REVIEW.md 为准。
