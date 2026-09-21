# Agent Plugins 1.0.0 规则总表

本表复用前轮 91 个规则 ID、失败半径、正反例及 RFC2119 原文索引；已决事项以 DECISIONS.md 为唯一真源。Rust 结构与执行契约见 DESIGN.md，实际验证状态见 REVIEW.md；表项不等于已实现。待决边缘按保守未检查策略处理。

## 阅读约定

- AP 规范来源均为本地 [research/1.0.0.md](research/1.0.0.md)，`L` 为该文件行号；§1–11 有规范效力，Appendix A / Design Decisions 没有。`research/rules.txt` 不是权威。
- `normative=true` 指规范要求，包括 MUST、SHOULD、RECOMMENDED 及它们引用的表格；并不表示可静态判定。`false` 指工具策略或启发式。`obligation` 为 `MUST / SHOULD / RECOMMENDED / NONE`；MUST NOT 归入 MUST，并在判定中写出禁止事项。
- 主体与检测模式：`P/S` 包/静态；`P/D` 包/同 workspace skill 库判定；`P/M` 包/人工或运行时；`C/T` 客户端/契约测试（包扫描不能证明）；`U/M` 上游发布方/人工。模式是规则元数据，不能将未检测记作通过。
- 半径是违规的影响：`fatal` 整包；`component` 组件类型或单项；`ignored` 字段被忽略或单一路径访问被拒；`advisory` 不存在已规定的包加载失败边界。客户端/发布方规则落 advisory 是本工具的记账选择，不声称规范给它们规定了此半径。
- `component` 必须另带 scope：`component-type:skills`、`component-type:mcp`、`skill:<name>` 或 `server:<key>`。缺少标准组件、宿主不支持组件、未知扩展不是包违规，表中对应条目是负向保护测试，正常不产生 finding。
- 例子默认嵌入其余有效的最小包。`P0` 为 `{"$schema":"https://agent-plugins.org/schemas/1.0.0/plugin.schema.json","name":"a"}`；`M0` 为 `{"$schema":"https://agent-plugins.org/schemas/1.0.0/mcp.schema.json","mcpServers":{}}`。`R` 是真实 plugin root，`D` 是客户端数据目录；不存在的 D 不能由校验器创建。
- “正例/反例”对 C/T、U/M 表示合规/不合规的实现行为，不是伪装成能从包中发现的问题。明确违规才输出 `confidence=certain`；推测只输出独立的非规范 advisory ID。

## L0：manifest 与失败入口

| rule id | spec §（原文行） | 失败半径 | 是否 normative / obligation；主体/检测 | 检测方法（含边界） | 正例 | 反例 |
|---|---|---|---|---|---|---|
| AP-MANIFEST-LOCATION | §4.1(2), §5.1 L61,133–137 | fatal | true/MUST；P/S | 指定包根必须有精确名称 plugin.json；lstat 区分缺失与断链；必须可按普通文件读取。仅宿主私有 manifest 不能替代。访问权限失败属工具 IO，不能冒充包格式违规。 | R/plugin.json=P0 | 仅 .claude-plugin/plugin.json |
| AP-MANIFEST-JSON | §5.2 L143,147 | fatal | true/MUST；P/S | 严格 JSON，解析后是非 null、非数组的对象。无注释、无尾逗号；BOM/重复键按 DESIGN 的工具边界处理，不推断额外 AP 要求。 | P0 | `[]`、`null`、截断 JSON |
| AP-MANIFEST-UNKNOWN-FIELD | §5.2, §11.3 L143–147,546 | ignored | true/MUST；P/S | 每个不在十字段白名单的顶层键各报一次；不得赋予发现/加载语义。忽略它后继续验证已知字段，不能因此 fatal。 | P0 加 description | P0 加 `skills:["other/"]`，仅此字段被忽略 |
| AP-MANIFEST-REQUIRED | §5.3 L191–196 | fatal | true/MUST；P/S | $schema/name 缺失、非字符串、空字符串；按 JSON pointer 各报一次；被此项拦截的字段不再报字符/版本错误。 | P0 | 缺 name；name:3 |
| AP-MANIFEST-SCHEMA-ID | §5.2 L151–153 | fatal | true/MUST；P/S | 必须逐字等于 1.0.0 canonical plugin 标识；不请求 URL、不归一化、不以 --spec 改写。未知/其他版本报告 declared 值的安全摘要。 | P0 的 $schema | http 替代 https；1.1.0；查询参数 |
| AP-MANIFEST-METADATA-TYPE | §5.2, §5.4 L147,200–212 | fatal | true/MUST；P/S | version/description/homepage/repository/license 为字符串；keywords 为字符串数组；author 单独检查。可选字段缺失允许；空字符串、空数组不因额外偏好拒绝。 | version:"nightly", keywords:[] | version:1；keywords:[1] |
| AP-MANIFEST-AUTHOR | §5.4 L204,210 | fatal | true/MUST；P/S | author 若存在为非 null、非数组对象，只含 name/email/url 且值均为字符串；嵌套未知键不享受顶层例外。 | author:{}；email:"local" | author:"Eric"；author:{github:"x"} |
| AP-NAME-LENGTH | §5.5 L216–220 | fatal | true/MUST；P/S | name 长 1–64；有效字符是 ASCII，长度单位在有效域无分歧；空值由 REQUIRED 报。 | a；64 个 a | 65 个 a |
| AP-NAME-CHARSET | §5.5 L216–223 | fatal | true/MUST；P/S | 仅 a-z、0-9、连字符、点；不能复用 skill 名校验。 | acme.tools | My-Plugin；a_b；中文 |
| AP-NAME-ENDS | §5.5 L222 | fatal | true/MUST；P/S | 首尾都是 ASCII 小写字母或数字。 | a.1 | .a；a- |
| AP-NAME-REPETITION | §5.5 L223 | fatal | true/MUST；P/S | 禁止 -- 和 ..；不禁止 .- 或 -.。 | a.-b | a--b；a..b |
| AP-METADATA-NO-FORMAT-REJECTION | §5.4 L212 | advisory | true/MUST；C/T | 类型合规的版本、URL、邮箱、license 不因格式被拒；不擅自加入 JSON Schema format 断言。测试不应出现 fatal/component。 | version:"nightly", homepage:"local" | 客户端只因 homepage 无协议拒包 |
| AP-VERSION-SEMVER | §5.4, §10.2 L202,514 | advisory | true/SHOULD；P/S | 字段存在才检测 SemVer 2.0 形态：三段非负整数无前导零，预发布标识和 build 合法；不判断版本发布历史/破坏性变更。覆盖同一 RECOMMENDED，去重。 | 1.2.3-beta.1+build.4 | nightly；01.2.3 |
| AP-LICENSE-SPDX | §5.4 L207 | advisory | true/RECOMMENDED；P/M | SPDX identifier 推荐；没有冻结的 SPDX 列表不能凭短白名单判非法，v1 标 manual、不自动报警。SPDX expression 与 identifier 不混同。 | MIT（示例） | 自定义非 SPDX 名称，经对应列表核实后才算偏离推荐 |
| AP-CLIENT-MANIFEST-FIRST | §5.1, §5.2, §5.3, §11.3 L137,147,196,546 | fatal | true/MUST；C/T | 已知字段 fatal 后不再发现/执行组件；测试将组件读操作设为失败哨兵，确认未调用。输出 blocked coverage，不能写组件通过。 | name 错后立即停该包 | name 错仍读 mcp.json |
| AP-CLIENT-MANIFEST-REPORT | §5.2, §5.3 L153,196 | advisory | true/SHOULD；C/T | unsupported version / required field 原因可定位；报告不可依赖不稳定 serde_json 错误文案。 | 指向 /name | 仅“失败”或完全无诊断 |
| AP-LOCAL-SCHEMA-SELECTION | §5.2, §7.2.1 L153,311 | advisory | true/MUST；C/T | 只用显式支持的 canonical ID 选本地规则；可兼容映射但 v1 不声明任何映射；静态引擎自检网络入口不可达。 | 1.0.0 → 内嵌快照 | 按任意 $schema 发起 fetch；默认最新 |

