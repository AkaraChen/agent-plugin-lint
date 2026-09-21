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

## S3 首轮：真实靶子命中，仍退回

astra 运行 `AP_LINT_CORPUS=/home/akrc/Developer/eric-way/plugins cargo test --workspace`（54 个测试，语料变量显式提供）、clippy、fmt、build、21 个文件系统及 6 个 S2b 复现通过。独立 CLI 扫描确实产生预期三条外链，11 包，退出1；两个资源 ignored/deny-path，整个 guided-review component/skip-skill（§4.1、§7.1）。

但额外探针暴露：skills 权限错误返回0或误报kind；外链中的SKILL.md是目录仍被当skill；500行建议被适配层吞掉；合法skill只剩总括pass、库coverage丢失；资源断链完全无记录。源码还缺原定读取身份二次核验，read_dir/编码错误被吞。已退回，真实靶子成功没有取代正确性审查。

S3 后续复验：cargo 显示54个 workspace 测试通过，其中语料测试未给环境变量而空返回；这项不计真实语料验证，只有53项实际执行测试。clippy、fmt、21 filesystem + 6 S2b + 12 S3 probes 通过，但尚未接受。源码仍无 File handle 身份核验，manifest 未接共同 reader，resolve 丢 IO 原因。新增三个权限探针（SKILL.md 不可读、mcp 链接穿过无权限目录、资源链接穿过无权限目录）实际退出分别0/1/0，应全为2。已换新 terra 会话，将读取/IO/精确发现拆成明确修正片，继续验证，不以实现总结作为通过证据。

S3 读取/IO 修正片：astra 独立执行57个常规测试（另1语料测试明确ignored）、clippy、fmt、build；21+6+12黑盒通过；再显式 `AP_LINT_CORPUS=... cargo test -p agent-plugin-lint --test s3 corpus -- --ignored` 1/1通过。已核实typed IO、File metadata dev/ino/mtime比较、manifest/skill共同reader、递归前根界检查。仍退回精确文件名发现遗漏、外部helper缺SKILL的ENOENT分类、FIFO替换测试，以及AS建议项/元数据汇总收口。

JSON收口待修：独立CLI证明未知字段1e400被解析器范围限制误报manifest JSON fatal。独立依赖实验启用arbitrary_precision后，大数通过但真实`$serde_json::private::Number`对象键被改成Number（1/2实验失败），因此不采用仅开feature的修法。S5将采用语法/表示能力分离，能力不足明确未检查；临时实验文件已移除，不计常规测试通过。

S3 接受：astra 最后独立执行62个常规测试、clippy、fmt、build，18个强化skill黑盒全过；显式ignored语料1/1通过。此前21文件系统+6 CLI错误复现及29个已实现规则元数据对照也已通过。确切枚举、句柄身份变化检测、普通文件换FIFO、IO隔离、完整AS适配、建议不提升为MUST、混合可读/不可读skill汇总均已核实。真实语料仍11包、23个AS实际读取target、1个外部skill未读取；精确三条路径/radius/effect通过（§4.1、§6.2、§7.1）。读取不是原子快照；其他平台未测试。JSON数值能力边界仍作为S5已知待修，不声称整个引擎已最终验收。

## S4 首轮：退回

astra 独立执行65个常规测试、clippy、fmt、build通过；42个当前规则元数据对照通过。强化MCP脚本在合法cwd `./` 失败。额外探针发现：相对cwd错误基于进程目录；包内command `./bin/../server` 被词法误拒；空command和无./的bin/server漏检；command权限错误0；percent/hex host无unchecked；空authority被URL解析器修复后放行。已按§4.1、§7.2.1退回，并要求正式集成矩阵、IO贯通及覆盖状态，不接受S4。

S4 第二轮：astra 独立72常规测试+1显式语料、clippy、fmt，以及39 MCP+21 filesystem+18 skills+6 CLI错误黑盒通过；42元数据对照通过。组合探针仍发现./cwd跳过placeholder展开、后续IPv4段hex未标未知、非压缩mappedIPv6被误判HTTPS违规。已按§7.2.1/§9.2与D5保守策略再次退回；读取/脚本通过不代替这些组合判定。

S4 接受：astra 最后独立执行74个常规测试、clippy、fmt、build、42个MCP黑盒、1个显式ignored语料测试，均通过；另重跑command权限错误探针，确认exit2且独立good server仍有实际Pass。之前21filesystem+18skills+6CLI错误及42规则元数据对照已通过。已核对§4.1/§7.2.1–7.2.2配置边界与§9.2单次展开，真实路径、raw authority及非标准IP未检查策略落实。没有运行MCP启动/连接/认证/握手，也未证明任何宿主客户端合规。JSON大数能力与扩展/全表覆盖仍待S5。
