# ap-lint 决策日志

> `PLAN.md` 是 v1 初稿，`DESIGN.md`/`rules.md` 是 astra 的细化版（**基于 Node**，语言一改就整块作废，
> 待按 Rust 重做）。本文件记录**已拍板的决定**，是当前唯一的真源。

## 已拍板

| # | 决定 | 日期 | 说明 |
|---|---|---|---|
| D0 | **私有 skill 起步**，引擎从第一天按「可发布」的姿态写（稳定 JSON / 退出码契约） | 2026-09-21 | 飞鸢「方向是对的」；未明确否决 |
| — | **不做 `--fix`**，工具只读 | 2026-09-21 | 飞鸢 + astra 一致 |
| D1 | 两份 schema 正文已取得并**按 git blob 校验**，落在 `research/schemas/` | 2026-09-21 | 取证链：contents API 的 `sha` + `download_url` → `sha1("blob <len>\0"+content)` 比对 |
| — | 语言 = **Rust** | 2026-09-21 | 飞鸢定。astra 的「零依赖单文件 Node ≥20」整块作废 |
| D2 | L3 走 **C**：先修 `AkaraChen/skills-ref`，再作为**库依赖**接入；我们只做薄适配 | 2026-09-21 | 飞鸢拍板 |
| D2″ | **不再走跨仓库依赖**：skill linter **并进 agent plugin linter 一起维护**，改法由我定；`AkaraChen/skills-ref` **archive** | 2026-09-21 | 飞鸢：正确性优先 |
| — | **正确性优先**（压倒发布/便利/零依赖之类的取舍） | 2026-09-21 | 飞鸢 |

## D2 的由来（为什么不是直接依赖）

对 `AkaraChen/skills-ref` 0.1.0 做了源码审计 + 实测（`cargo build --release`，1m53s），
发现两条**真实误报**——linter 最不能有的失败模式：

1. **字节当字符**：`description.len()` / `name.len()` / `compatibility.len()` 是 UTF-8 字节，
   规范上限是 characters。400 汉字 description（1200 字节）→ 误报 `exceeds 1024 character limit (1200 chars)`。
   修：`chars().count()`。
2. **`metadata` 被判非法**：`ALLOWED_FIELDS` 缺规范允许的 `metadata` →
   带 `metadata:` 的 skill 一律 `Unexpected fields`。

另：`validate() -> Vec<String>`，`ValidationError` 枚举 validator 里没用上 → 无 rule id / 无位置 / 无严重度。
crates.io 上 `skills-ref` **不存在** → 只能 git/path 依赖，或先发布。

fixture：`/tmp/fx/{ascii-ok,cjk-desc,metadata-field}`（ascii-ok 通过，另两个误报）；crate 克隆在 `/tmp/skref`。

## 待决

- **D2′**（当前问的）：`validate()` 结构化输出的接口形状
- D3 未实现的 extension 值怎么处理
- D4 CI 默认对 `ignored` 的 MUST 是否返回 1
- D5 原文欠定的语法边缘（token/loopback/env 大小写）
- D6 Agent Skills 无冻结版本 + NFKC/Unicode 分歧裁决
- D7 L6 宿主矩阵与发行范围
- D8 支持平台承诺

## 待办

- [x] ~~给 `AkaraChen/skills-ref` 提 patch~~ → 改为**把它整体吸收进新仓库**（D2″）
- [ ] 新仓库形态确认 → 建库 → 把 skills-ref 移植进来（含两条 bugfix + 结构化 issue + 按字符计数 + 换掉弃用的 YAML 后端）
- [ ] 新仓库建好后再 archive `AkaraChen/skills-ref`，并在旧 README 顶部留指路（归档后仍可读，别让搜到的人用旧的）
- [ ] astra 按 Rust 重做 `DESIGN.md` / `rules.md`

## 已核实的命名/工具事实（2026-09-21）

- crates.io 空闲：`ap-lint` / `agent-plugin-lint` / `plugin-lint` / `ap-linter`；占用：`apx` / `agent-plugins`
- 本机 `gh` 已登录 AkaraChen（scopes 含 `repo`）→ 建库/归档都能做
- 本机 rust：cargo 1.98.0 / rustc 1.98.0

---

## D3–D8 定案（2026-09-21，飞鸢授权「都按你推荐的来」）

| # | 定案 | 落地 |
|---|---|---|
| **D3** 未实现的 extension 值 | **保守**：不校验未实现 namespace 的 value；只判「明确非法的 namespace 形态」；**禁止**把未实现当作作者侧违规 fatal | 现状即定案，无代码变更 |
| **D4** 默认政策 | **采用**：确定的包侧 normative MUST → 退出 1（**含 ignored**）；只有 advisory → 0；`--strict` 再拦 advisory 与选定静态规则的 unchecked。要「加载可用性」视角的消费者从 JSON 的 `radius` 自行过滤，不混进默认 policy | 现状即定案 |
| **D5** 欠定语法边缘 | **保守**：`command` token 字符语法 / loopback 接受域 / env 大小写差异（Windows 不敏感）一律标 unchecked 或 advisory，由 `--strict` 拦截；边界例子固化进 fixture；正文澄清后再升级 ruleset | 现状即定案 |
| **D6** AS 快照与 Unicode | **保守**：不做静默 NFKC；非 ASCII 仅规范化差异不擅自拒绝；未知 frontmatter 字段不按白名单定罪；须记录所依据的 AS 文本 | ✅ **已满足**：`research/agent-skills-specification.md` + `research/PROVENANCE.md`（SHA-256 + 获取日期，并标明「无版本号」） |
| **D7** 宿主矩阵与发行范围 | **L6 不进 v1**；本轮不含 crates.io 发布、SARIF、`--fix`、旧库归档。发行目标 = 「clone 下来 `cargo build` 就能用」 | 现状即定案 |
| **D8** 平台承诺 | **Windows 提上日程**（飞鸢 2026-09-21）：改用 GitHub Actions 的 windows runner 实测，不再只写「未验证」 | 见下 |

### D8 的推进方式

`.github/workflows/ci.yml`：三平台矩阵 `ubuntu-latest` / `macos-latest` / `windows-latest`，
各自 `cargo clippy --workspace --all-targets --locked -- -D warnings` + `cargo test --workspace --locked`；
另有独立的 `fmt` job（ubuntu，避免三平台重复）。

外部 action 版本于 2026-09-21 实时核对上游 release：`actions/checkout@v7.0.1`、
`actions-rust-lang/setup-rust-toolchain@v2.0.0`、`Swatinem/rust-cache@v2.9.2`。
**不用 `dtolnay/rust-toolchain`**：它无 release，只有一个 2022 年的浮动 `v1` tag。
`actionlint` 1.7.12 对改动文件零诊断。

**规则：Windows 通过之前，README 不宣称支持 Windows。** 通过后再按实测更新平台承诺与 MSRV。

### 已知缺口（随本轮 CI 上线）
- Windows 从未跑过本仓库的测试。symlink 相关语料（含 eric-way 那三条逃逸靶子）在 Windows 上可能需要
  特权或 Developer Mode —— 这是假设，待 CI 结果证实或推翻，不要先行写进文档。
- `crates/agent-plugin-lint` 依赖 `libc`（Unix 向），Windows 上的可编译性是待验项。