## L1/L2：发现与包含关系

| rule id | spec §（原文行） | 失败半径 | 是否 normative / obligation；主体/检测 | 检测方法（含边界） | 正例 | 反例 |
|---|---|---|---|---|---|---|
| AP-PATH-MANIFEST-ESCAPE | §4.1 L62,98–100 | fatal | true/MUST；P/S | realpath(plugin.json) 与 realpath(R) 按目录段比较；先验证才读内容；R 自身是 symlink 时以其真实目标为根。 | plugin.json → R/meta.json | plugin.json → R 的兄弟目录 |
| AP-PATH-FIXED-ESCAPE | §4.1 L62,101 | component | true/MUST；P/S | skills/ 或 mcp.json 的真实目标越界，则相应 component-type 无效；不继续扫描该组件类型。 | skills → R/shared-skills | skills → ../shared-skills |
| AP-PATH-SKILL-ESCAPE | §4.1, §7.1 L62,102,281 | component | true/MUST；P/S | 直接候选目录或其精确 SKILL.md 越界，按最窄 skill scope 跳过；先查链接，不读取外部正文。同一候选的目录/文件逃逸只报一次。 | skills/a → R/internal/a | skills/a → ../../../vendor/a |
| AP-PATH-SERVER-ESCAPE | §4.1, §7.2.1 L63,103,337 | component | true/MUST；P/S | command 的 ./ 路径及 root cwd 展开后真实路径留在 R；data cwd 留在 D。D 未知时仅做形态和符号风险检查，真实包含关系记 runtime，绝不假造 D 或把词法 .. 当最终逃逸证明。 | ./bin/a；${PLUGIN_ROOT}/work | ./../bin/a；${PLUGIN_ROOT}/../sibling |
| AP-PATH-RESOURCE-ESCAPE | §4.1 L62,104 | ignored | true/MUST；P/S | 已知包资源链接越界：拒该路径访问，不断言所属 skill 无效。v1 扫 skills 安全子树内文件系统链接；其他未知扩展不读内容。访问是条件性语义，finding 写“访问时会被拒”。 | references/a → R/docs/a | references/a → R 外文档 |
| AP-PATH-RELATIVE-FORM | §4.1 L63；§7.2.1 L325,331–337 | component | true/MUST；P/S | 仅对定义为 plugin-relative 的 command/cwd 分支要求 ./；绝对路径、../、裸 cwd 非法；不套用到 Markdown 链接或 args/env。 | command:./bin/a；cwd:./ | command:../bin/a；cwd:data |
| AP-PATH-OPAQUE-VALUES | §4.1 L64；§9.2 L475 | advisory | true/MUST；C/T | args/env 值虽会替换占位符，但不当作受 §4.1 限制的包路径；不扫描字符串里的 ../、绝对路径来判逃逸。 | args:["/tmp/output","../input"] | 因上述 args 判路径违规 |
| AP-PATH-NARROWEST-BOUNDARY | §4.1 L98–104 | advisory | true/MUST；C/T | 在 manifest → fixed → skill/server → resource 层次选择实际受影响最小单元；不重复传播为整包 fatal。 | references 外链 → ignored | 文档外链 → fatal |
| AP-DISCOVERY-FIXED | §6.1, §7.2.1, §7.2.2 L241–248,303,395 | advisory | true/MUST；C/T | 只用根 skills/、mcp.json；忽略 manifest 的未知 skills/mcpServers 等字段，不从宿主文件补充配置。 | 仅 R/mcp.json 被读取 | 读取 R/.mcp.json 作 core 配置 |
| AP-DISCOVERY-MISSING | §6.2 L263 | advisory | true/MUST；C/T | lstat=ENOENT 的固定位置正常缺省；断链是存在但无效，不能与缺省混同。 | 无 skills、无 mcp.json 的 P0 | 因无 mcp.json 报包错误 |
| AP-DISCOVERY-KIND | §6.2 L265 | component | true/MUST；P/S | skills 最终须为目录，mcp.json 最终须为普通文件；目录/文件互换、断链、循环链接等可确认无效按类型隔离。EACCES 记 IO，不说格式错。 | skills 是内部目录链接 | skills 是普通文件；mcp.json 是目录 |
| AP-DISCOVERY-SKILL-EXACT | §7.1 L281 | advisory | true/MUST；C/T | readdir 精确名比对后，仅直接子目录内名为 SKILL.md 且 resolve 为普通文件的路径成为候选；不递归发现。大小写不敏感磁盘也不能用 exists("SKILL.md") 代替枚举。 | skills/a/SKILL.md | 将 skills/group/a/SKILL.md 或 skill.md 当已发现 skill |
| AP-DISCOVERY-UNSUPPORTED | §7, §11.3 L271–273,545,548 | advisory | true/MUST；C/T | 不支持的组件类型忽略；agents/hooks 等不在 core 不等于包违规；v1 校验两种已定义配置但不代表运行支持。 | skills 与 hooks 共存 | 因存在 hooks 拒包 |
| AP-ADVICE-UNDISCOVERED-SKILL | §7.1 L281（工具提示） | advisory | false/NONE；P/S | 仅直接子目录没有精确且普通的 SKILL.md 时提示未被发现；不扫描更深层寻找 skill，不称其 MUST 违规。普通辅助目录也可能触发，所以是非规范提示。 | skills/a/SKILL.md | skills/a/skill.md；skills/group/ 无 SKILL.md |
| AP-ADVICE-UNRESOLVED-PATH | §4.1（工具可见性） | advisory | false/NONE；P/S | 任意资源断链/循环、无法证明的未来路径等写明原因；不同于已证实 outside。关键固定位置用 KIND；权限失败是 errors，不重复此条。 | 可解析的内部资源 | references/a 是断链，不能推断最终目标 |

