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

本轮源码引擎切片已按下文范围复验；以下不在已验证范围：Windows/macOS、实际宿主加载、安装器产物、MCP 启动/连接/认证/握手、PLUGIN_DATA 生命周期、秘密真实性、域名控制权、完整客户端符合性、发布安装尚未验证。

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

## S5a 接受

astra 独立执行79个常规Rust测试、clippy、fmt、build，以及10 JSON边界+10 extensions+42 MCP+21 filesystem+18 skills+6 CLI错误+25完整CLI黑盒，共132个；显式真实语料ignored测试1/1通过。核对§8/§8.1：未知namespace值不验证，不推断同名目录必须存在；空名/路径分隔符拒绝，其他语法欠定unchecked。JSON语法与表示能力分离，1e400及额外160层合法嵌套探针均exit2而非JSON违规；重复键按最后值检查并仅advisory，strict提升。§5.2与§7.2父级失败门禁保持。过深输入当前提示写“数值”偏窄，S5b改为通用解析器表示能力措辞。

## S5b 首轮：退回

astra 独立运行83个常规Rust测试、clippy、fmt、build通过；44个已实现RuleId元数据与原表对照通过，内嵌91条registry JSON与原表逐项相同。root coverage.py初版正则错误收集6条规则组摘要，97不是规则数量；已修正精确ID提取，未削弱状态断言。随后实测合法DATA cwd的MCP strict错误返回1，原因是P/M人工规则被标为静态unchecked。8个进一步证据一致性探针全部失败：已读manifest、有效MCP、component/resource finding、skill语义人工项、JSON表示能力父级错误、MCP envelope错误、ignored extensions的coverage分支均存在错误N/A或未阻断状态。已全部退回terra；不能用83测试通过替代报告正确性。

S5b 第二轮：astra 独立执行整套18条验证命令（locked/offline Rust测试、clippy、fmt、doc、build、显式语料、9份黑盒/运行观察、双crate包清单）通过。源码复查仍发现旧分支未完成：坏MCP同rule/target同时blocked与manual、relative form没有实际coverage、正常skills固定/skill containment缺实际pass。强化coverage_edges为10项后7过3失败。已换新terra会话，限定coverage证据收口，不接受“脚本绿但分支缺失”的实现；此前18条命令通过不等于S5b验收通过。

## S5b 接受与最终复验（2026-09-21）

换新terra会话后，astra核对了实际分支中的位置/包含/relative-form记录、finding对应fail、MCP gate后继blocked及同rule/target状态一致性。末尾registry无目标级记录的包侧条目统一明确为unchecked/RULE_NOT_EVALUATED，取消“缺记录意味着目标不存在”的假设。此索引占位未被strict选择；实际目标上的静态unchecked仍被strict提升，最小包/MIT/DATA runtime为0、IP语法歧义strict为1。该选择已写入DESIGN和README，不将全部91条注册项称为静态已验。

astra最终亲自执行以下命令，均通过（本机Ubuntu，rustc/cargo 1.97.1）：

```sh
cargo fmt --all -- --check
cargo test --workspace --locked --offline
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
cargo doc --workspace --no-deps --locked --offline
cargo build -p agent-plugin-lint --locked --offline
AP_LINT_CORPUS=/home/akrc/Developer/eric-way/plugins cargo test -p agent-plugin-lint --test s3 --locked --offline corpus -- --ignored
AP_LINT_CORPUS=/home/akrc/Developer/eric-way/plugins python3 scripts/review/cli.py full
python3 scripts/review/filesystem.py full
python3 scripts/review/s2b.py
python3 scripts/review/skills.py
python3 scripts/review/mcp.py
python3 scripts/review/json_edges.py
python3 scripts/review/extensions.py
python3 scripts/review/coverage.py
python3 scripts/review/coverage_edges.py
AP_LINT_CORPUS=/home/akrc/Developer/eric-way/plugins python3 scripts/review/runtime.py
cargo package -p agent-plugin-lint --list --offline --allow-dirty
cargo package -p agent-skills-lint --list --offline --allow-dirty
```

结果为85个常规Rust测试、另1个显式真实语料测试；9份黑盒脚本共152个用例（25+21+6+18+42+10+10+10+10）。doc构建通过，当前无doctest示例，不能把0个doctest当额外API覆盖。44个已实现RuleId的半径/义务/主体/规范性另作临时Rust测试与原表对照通过，测试文件已删除；内嵌91条完整registry元数据与原表逐项一致。

真实引擎扫描11包，24直接skill候选，23个安全SKILL.md进入库，1个越界skill被跳过，0 MCP配置。精确命中上表三条逃逸：两个资源ignored/deny-path（§4.1(3)及最窄边界第5项），guided-review为component/skip-skill（§4.1最窄边界第3项、§7.1）。最终summary为fatal=0、component=1、ignored=2、advisory=1、errors=0，exit=1。额外advisory是review包的AP-ADVICE-SKILLS-UNCHECKED，不是第四条逃逸；未读取的skill不被包装成合规。相同输入两次JSON逐字节一致，全语料coverage键唯一。

