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