## L3：agent-skills-lint 库边界

AP §7.1 将格式要求交给 Agent Skills。本轮冻结 research/agent-skills-specification.md 日期快照，使用 agent-skills-lint::validate_source 直接判定，取消外部报告导入。字符计数、metadata、完整 YAML、Unicode/NFKC 和未知字段保守策略见 DESIGN.md §3。P/D 表示库内委托，不是外部进程。

| rule id | spec §（原文行） | 失败半径 | 是否 normative / obligation；主体/检测 | 检测方法（含边界） | 正例 | 反例 |
|---|---|---|---|---|---|---|
| AP-SKILL-CONFORMANCE | §7.1 L277,283 | component | true/MUST；P/D | 被发现且包含关系合规的 skill 才进入格式校验；库未检查项是 unchecked，失败只隔离此 skill。覆盖下面 AS-* 的确定违规；不再重复生成同一总括 finding。 | 格式合规的 a/SKILL.md | 已确认格式无效但仍加载 |
| AS-FRONTMATTER | §7.1 → AS SKILL.md format | component | true/MUST；P/D | 委托 YAML 解析和 mapping 判定，不用正则模拟 YAML；合法但参考解析器不支持的语法是 indeterminate，不能自动定罪。 | YAML mapping + Markdown | 无 frontmatter；根是序列 |
| AS-NAME | §7.1 → AS name | component | true/MUST；P/D | 非空、长度1–64、允许字符、无首尾/连续连字符、与逻辑 skill 目录名相符。AP 的点号许可不继承；Unicode/NFKC 争议须按锁定 profile。 | skills/a 的 name:a | name:A；a--b；目录 a 中 name:b |
| AS-DESCRIPTION | §7.1 → AS description | component | true/MUST；P/D | 委托字符串类型与非空、1–1024字符；多行 YAML 值在解析后计长；不按原始源码行数/字节数。 | description: Useful task instructions | 空 description；1025字符 |
| AS-OPTIONAL-FIELDS | §7.1 → AS frontmatter/compatibility/metadata | component | true/MUST；P/D | 按冻结 AS 文本检查license/allowed-tools字符串、compatibility字符串1–500字符、metadata字符串键到字符串值的映射；参考实现通过不能代替这些检查；未补齐时必须 unchecked。未知字段的闭合性依据待核实，不能仅凭 skills-ref 白名单定罪。 | compatibility 为短串，metadata:{author:"Eric"} | compatibility 超500；metadata 值为对象 |
| AS-DESCRIPTION-QUALITY | §7.1 → AS description 建议 | advisory | true/SHOULD；P/M | 描述能力、适用时机和关键词需人工语义判断，不调用模型。 | 能说明何时处理何种文档 | 只有“帮忙”且无使用情境 |
| AS-SIZE-GUIDANCE | §7.1 → AS Progressive disclosure 建议 | advisory | true/RECOMMENDED；P/D | 委托预算检查；500行边界按冻结原文 under 500，不能写成≤500。token预算针对 body，未固定 tokenizer 不估算为确定违规。 | 499行且 body 在已定 tokenizer 下少于5000 token | 500行；已定 tokenizer 下 body 达5000 token |
| AP-CLIENT-SKILL-REPORT | §7.1 L283 | advisory | true/SHOULD；C/T | 报告无效 skill 的逻辑路径并继续其他项。 | a失败，b仍处理 | 静默丢弃全部 skills |
| AP-ADVICE-SKILLS-UNCHECKED | §7.1（工具覆盖提示） | advisory | false/NONE；P/S | 有可处理 skill 但库不能给出确定结果，每包汇总一次数量与未检查 AS IDs；不能给整个 L3 写 pass。 | 有对应输入及规则版本的结构化库结果 | 解析器不支持或语义欠定却宣称 skills 合规 |