`runtime.py`实际把二进制复制到仓库外，用strace观察独立MCP样例、未知schema URL、真实语料三个输入：网络系统调用均0，命令哨兵均未执行，扫描前后输入内容/链接/权限快照一致。源码审计也未发现生产网络请求、子进程执行或写入输入的路径；命令/FIFO写入仅在cfg(test)测试模块。此为已覆盖输入的观察，不是形式化证明。尝试`unshare -Urn true`被本机拒绝（写/proc/self/uid_map: Operation not permitted），未声称网络命名空间隔离验过。

两份随crate分发的schema与research原始字节、Git blob、SHA-256再次一致；plugin包清单含schema/Apache-2.0/来源说明/MIT，skills包含MIT/迁移说明。仅检查Cargo包清单，没有验证registry发布或完整发布包安装。原设计目录12文件SHA-256与开始基线一致；skills-ref仍为6c89f06b且工作区干净、未归档；eric-way仍为只读37d007ca快照且工作区干净。

未验：Windows/macOS/junction平台行为，原子文件系统快照或完整竞态安全证明，宿主加载/安装器产物，MCP启动/连接/认证/握手，PLUGIN_DATA生命周期，秘密真实性、域名控制权、完整客户端合规、固定tokenizer预算、crates.io发行及安装。没有配置远程CI，不声称GitHub Actions通过。运行期未知项、Unicode/NFKC及RULE_NOT_EVALUATED继续以未检查/人工/运行时状态公开。

待飞鸢裁决仍为D3未知扩展作者侧语义、D4默认ignored MUST及strict选择政策、D5欠定语法接受域、D6 AS快照/Unicode裁决、D7宿主与发行范围、D8平台承诺。新库接口形状已按本轮Rust设计落地；旧库归档仍是另行授权的后续动作。


## D3–D8 平台续轮（2026-09-21，进行中）

本节覆盖前一轮的“未配置远程CI / D3–D8待拍板”历史状态；DECISIONS.md 的追加定案有效，PR #1 保持未合并。astra 使用 Paseo 调度 codex/gpt-5.6-terra（full-access）实现，自己负责设计、源码review与独立执行验证。

astra 亲自执行 `gh run view 35594975412 --log-failed` 核对原始失败。macOS 首个错误是 InputChanged 精确断言；另三个测试是该 panic 导致测试串行锁 poisoning 的连带失败，不能据此声称三个 FIFO/link 分支各自有安全失败。Windows 同一 InputChanged 断言失败。根因是 read_safe 只 canonicalize 文件、没有 canonicalize 传入的根：macOS `/var` 别名与 Windows extended-length 前缀使包含比较提前返回 Unsafe，尚未进入竞态 hook。

52f67ee 固定每次操作入口的 canonical 根，整个读取期间不重新绑定边界；增加普通文件、Unix根别名/根切换测试。原 InputChanged、Unsafe 精确断言及 O_NONBLOCK 保留。HookGuard 清理测试 hook，poison 恢复只避免前次测试污染，不吞掉原 panic。astra 本机独立运行 `cargo fmt --all -- --check`、`cargo test --workspace --locked`、`cargo clippy --workspace --all-targets --locked -- -D warnings`、`git diff --check` 均过。

