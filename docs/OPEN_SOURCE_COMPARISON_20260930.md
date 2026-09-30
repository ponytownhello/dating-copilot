# Dating Copilot 开源项目对照审计（2026-09-30）

## 审计对象与方法

本地产品检出：`H:\Demo_Git\dating-copilot`，分支 `feat/p0-deterministic-core`，HEAD `f87d05f`（与该分支 origin 一致）。检查了 `AGENTS.md`、`docs/ARCHITECTURE.md`、`docs/PRIVACY.md`、`docs/PROMPT_DESIGN.md`、`docs/ROADMAP.md`、`docs/TEST_PLAN.md`，以及 `crates/core` 的 canonical/evidence/state/NBA 实现和测试。

另将最贴近 Rust companion/relationship state 的 [eros-engine](https://github.com/etherfunlab/eros-engine) 浅克隆到系统临时目录作只读源码审阅，快照为 `9cb85a0c1703c6e038f780e151aea6229c693a7e`（上游提交时间 2026-09-22，版本 1.9.3）。本次没有在这个第三方仓库安装依赖或运行测试；对 Goutoujunshi 与 trust-dating 的代码审阅依据本轮总审计此前保存的本地快照；Resonant 与 Dating Coach 只作 GitHub README/许可证页面审阅。下列建议是设计结论，不表示第三方项目的宣传功能已经独立验证。

## 对照结论

| 项目 | 本次看到的长处 | 可转成 Dating Copilot 的需求 | 不应照搬/限制 |
|---|---|---|---|
| [eros-engine](https://github.com/etherfunlab/eros-engine)（Rust，AGPL-3.0-only） | 分 crate 隔离 domain/core、模型访问、持久化和 HTTP；规则优先的 Persona Decision Engine；记忆与关系状态独立建模；按任务选模型、provider fallback 与调用审计 | 把 Dating Copilot 按 Core、Persistence、Provider/Prompt、Gateway、Client/Adapter 拆边界；将生成请求审计元数据与确定性裁决分离；为模型失败/JSON schema 失败建立“不写入状态”的回退契约 | 它面向 AI companion，包含 affinity/ghost/沉默不回复等产品行为，与“尊重真实对象边界、用户作最终决定”的 dating coach 不同。Postgres/pgvector、OpenRouter 默认配置也不是本项目必须的部署形态。AGPL-3.0-only 有强 copyleft 义务；本轮只学机制，不复制代码或引入依赖。 |
| [Goutoujunshi](https://github.com/shengjidaguai-china/goutoujunshi)（MIT；Codex Skill） | 明确首次同意后才启用长期记忆；可查看、暂停、撤回、清空；关系分析将事实、假设、未知项分开，保留观察窗口和停止条件 | 把 consent 状态/版本与联系人或会话范围绑定；让用户能查阅、暂停、撤回、导出、删除；每条记忆保留来源、时间、事实/推断类别、反证和删除影响；建议输出观察窗口与停止条件 | 它是提示词/Skill 工作流，不等于具备数据库事务、删除覆盖、模型评估或多端 E2E 的应用后端；不能仅凭 README 宣称当作实现验收。 |
| [trust-dating](https://github.com/abhi-yo/trust-dating)（GPL-3.0） | Windows/macOS 桌面交互、本地 BYOK、多 Provider、自托管数据、短历史 | 后续客户端需要清楚显示当前 Provider、数据去向、发送范围与本地历史删除；把 Provider 配置与纯领域核心隔离 | 桌面剪贴板监控/自动化会扩大敏感聊天的采集范围；风险检测/“catfish”输出不得当作事实。GPL-3.0 代码不复制。 |
| [Resonant](https://github.com/codependentai/resonant)（Apache-2.0） | 本地 SQLite、可编辑 persona、对话全文/语义搜索、按 thread 控制上下文 | 参考本地优先存储、可读配置、按会话检索范围控制；用户删除应覆盖全文索引、向量索引与派生数据 | README 明确绑定 Anthropic Claude Agent SDK，不是多 Provider 实现；其“AI partner”身份与关系分析产品不同。本轮不引入依赖。 |
| [Dating Coach](https://github.com/connecttoriyan/dating-coach)（README 展示 Next.js/FastAPI/Claude/Supabase/Vapi 混合方案；未在本轮核实到明确许可证） | 文本教练与语音练习分离，展示了后续可选训练/复盘入口 | 可把语音练习列为独立可选里程碑，另立音频同意、保留时限和转写删除验收 | 后端依赖托管数据库/语音服务，和本地优先目标不一致；没有明确许可前不复制源码。 |

## 本地实现复审

目前分支已落地的是零外部依赖、内存运行的 Rust 确定性核心，不是可交付的完整 App：

- `canonical.rs` 校验来源、内容、时间戳和字段化去重身份；`evidence.rs` 将来源证据和推断分开，拒绝无来源证据、未知引用及敏感臆测；`state.rs` 用确定规则裁决 LLM 可提议的阶段变化；`nba.rs` 让拒绝优先于一般边界、边界优先于阶段建议，STOP/COOL_DOWN 清空所有回复候选。
- 这些门禁与 Goutoujunshi 的证据边界目标相符；其可执行性由本地 26 个单测/集成式 crate 测试确认。Harness 的 P0 验收还没有覆盖 Prompt Registry、真实 Provider、数据库、Adapter、客户端或端到端流程。
- `Ledger::remove_evidence()` 当前只删 evidence，保留引用它的 inference，并将其变为 partially-grounded/unsupported。这个行为适合“证据过期后仍展示审计痕迹”，但不能同时被称为彻底隐私删除。产品必须拆开两种操作：**撤销/失效证据并保留脱敏审计痕迹**，以及**用户要求删除时级联清除相关 inference、状态快照、索引、缓存和来源消息**；后者需要持久层事务与跨模块删除测试。不得将现有内存方法标成完整联系人/会话删除。
- `docs/PRIVACY.md` 和 `docs/ROADMAP.md` 已规划用户控制、SQLite、Provider 与客户端，但核心仓库目前没有 consent 记录、持久化、Provider trait、Prompt Registry、schema 校验、Android、Gateway、MV3 Adapter 或实际 E2E。这些必须留在未完成清单。

## 采纳顺序与闭环验收

| 优先级 | 需求/交付 | 完成判据 | 必须验证 |
|---|---|---|---|
| P0.1 | 同意和删除语义 | 按 user/contact/conversation 范围记录 consent、用途、版本、时间；拒绝或撤回后不再做记忆读取/写入；删除清除原消息及所有派生状态/索引/缓存。导出能追溯字段来源 | SQLite migration；拒绝、暂停、撤回、联系人删除、会话删除、重启后不复现；故障注入验证事务回滚或可恢复删除 |
| P0.2 | 持久化领域模型 | CanonicalMessage、Evidence、Inference、StateProposal/Applied、NBA、PromptRun 各自有 schema 和来源键；UI/Adapter 不直接写关系状态 | repository 集成测试、重启/迁移/并发写入测试、未知外键 fail-closed |
| P0.3 | Provider 与 Prompt Registry | 每个 prompt 有 id/version/schema/model-family/eval-set；JSON/schema 校验通过才可进入裁决；Provider 失败、超时、拒绝或解析失败不修改关系状态；密钥不进日志 | mock Provider 契约测试、错误注入、密钥日志扫描、版本回归比较 |
| P1 | 证据与边界 eval | 建立 `docs/TEST_PLAN.md` 的 12 类场景并扩展到至少 100 条固定 eval；测 groundedness、unsupported-inference、拒绝/边界召回、误报、动作有用性；关键指标回退阻止晋级 | CI 固定数据集回归；prompt 变更 old/new 报告；拒绝与一般不适分开统计 |
| P1 | 导入与平台 Adapter | Android 截图/OCR、Chrome/Edge Adapter 只生成 CanonicalMessage；用户明确选择数据后才上传给模型；DOM 选择器集中配置、有 fixture | Adapter fixture、去重/时序/说话人映射、权限/撤销、匿名日志测试 |
| P1 | 人机交互 | 先完成一个可用客户端和可见 Evidence/Ground Check/边界理由；默认只给可复制建议，不自动发送 | 真实 API 测试与 UI E2E；拒绝、未知、OCR 错误、网络断开均展示安全降级 |
| P2 | 可选语音/跨端 | 独立开关、单独 consent、可配置转写/音频保留和删除，不阻塞本地文字 MVP | 设备/服务集成测试、音频彻底删除证明、成本/延迟门槛 |

## 裁决

保留 Dating Copilot 当前“证据账本 + 确定性状态/NBA”作为安全内核。先补齐隐私删除语义、持久化和 prompt/provider 契约，再做可见客户端与 Adapter；不要把它替换为 companion engine，也不把第三方 affinity、心理分类或自动发消息接入决策链。代码复用只在单独许可证审查、依赖审计和兼容性确认后考虑；当前结论一律为**借鉴架构与需求模式，不拷贝源码**。

## 复核记录

- Dating Copilot：`cargo fmt --all --check`、`cargo clippy --workspace --all-targets --offline -- -D warnings`、`cargo test --workspace --offline`、`cargo build --workspace --all-targets --offline` 均通过；测试 **26 passed / 0 failed**。这只验收确定性 Rust 核心。
- 上游源码审阅：eros-engine 本地浅克隆 HEAD `9cb85a0c1703c6e038f780e151aea6229c693a7e`；静态审阅，未运行上游测试。
- 本报告日期：2026-09-30。最新性复核需下次巡检重新 fetch 上游 HEAD 和许可证，不得把此快照说成永久最新。