## L4：MCP 配置与运行时契约

| rule id | spec §（原文行） | 失败半径 | 是否 normative / obligation；主体/检测 | 检测方法（含边界） | 正例 | 反例 |
|---|---|---|---|---|---|---|
| AP-MCP-ENVELOPE | §7.2.1–7.2.2 L305,396 | component | true/MUST；P/S | JSON 对象；恰好 $schema/mcpServers；两者必需；mcpServers 非 null 对象，可空。顶层任何额外键禁用整类 MCP；单个 server 值非对象是 entry 错，不放大。 | M0 | mcpServers:[]；M0加foo |
| AP-MCP-SCHEMA-ID | §7.2.1 L309,311；§7.2.2 L396 | component | true/MUST；P/S | MCP canonical ID 必须受支持；类型/缺失先由 envelope 报；不发请求。版本不支持同时不匹配时由 VERSION-MATCH 优先报告一次并带原因。 | 1.0.0/mcp.schema.json | 1.0.0/plugin.schema.json |
| AP-MCP-VERSION-MATCH | §7.2.2, §10.1 L396,508 | component | true/MUST；P/S | 对识别出的 canonical 标识提取版本，与已有效 plugin manifest 比较；不比较两 URL 整串，也不比较 plugin.version。 | plugin 1.0.0，mcp 1.0.0 | plugin 1.0.0，mcp 1.1.0 |
| AP-MCP-SERVER-VARIANT | §7.2.1 L305,313–347 | component | true/MUST；P/S | 每 entry 独立闭合：stdio={type,command,args?,env?,cwd?}；remote={type,url,headers?}，type仅三枚；字段类型按表。值非对象/数组、未知字段/跨变体字段、缺必填均只废该 server。server key 不额外套 plugin name 规则。 | {type:stdio,command:node,args:[]} | stdio含url；type:http；值null |
| AP-MCP-COMMAND | §7.2.1 L325 | component | true/MUST；P/S | 单个可执行 token，只准裸名或 ./ 路径；不分词、不 shell 执行、不展开。./ 路径中空格不能直接判 shell 串；裸名含空白/引号/运算符的歧义按 DESIGN 返回 advisory 待核而非未经证实的 fatal。绝对路径/../ 路径确定非法。 | node；./bin/my server | /usr/bin/node；../server |
| AP-MCP-BUNDLED-COMMAND | §7.2.1 L327 | component | true/MUST；P/M | 确知意图执行包内二进制时必须 ./ 路径；同名文件存在不证明裸命令指向它。v1 不读程序以猜意图。 | 包内 bin/a 用 ./bin/a | 已确认依赖包内 a 却用 command:a |
| AP-MCP-PATH-DEPENDENCE | §7.2.1 L327 | advisory | true/MUST；P/M | 不依赖配置 PATH 参与裸名查找；有 env.PATH 本身合法。需作者说明或宿主对照执行证明依赖，规范未指定独立失败边界。 | 裸node来自平台搜索 | 只有 env.PATH 生效时才找得到必需命令 |
| AP-MCP-CWD-FORM | §7.2.1 L331–337 | component | true/MUST；P/S | 缺省合法；存在时三种精确前缀：./、${PLUGIN_ROOT}(后接/或结束)、${PLUGIN_DATA}(后接/或结束)。展开后 containment 单独检查。 | ${PLUGIN_DATA}/cache | data；${PLUGIN_ROOT}suffix；/tmp |
| AP-MCP-URL | §7.2.1 L351 | component | true/MUST；P/S | 绝对 http(s)；原始 authority 无userinfo标记，原串不含 #（含空fragment也拒）；url::Url 辅助解析但不让其“修复”非绝对/反斜杠/空白输入。无 DNS。 | https://api.example/mcp | /mcp；ftp://a；https://u:p@a；https://a/# |
| AP-MCP-HTTPS | §7.2.1 L351 | component | true/MUST；P/S | 非loopback必须https；http只允许规范化DNS名恰为localhost或已识别IP loopback。不信DNS解析、不接受localhost后缀/localhost.；IP处理见DESIGN。 | http://localhost:3000；http://127.2.3.4；http://[::1] | http://api.example；http://localhost.example；http://0.0.0.0 |
| AP-MCP-HEADERS | §7.2.1 L353 | component | true/MUST；P/S | headers值先有字符串类型；用 http::HeaderName/ http::HeaderValue，ASCII小写名判跨大小写重复；同字面重复键交JSON重复键策略。没有url/header替换。 | X-Tenant:public | X-Test与x-test同时存在；值含CR/LF |
| AP-MCP-HEADER-SECRETS | §7.2.1 L355；§7.2.2 L397 | component | true/MUST；P/M | 已确认为凭据/秘密才是此规则；header名 Authorization 不能单独证明值为有效secret；静态启发式只报 ADVICE-POSSIBLE-SECRET。 | X-Tenant:public-tenant | 人工确认有效的私有 API token 被写入 headers |
| AP-MCP-ENV-SECRETS | §9.2 L479；§7.2.2 L397 | component | true/MUST；P/M | 与header同样区分确定秘密与怀疑；不在报告回显值。 | env:{MODE:"test"} | 已确认凭据明文放env |
| AP-MCP-RESERVED-ENV | §9.2 L481 | component | true/MUST；P/S | 精确键 PLUGIN_ROOT/PLUGIN_DATA 禁止；等价大小写是否同样违规原文未明确，v1混合大小写仅advisory，运行时仍须按平台覆盖。 | env:{DATA_DIR:"${PLUGIN_DATA}"} | env:{PLUGIN_ROOT:"/tmp"} |
| AP-ADVICE-POSSIBLE-SECRET | §7.2.1, §9.2（启发式） | advisory | false/NONE；P/S | 键名不区分大小写匹配 token/secret/password/api_key 或 Authorization/Proxy-Authorization，加非空字面值即提示候选；可误报样例、漏报随机名。不给真实性背书，不输出原值或摘要。 | MODE:test | API_TOKEN:example 被提示但不判确定违规 |
| AP-ADVICE-AMBIGUOUS-COMMAND | §7.2.1 L325（语法欠定） | advisory | false/NONE；P/S | 裸名含空白、shell标点，或未能证明为单个可执行路径时提示检查；不将字符串自动拆成 args。文件名可含这些字符。 | node | node --version；echo x > y |
| AP-ADVICE-ENV-CASE | §9.1–9.2 L460,481（平台差异） | advisory | false/NONE；P/S | env含与保留名仅大小写不同的键，或用户键相互仅大小写不同，提示平台行为差异；没有 --host 推断。 | DATA_DIR | plugin_root；Path与PATH |
| AP-CLIENT-COMMAND-RESOLUTION | §7.2.1 L325,329 | advisory | true/MUST；C/T | 裸名走平台搜索，./走R；命令无占位符替换；即使用平台解释器启动.cmd也保留单token并分离args。不在linter做实际搜索/启动。 | executable与args分开传递 | 拼接shell串；展开command中的ROOT |
| AP-CLIENT-CWD | §7.2.1 L331,337,339 | advisory | true/MUST；C/T | 缺省R；先一次展开再文件系统解析并校验对应R/D边界。参数cwd里的未知placeholder仍是字面。 | 缺cwd→R | 缺cwd→启动linter的工作目录 |
| AP-CLIENT-REMOTE-LITERALS | §7.2.1 L353 | advisory | true/MUST；C/T | url/header名/值完全不做placeholder或环境变量替换；字面包含占位符不自动等于包违规，另看URL/header语法。 | X-Dir:${PLUGIN_ROOT}保持字面 | 将header的ROOT替成路径 |
| AP-CLIENT-HEADER-PRECEDENCE | §7.2.1 L355 | advisory | true/MUST；C/T | 客户端为协议/鉴权生成的header按大小写不敏感覆盖配置；这是规范段落中的直接要求。包静态不可验证。 | client Authorization覆盖配置 | 包配置覆盖client鉴权header |
| AP-CLIENT-HEADER-REDIRECT | §7.2.1 L355 | advisory | true/MUST；C/T | 未明确授权不得向不同origin重定向/SSE事件目标转发配置headers；origin含scheme/host/port，不只域名。 | 同origin；跨origin先获授权 | https:a:443→https:a:8443且未经授权带header |
| AP-CLIENT-TRANSPORT-MINIMUM | §7.2.1, §11.1 L361,534 | advisory | true/MUST；C/T | 支持MCP的client至少支持stdio或streamable-http之一；只支持sse不足。包中只有sse仍合法。 | client支持stdio | client仅支持sse却宣称AP MCP合规 |
| AP-CLIENT-TRANSPORT-BOTH | §7.2.1 L361 | advisory | true/SHOULD；C/T | 建议同时支持stdio/streamable-http；非包检查，不因包缺其中一种报警。 | client支持两者 | client只实现一种（仍可符合最低要求） |
| AP-CLIENT-TRANSPORT-INITIAL | §7.2.1 L361 | advisory | true/MUST；C/T | 初次连接必须使用entry.type；fallback没规定，不编造禁令。 | sse初次用sse | 忽略type先尝试另一协议 |
| AP-CLIENT-MCP-CONFIG-BOUNDARY | §7.2.2 L396 | component | true/MUST；C/T | envelope/版本错误禁用MCP，保留skills；不把mcp JSON Schema整体失败都归整类，entry另判。 | 坏mcp仍发现skills | 坏mcp拒绝整个plugin |
| AP-CLIENT-MCP-ENTRY-BOUNDARY | §7.2.2 L397 | component | true/MUST；C/T | 坏entry只跳它；空mcpServers不违规；保留其他entry。 | a坏b好→b继续 | a坏禁全部MCP |
| AP-CLIENT-MCP-UNSUPPORTED | §7.2.2, §11.3 L398,548 | component | true/MUST；C/T | 合法但不支持的transport只跳该server；不是包配置错误，包扫描不生成该finding。 | client不支持sse而跳它 | 不支持sse→整包失败 |
| AP-CLIENT-MCP-CONNECTION | §7.2.2 L399 | component | true/MUST；C/T | 启动/连接/认证/握手失败隔离进程，独立组件继续；不静态探测endpoint。 | 一台离线其他继续 | 一台认证失败拒全部skills |
| AP-CLIENT-MCP-REPORT | §7.2.2 L396–399 | advisory | true/SHOULD；C/T | 分别报告config、entry、unsupported、connection类别；不能把授权失败称manifest非法。 | 报server a握手失败 | 报整包schema无效 |