[原生CI 35596049438](https://github.com/AkaraChen/agent-plugin-lint/actions/runs/35596049438) 验证 containment 所在 lib：macOS 13/13、Windows 8/8 全过，Ubuntu全套通过；不能据此宣称三平台全绿。继续执行后暴露 macOS 的 cwd fixture 用 raw 根构造路径，未跟随 canonical ROOT；Windows 的 vendor schema 哈希错误，实际哈希与 LF 转 CRLF 的独立计算完全吻合。保留哈希和路径语义断言，修正 fixture/checkout 字节保真，不接受改变预期哈希。

D3/D4 初审：astra 独立实际 CLI 输入涵盖未知 namespace 的 number/array、明确非法 namespace、非object extensions、未知顶层字段、非SemVer版本、env大小写。结果符合 §8.1 的未知值不检、D4 ignored MUST 默认1/advisory默认0。发现 D5 缺口：反斜杠/驱动器形态 command 被判 COMMAND_BARE，strict也0，待边界fixture修正。D7代码未加入--fix/SARIF/host或发布流程，双crate仍可clone后cargo build；不执行crates.io发布。

3800a61 的 [CI 35596534564](https://github.com/AkaraChen/agent-plugin-lint/actions/runs/35596534564)：Ubuntu、Windows全套clippy/test通过，fmt通过；Windows schema原字节哈希与 `./${PLUGIN_ROOT}` 展开路径的明确IO/unchecked断言均通过。macOS schema与MCP通过，s2b后续fixture创建时拒绝非法UTF-8文件名（EILSEQ 92），不是引擎误报。Unix正例改用canonical ROOT构造后仍精确要求containment Pass；Windows无效OS路径测试精确要求exit2/MCP_PATH_IO/Unchecked且无escape违规。非递归展开正例拆分后在所有平台保留并增加Pass证据断言。

macOS runner不允许创建非法UTF-8目录项，真实collection遇到该目录项的扫描场景在该runner未验；不将文件系统拒绝创建当成扫描通过。输入参数本身的非法UTF-8则无需创建目录，可直接验证PATH_ENCODING/exit2。Linux仍保留真实非法目录项扫描。

cbb121d 的 [CI 35596940005](https://github.com/AkaraChen/agent-plugin-lint/actions/runs/35596940005)：Ubuntu/Windows及fmt通过；macOS的plugin crate全部通过，skills库另一个同类非法UTF-8目录fixture在创建阶段返回EILSEQ。下一片将目录名编码检查前置于读取，并将断言强化为精确NonUtf8Directory；不依赖创建macOS文件系统禁止的名字。

身份增强片的中途review曾退回一个未提交实现：BeforeOpen hook被移到正文读取句柄打开之后，虽保留原InputChanged断言文字，却削弱FIFO替换攻击时序。astra要求持有独立身份guard，实际正文句柄仍在BeforeOpen之后打开，再比较身份；FIFO测试额外证明到达AfterOpen。初始guard打开已通过包含检查的canonical目标，避免再次跟随可被重定向的逻辑链接。Windows原生API失败须立即取last error，能力不支持与普通IO分开。此退回不作为验收结果。

498964f 的 [CI 35599455538](https://github.com/AkaraChen/agent-plugin-lint/actions/runs/35599455538) 首次三平台全绿，fmt亦通过。astra此前亲自执行workspace test/clippy/fmt/diffcheck通过。Windows原生日志明确运行并通过 `windows_file_id_distinguishes_same_length_same_mtime_files`（直接比较volume+128bitID）、`detects_same_length_changes_with_restored_mtime`（BeforeOpen/AfterRead替换）、原先不同长度替换/改写、两项Unsupported报告测试。macOS整个workspace通过，非UTF8入口错误、ELOOP与FIFO分支已原生执行。此时仍有review未收口：原地同长度恢复mtime用例、Unsupported后继blocked及FIFO有界watchdog。不能把这次全绿当作这些缺口已验。已更换新的Terra会话限定收口。

本片曾尝试本机GNU Windows cross-check，但未安装该target，E0463；不算Windows证据。上述Windows证据来自原生MSVC runner。

d746b84 收口片经astra独立workspace test/clippy/fmt/diffcheck通过后推原生CI。补齐BeforeOpen替换、AfterRead替换、AfterRead原地改写三例，增加FIFO 2秒watchdog及Unsupported完整JSON/独立组件证据。重构时曾丢失旧MCP JSON_REPRESENTATION入口记录，astra指出后已恢复并强化回归。

[CI 35600630893](https://github.com/AkaraChen/agent-plugin-lint/actions/runs/35600630893)：Ubuntu、macOS、fmt通过；Windows新增的AfterRead原地同长度改写/恢复mtime断言失败（14过1失败），其他新回归及Unsupported传播通过。原日志只有matches失败，没有实际Result，因此先补诊断原生复跑，不凭Linux成功猜Windows结果，更不能删该用例或延时避开它。

astra在隔离源码副本中只将O_NONBLOCK改为0，FIFO substitution单测在2.00秒由watchdog失败（cargo exit101，预期负向结果），进程正常结束。该次使用共享target导致随后的正常树单测误复用了mutant产物；源码未变。已删除副本并clean两个本地crate的构建缓存，重新执行完整测试/clippy；以重建后的结果为准。此负向注入仅Linux做过。

诊断提交d8950af的[原生CI 35601200199](https://github.com/AkaraChen/agent-plugin-lint/actions/runs/35601200199)再次仅Windows失败，精确输出`Ok("one")`；改写前后volume/FileId/ChangeTime/len/mtime全部相同。现有元数据门禁确实漏检正文变化，非错误类别误判。astra据此设计已打开句柄的二次内容确认，并要求去除额外诊断查询后保持原攻击时序复验。

内容确认片已去掉额外Windows诊断查询，三条同长度/恢复mtime的精确InputChanged断言均保留。astra再次亲自执行 `cargo fmt --all -- --check`、`cargo test --workspace --locked`、`cargo clippy --workspace --all-targets --locked -- -D warnings`、`git diff --check` 均过，原生CI结果另记。

追加README任务由astra撰写，71cda37直接推main，仅双语README变更。安装命令在614f45d源码的隔离worktree执行 `cargo install --path crates/agent-plugin-lint --locked --offline --root <临时目录>` 成功；安装二进制扫描frontend真实语料，README输出节选逐字匹配、退出1。README保留当时main的Linux已验/其余未验状态，安全读取改动仍仅在未合并PR分支。main已合入ci/platform-matrix。
