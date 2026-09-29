# Dating Copilot 进度台账

规则：不得仅凭"代码已写"宣称完成（`AGENTS.md` Definition of Done）。每条成果必须绑定可复跑的验证命令。

## 2026-09-30 总监轮：P0 确定性核心落地

### 已完成并验证
- `crates/core`（crate 名 `dating-core`，lib `dating_core`）：零外部依赖、可离线构建的标准库实现。`Cargo.lock` 只含本 crate 自身，已核。
  - `canonical.rs`：`Direction`/`MessageType`/`CanonicalMessage`（含 `timestamp_millis`）、`validate()`、`compute_dedupe_key()`（长度前缀 FNV-1a 64，16 位十六进制；注释已写明仅作去重身份、不具安全含义）、`DedupeIndex::insert()` 先校验后入库。
  - `evidence.rs`：`FactType` 含 `SensitiveSpeculation` 且在插入处机器拒绝（把 AGENTS.md 第 10 条编码成门禁，而不是写在注释里）；`Ledger::add_evidence/add_inference` fail-closed（无源证据拒收、置信度 >100 拒收、无支持证据的推断拒收、引用未知 evidence id 拒收并点名是哪个 id）；`ground_check()` 返回 `Option<Groundedness>`，未知推断 id 得 `None` 而非编造裁决。
  - `state.rs`：`Stage` 十级阶梯 + `rank()`、`Trend`、`Proposal`、`adjudicate()`——LLM 只提议，规则裁决：无证据 id 拒、引用未知证据拒、前进只允许一级（越级拒）、后退一律放行、边界存在时禁止任何前进、提议 `Boundary` 趋势则无论阶段如何变动都带入 `Applied`。
  - `nba.rs`：九种动作齐备，优先级 STOP > COOL_DOWN > 阶段表；STOP/COOL_DOWN 路径剥离全部 `GentleProgress` 候选；理由必须带 evidence id；不输出任何"好感度百分比"式假精确。
- 验证（本人复跑，非代理自述）：`cargo fmt --all --check` exit 0；`cargo clippy --workspace --all-targets -- -D warnings` exit 0；`cargo test --workspace` **22 passed / 0 failed**（`crates/core/tests/p0_core.rs`，实现+测试共 1449 行）。
- 治理文件：`.gitignore`（忽略 `/target`、`/worktrees`、`*.log`）、`.gitattributes`（`* text=auto eol=lf`，统一 LF）。本目录下的 `worktrees/<uuid>` 是别家代理的检出，不纳入版本控制。

### 代理在文档含糊处的 5 处判断（需总指挥确认，不默认接受）
1. `ground_check` 签名：契约同时要求"返回 Groundedness"与"未知 id 不得编造裁决"，代理改为 `Option<Groundedness>`（`None`=未知）。
2. 因 `add_inference` 已拒绝未知引用，`PartiallyGrounded` 原不可达；代理据 ARCHITECTURE.md 的"会话级删除"新增 `Ledger::remove_evidence`，使证据可撤销、降级可被检测。
3. "拒绝后不得生成推进话术"实现为按 `GentleProgress` 风格门过滤，不做不可靠的文本嗅探；核心不生成回复文本，只过滤候选。
4. 平级提议（`from == to`）放行；边界趋势始终带入结果。
5. 阶段→动作基线表（如 `ActiveChat→CHANGE`、`Unknown→WAIT`）文档未定义，代理选了保守映射并保证九种动作皆可达，边界优先级覆盖它。**这张表属产品口径，应由你或 Reasoning Agent 定稿，不宜由实现侧长期持有。**

### 当前卡点
- 平台适配器（Soul/小红书/抖音）需要真实网页会话与 DOM fixture；本机无授权账号，且 douyin 线已实测缺 Android 工具链（`adb`/`java`/`HBuilderX` 均不在 PATH），V0.1 的 Android 截图→OCR 链路为 `BLOCKED_EXTERNAL`，不得记作通过。
- Prompt Registry、Claude-14 提示与 100+ eval 集尚未建立：`docs/TEST_PLAN.md` 要求的 12 类提示回归与 Groundedness/边界召回等指标需要真实模型调用授权与评测数据，属未开闸门。
- 持久层（SQLite）、localhost Gateway、MV3 扩展与 Compose 客户端均未开始；V0.2/V0.3/V1 的验收标准未逐项登记。
- 本分支 `feat/p0-deterministic-core` 仅为特性分支；按 `AGENTS.md` 分支策略需 PR/审阅后才并入 `main`，不单方面快进主线。

## 2026-09-30 总监复审：证据门禁与拒绝边界

### 本轮修正
- `CanonicalMessage` 现拒绝空白 `source_ref`；Evidence/Inference 拒绝空白 id、事实/结论和空白来源消息 id。
- NBA 不再信任外部 `explicit_rejection` / `boundary_signal` 布尔值；必须在 Ledger 找到所有引用，按 `ExplicitRejection` / `Boundary` 类型导出 STOP / COOL_DOWN。缺少或引用未知证据时返回错误、不生成建议。
- STOP / COOL_DOWN 不再留下自然或幽默回复候选；风格标签不能证明边界话术安全。
- 状态裁决使用调用方传入的可信当前阶段，不接受模型自行声明 `from`；只允许一步前进，来源证据出现边界则阻止前进并固定 BOUNDARY 趋势，未引用边界证据不能声明 BOUNDARY 趋势。Applied 结果保留证据 id。
- 测试计划、架构与回复策略文档已同步。

### 本轮验证
- `cargo fmt --all --check`：通过。
- `cargo test --workspace --all-targets`：26 passed / 0 failed。
- `cargo clippy --workspace --all-targets -- -D warnings`：通过。
- `cargo build --workspace --all-targets`：通过；`git diff --check`：通过。
- 以上仅验证 Rust 确定性核心。Prompt regression、模型 schema、Adapter→Gateway→DB 集成、网页 E2E 和 Android 截图→OCR E2E 尚未实现或验证，不能将 Dating Copilot 全项目标记验收完成。
- 此前 P0 基线已提交为 `7f22199` 并推送到 `feat/p0-deterministic-core`；本轮复审修正已另行提交到同一特性分支，推送需要重试（首次尝试时本机无法解析配置的 SSH 主机名）。

### 开源参照
- 本项目可借鉴 Goutoujunshi 的用户同意控制、可追溯来源、事实/推断/未知分离；Harness 侧调研记录及版本/许可证见 [`../codex-harness/docs/OPEN_SOURCE_REFERENCE_REVIEW_20260930.md`](../codex-harness/docs/OPEN_SOURCE_REFERENCE_REVIEW_20260930.md)。本轮没有复制第三方代码。

### 后续验收闸门
- 建立版本化 Prompt Registry、JSON Schema 校验和 `docs/TEST_PLAN.md` 规定的 12 类回归集；明确真实拒绝、一般边界、暂时忙碌的区分与误报/漏报指标。
- 设计并实现用户同意、撤回、会话删除/导出后的 SQLite 持久层；为 Adapter/Gateway/客户端拆出可独立验收的需求与跨平台 CI。
- 真实 API/网页账号、Android 工具链和设备 E2E 在可用前列为外部验收项；任何单元测试通过不能替代这些门禁。