## L5：扩展

| rule id | spec §（原文行） | 失败半径 | 是否 normative / obligation；主体/检测 | 检测方法（含边界） | 正例 | 反例 |
|---|---|---|---|---|---|---|
| AP-EXTENSIONS-OBJECT | §5.2, §8.1, §11.3 L147,411,427,546 | ignored | true/MUST；P/S | extensions非对象（含null/数组）报告并忽略整个字段，仍加载有效组件。这是第二个非fatal例外。 | extensions:{} | extensions:[] |
| AP-EXTENSION-NAMESPACE | §8, §8.1 L403,411 | fatal | true/MUST；P/S | namespace要reverse-domain；仅明确非法时使用fatal，语法边缘待决；空名或含路径分隔符明确不构成reverse-domain。标签数和完整语法未在本地正文定义，schema正文已核实但未定义 namespace 正则，无点/空段/IDN/大小写/下划线等暂不强判，记未评估；不能查DNS证明控制权。 | com.openai | ""；../x（example/com..x暂列未评估） |
| AP-EXTENSION-VALUE | §8.1 L411,427；§11.1 L532 | ignored | true/MUST；P/M | 本行ignored是待决提案，非已裁定的规范半径。包结构要求每value为对象，与忽略未知value不验证的client要求存在解释空间。v1不实现namespace，故不检查未知value，记unchecked-by-design；人工确认非对象可记录包侧不合规，半径未获规范明确裁决，不得自动升级fatal。 | com.x:{} | com.x:3（作者侧要求不符，客户端处理待澄清） |
| AP-EXTENSION-UNKNOWN | §8.1, §11.1 L427,532 | advisory | true/MUST；C/T | 不实现的namespace值整体不解释/不检查内容，不运行私有校验器；顶层extensions类型与namespace键仍处理。 | 未知namespace含任意对象内容不报错 | 遍历com.openai.interface并要求自定字段 |
| AP-EXTENSION-FILE-LOCATION | §8, §8.2 L403,431,446 | advisory | true/MUST；P/M | 已确认某文件属于某扩展时应在同名顶层目录；未知文件不凭目录名推断意图。规范未给通用包失败半径；client文件发现行为另测。manifest数据和目录可各自独立。 | 仅com.x/；仅extensions.com.x | 已声明使用com.x的file行为却从其他目录取该扩展 |
| AP-EXTENSION-CLIENT-DISCOVERY | §8.2 L446 | advisory | true/MUST；C/T | 实现某namespace的file行为才去对应顶层目录；没有目录不自动报错。 | 实现com.x→查R/com.x | 从R/private/com.x发现同一file扩展 |
| AP-EXTENSION-DOMAIN-CONTROL | §8 L405 | advisory | true/SHOULD；U/M | 控制namespace对应域名是建议，离线包不能证明；不进行WHOIS/DNS请求。 | 所有者控制example.com使用com.example | 使用明确由他人控制的域名 |
| AP-EXTENSION-STABILITY | §8 L405 | advisory | true/SHOULD；U/M | 需跨版本历史评估namespace稳定性；一次目录扫描不可证明。 | 连续版本com.example | 无必要每版更换namespace |

## 环境、发布方与总契约

| rule id | spec §（原文行） | 失败半径 | 是否 normative / obligation；主体/检测 | 检测方法（含边界） | 正例 | 反例 |
|---|---|---|---|---|---|---|
| AP-CLIENT-ENV-ROOTS | §9.1, §9.2 L454,481 | advisory | true/MUST；C/T | 每个stdio子进程提供绝对真实R和该安装实例专属持久D；不是要求包预定义它们。 | client设定两个变量 | 缺PLUGIN_DATA；把进程cwd当ROOT |
| AP-CLIENT-DATA-LIFECYCLE | §9.1 L456 | advisory | true/MUST；C/T | 启动前建D且可写，更新保留；卸载删除允许；需模拟升级和权限测试，静态不做。 | 更新保留缓存 | 更新删除D；启动时D不可写 |
| AP-CLIENT-ENV-OVERLAY | §9.1 L460 | advisory | true/MUST；C/T | 先展开env，再按平台同名语义覆盖base，最后写保留变量；Windows大小写等价覆盖，不能从POSIX样本推断跨平台。 | env覆盖base，ROOT最后设 | base覆盖env；env覆盖ROOT |
| AP-ENV-BASE-DEPENDENCE | §9.1 L462 | advisory | true/MUST；P/M | 除裸command平台查找外，不依赖未定义且未显式配置的base变量；必须审程序/运行矩阵，不能因字符串$HOME就判程序依赖。 | 依赖env显式MODE | 必需隐式HOME且无其他输入来源 |
| AP-EXPANSION-SINGLE-PASS | §7.2.1, §9.2 L339,473–475 | advisory | true/MUST；C/T | args每项、env每值、cwd每个精确ROOT/DATA出现一次非递归替换；不替换key/command/fixed location；不得扫描替换新文本。 | ROOT的值含DATA字面时不再次展开 | 循环replace直至无placeholder |
| AP-EXPANSION-LITERAL | §9.2 L477 | advisory | true/MUST；C/T | 未识别placeholder保留；不扩展$HOME、~、%NAME%、${X}；不因这些字面串直接判包错。 | ${HOME}保持字面 | 展开所有环境变量 |
| AP-RELEASE-SCHEMA-PAIR | §10.1 L506 | advisory | true/MUST；U/M | 发布方每规范版本发布同版两份schema；索引不证明正文，v1 vendor步骤核文件身份与$id，缺正文不能过门槛。 | 1.0.0两份同版schema | 只发布plugin schema或MCP不同版 |
| AP-RELEASE-SCHEMA-IMMUTABLE | §10.1 L510 | advisory | true/MUST；U/M | canonical ID不复用为不同内容；只有两个可信历史快照才可比较。工具固定hash自检能查本地漂移，不能证明上游全部历史。 | 新schema内容用新spec版 | 同ID悄悄改内容 |
| AP-CLIENT-CONFORMANCE | §1, §11.1 L32,528–537 | advisory | true/MUST；C/T | §1总括映射全表；§11.1八项中1–7映射前述规则，第8项至少支持一个component type；不能以本linter通过证明client合规。 | skills-only且遵循适用要求 | 两类均不支持却声明conformant |
| AP-CLIENT-FAILURE-ISOLATION | §11.3 L547 | advisory | true/MUST；C/T | 类型/条目/进程失败不得阻断独立有效组件；汇聚边界测试，不为同一包问题重复发finding。 | a失败b继续 | fail-fast终止批次其他plugin |
| AP-CLIENT-FAILURE-REPORT | §11.3 L548 | advisory | true/SHOULD；C/T | 报无效配置与组件失败；不支持本身不是error。 | 明确component原因 | 把未实现扩展报schema错 |
| AP-ADVICE-DUPLICATE-JSON-KEY | 工具策略（AP未明定） | advisory | false/NONE；P/S | JSON同对象同字面键重复提示；serde_json 默认保留最后值用于后续检查。需 serde 自定义 Visitor 记录重复键，不能正则；未实现则 coverage=unchecked；不能声称AP禁止所有重复键。 | {"name":"a"} | {"name":"a","name":"b"} |

## RFC2119 覆盖核对索引

下表以**原文行**作审计单位，不以 rules.txt 的“73”作完成率分母。一个源行可能有多个 MUST 和多个规则；一个规则也可能覆盖规范多处重复陈述。上面每个要求的正反例是验收输入，不是已执行测试。

| 原文 L | 映射 rule id（省略共同 AP- 前缀；AS保留） |
|---|---|
| 32 | CLIENT-CONFORMANCE |
| 40 | 元语言定义；不是待检测的包规则，定义MUST/SHOULD/REQUIRED等关键词 |
| 61 | MANIFEST-LOCATION |
| 62 | PATH-MANIFEST-ESCAPE, PATH-FIXED-ESCAPE, PATH-SKILL-ESCAPE, PATH-SERVER-ESCAPE, PATH-RESOURCE-ESCAPE |
| 63 | PATH-RELATIVE-FORM, PATH-SERVER-ESCAPE |
| 64 | PATH-OPAQUE-VALUES |
| 98 | PATH-NARROWEST-BOUNDARY |
| 100 | PATH-MANIFEST-ESCAPE |
| 101 | PATH-FIXED-ESCAPE |
| 102 | PATH-SKILL-ESCAPE |
| 103 | PATH-SERVER-ESCAPE |
| 104 | PATH-RESOURCE-ESCAPE |
| 133 | MANIFEST-LOCATION |
| 143 | MANIFEST-JSON, MANIFEST-UNKNOWN-FIELD |
| 145 | MANIFEST-UNKNOWN-FIELD |
| 147 | MANIFEST-METADATA-TYPE, MANIFEST-AUTHOR, EXTENSIONS-OBJECT, CLIENT-MANIFEST-FIRST |
| 151 | MANIFEST-SCHEMA-ID |
| 153 | MANIFEST-SCHEMA-ID, LOCAL-SCHEMA-SELECTION, CLIENT-MANIFEST-REPORT |
| 196 | MANIFEST-REQUIRED, CLIENT-MANIFEST-FIRST, CLIENT-MANIFEST-REPORT |
| 202 | VERSION-SEMVER（RECOMMENDED） |
| 207 | LICENSE-SPDX（RECOMMENDED） |
| 212 | METADATA-NO-FORMAT-REJECTION |
| 216 | NAME-LENGTH, NAME-CHARSET, NAME-ENDS, NAME-REPETITION（包含整个表） |
| 220 | NAME-LENGTH |
| 222 | NAME-ENDS |
| 241 | DISCOVERY-FIXED |
| 263 | DISCOVERY-MISSING |
| 265 | DISCOVERY-KIND |
| 273 | DISCOVERY-UNSUPPORTED |
| 277 | SKILL-CONFORMANCE, AS-* |
| 281 | DISCOVERY-SKILL-EXACT |
| 283 | SKILL-CONFORMANCE, CLIENT-SKILL-REPORT |
| 303 | DISCOVERY-FIXED, MANIFEST-UNKNOWN-FIELD |
| 305 | MCP-ENVELOPE, MCP-SERVER-VARIANT |
| 309 | MCP-SCHEMA-ID |
| 311 | LOCAL-SCHEMA-SELECTION |
| 313 | MCP-SERVER-VARIANT |
| 325 | MCP-COMMAND, PATH-RELATIVE-FORM, CLIENT-COMMAND-RESOLUTION |
| 327 | MCP-PATH-DEPENDENCE, MCP-BUNDLED-COMMAND |
| 329 | CLIENT-COMMAND-RESOLUTION |
| 331 | MCP-CWD-FORM, CLIENT-CWD |
| 337 | CLIENT-CWD, PATH-SERVER-ESCAPE |
| 339 | EXPANSION-SINGLE-PASS |
| 351 | MCP-URL, MCP-HTTPS |
| 353 | MCP-HEADERS, CLIENT-REMOTE-LITERALS |
| 355 | MCP-HEADER-SECRETS, CLIENT-HEADER-REDIRECT, CLIENT-HEADER-PRECEDENCE |
| 361 | CLIENT-TRANSPORT-MINIMUM, CLIENT-TRANSPORT-BOTH, CLIENT-TRANSPORT-INITIAL |
| 395 | DISCOVERY-FIXED |
| 396 | MCP-ENVELOPE, MCP-SCHEMA-ID, MCP-VERSION-MATCH, CLIENT-MCP-CONFIG-BOUNDARY, CLIENT-MCP-REPORT |
| 397 | CLIENT-MCP-ENTRY-BOUNDARY, CLIENT-MCP-REPORT |
| 398 | CLIENT-MCP-UNSUPPORTED, CLIENT-MCP-REPORT |
| 399 | CLIENT-MCP-CONNECTION, CLIENT-MCP-REPORT |
| 403 | EXTENSION-NAMESPACE, EXTENSION-FILE-LOCATION |
| 405 | EXTENSION-DOMAIN-CONTROL, EXTENSION-STABILITY |
| 411 | EXTENSIONS-OBJECT, EXTENSION-NAMESPACE, EXTENSION-VALUE |
| 427 | EXTENSIONS-OBJECT, EXTENSION-UNKNOWN |
| 446 | EXTENSION-CLIENT-DISCOVERY |
| 454 | CLIENT-ENV-ROOTS |
| 456 | CLIENT-DATA-LIFECYCLE |
| 460 | CLIENT-ENV-OVERLAY |
| 462 | ENV-BASE-DEPENDENCE |
| 473 | EXPANSION-SINGLE-PASS |
| 477 | EXPANSION-LITERAL |
| 479 | MCP-ENV-SECRETS |
| 481 | MCP-RESERVED-ENV, CLIENT-ENV-ROOTS |
| 506 | RELEASE-SCHEMA-PAIR |
| 508 | MCP-VERSION-MATCH |
| 510 | RELEASE-SCHEMA-IMMUTABLE |
| 514 | VERSION-SEMVER |
| 528 | CLIENT-CONFORMANCE（包括L530–537全部八项） |
| 545 | DISCOVERY-UNSUPPORTED |
| 546 | MANIFEST-UNKNOWN-FIELD, EXTENSIONS-OBJECT, CLIENT-MANIFEST-FIRST |
| 547 | CLIENT-FAILURE-ISOLATION |
| 548 | CLIENT-FAILURE-REPORT |

抽取修正：rules.txt L3 是关键词释义，L68 只有 MAY，L74 位于非规范 Design Decisions，均不能当独立 MUST 规则；漏掉原文L202/207的 RECOMMENDED 和L220/222表格中的MUST，L216也不能代替逐约束实现。原文还有无全大写关键词但被 MUST 引用的表格/条目（name字符集、author、stdio/remote字段、cwd三形态、§11.1八项），本表都显式展开。Appendix A 用作交叉检查，不另产生规则。FUTURE_CONSIDERATIONS 全为非规范，不纳入合规判据。

## Rust 执行归属与切片

| 规则组 | crate / 模块 | 类型边界 | 切片 |
|---|---|---|---|
| AS-* | agent-skills-lint / parser、validator、diagnostic | SkillRule + SkillIssueKind + SkillReport；计数为 chars().count() | S1 |
| AP-MANIFEST-*、AP-NAME-*、AP-VERSION-* | agent-plugin-lint / manifest、report、vendor | RuleId + Finding；两非 fatal 例外先投影 | S2 |
| AP-DISCOVERY-*、AP-PATH-*、AP-SKILL-* | agent-plugin-lint / discovery、containment、skills | ResolvedRoot、PathRole、Containment；Scope/Effect 最窄隔离 | S3 |
| AP-MCP-*、AP-EXPANSION-* | agent-plugin-lint / mcp、expansion | envelope 与 server 分开；未知数据根 Runtime | S4 |
| AP-EXTENSION-*、AP-EXTENSIONS-* | agent-plugin-lint / extensions | 未实现 namespace 不读 value；不强制数据/目录成对 | S5 |
| AP-CLIENT-*、AP-RELEASE-*、人工规则 | report / coverage 与开发测试 | 适用的模拟契约写测试，其余 Manual/Runtime；不伪报通过 | 各片 |

91 条 ID 保持不变。代码元数据测试应核对 ID 与表的映射；每条具体规则的执行状态在 coverage 或验证记录中呈现。默认政策 D4 暂按确定 normative MUST 包违规返回 1（含 ignored）；D3/D5/D6 保守未检查；待决身份不改变。
