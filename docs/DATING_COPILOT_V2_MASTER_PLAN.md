# Dating Copilot V2.0 — 总体建设方案

状态：架构基线草案；实施需通过平台可行性验证、隐私审查和自动化测试。

## 1. 定位与目标
跨平台聊天归档、证据驱动关系分析、AI 沟通建议系统。完整闭环：
聊天导出/实时采集 → CanonicalMessage → 去重归档 → 三级长期记忆 → Claude14/多 Skills → 狗头军师/多 Agent 裁决 → 人工发送 → 后续互动复盘。
支持抖音优先，预留 Soul、小红书、微信、QQ 接口。不得假定任一平台提供完整可用的网页聊天或导出能力。

## 2. 四端职责
| 端 | 责任 | 技术 |
|---|---|---|
| Windows 11 | 开机自启、实时优先监听、定时补偿、历史导入、断网缓存、恢复补齐 | .NET 10 Worker + 用户会话 Agent、Python/Playwright、SQLite |
| Android | 授权 MediaProjection 截屏、本地 OCR、悬浮窗、实时建议、复制 | Kotlin、Compose、WindowManager、ML Kit、Room |
| Cloud | HTTPS 同步、统一消息、FTS5、记忆、Skills、模型路由、审查与复盘 | FastAPI、SQLite、可替换 Worker |
| Web/PWA | 会话搜索、画像、分析报告、同步状态、模型配置、隐私管理 | Vue 3、TypeScript |

Windows Service 不直接操作用户桌面；用户登录后启动独立会话采集 Agent。PC 开机即启动服务，但网页/客户端未登录、锁屏截图不可用或平台无接口时，不能保证实时获取。Android 在 PC 离线时独立工作。

## 3. 采集与同步
优先：官方导出 → 经审查的开源导出 → 用户授权可访问网页 DOM → 用户截图/文件导入。不得破解数据库、绕过登录或抓取无权访问的私人数据。
PC 实时事件监听为主、可配置定时扫描为辅；离线期间可获取的消息恢复后补齐。Android 只在用户授权的屏幕采集会话内运行，并可随时暂停。
消息结构：platform, conversation_id, message_id, sender_id, direction, type, content, media_ref, sent_at, captured_at, ingested_at, source, confidence, dedupe_key, source_ref。
服务端按幂等键确认批次；PC 仅在 ACK 后推进同步水位线；保留重试队列与冲突审计。跨平台联系人匹配须人工确认。
PC 延迟补齐可能改变旧分析：保留分析版本和原始证据，按受影响时间范围增量重算。

## 4. 记忆与画像
短期：最近20–100条相关消息；中期：数周话题/事件摘要；长期：有证据或用户确认的偏好、边界、重要事件。SQLite FTS5 优先，向量检索后置。
画像按互动行为、情感表达、关系发展、沟通质量四组共24项可观察指标；每项记录样本范围、算法、缺失与解释限制。不得将主动发起率直接解释为好感概率，也不得臆测疾病、智商、财富等敏感属性。
删除消息应级联清理派生摘要、索引与云端副本，并审计清理结果。

## 5. Skills 与 Agent
P0：conversation-import、screenshot-parser、evidence-extractor、memory-update、claude14-analysis、doghead-reply、psychology-review、communication-review、ground-check、next-best-action。
P1：jev-strategy、tongjincheng-style、intelligence-analysis、invite-planner、advice-feedback、trend-analysis。
P2：game-theory、pickup-research、redpill-critique（仅用于识别偏见、互惠与边界，不输出操纵策略）。
Claude14 是 provider-neutral 的多视角推理协议，不等于14次模型调用；当前14条是工程基线，用户若提供原版则单独版本化并核验。
JEV/泡学/红丸的来源必须区分原始观点、案例、研究与模型推测。
每条结论引用 evidence IDs；无证据标记 UNSUPPORTED。明确拒绝时 STOP/COOL_DOWN 优先。
极速模式1个生成 Agent；标准模式策略+生成+审查；深度模式按需5–8角色。默认标准，不每条消息启动所有 Agent。

## 6. 回复与复盘
狗头军师、高情商、松弛轻幽默、仙侠、甜宠、理性、冲突修复、自定义风格。至少提供推荐回复、两条备选、理由、风险、引用原文。用户自行发送；不自动代发。
Advice → UserAction → SubsequentInteraction → Feedback，区分相关性与因果关系；反馈可修正建议偏好，但不对对方建立未经验证的心理结论。

## 7. 部署与成本
云端保持 FastAPI + SQLite + 可替换 Worker，PythonAnywhere 为候选部署，不绑定平台；必须验证所选套餐的外部模型 API 可达性、后台任务、存储、并发、WebSocket 限制。MVP 采用 HTTPS 请求即可。
低成本路由：规则/本地 OCR优先，低成本模型提取与分类，复杂推理才使用强模型；价格/免费额度不得硬编码。
默认本地保留原文；云端原文同步为明确授权的可选模式。加密传输、最小权限、数据保留/导出/删除、敏感日志脱敏、密钥安全存储。若云端分析调用第三方模型，明确提示必要上下文会发送给模型提供商。

## 8. 分期交付
M1：统一消息模型、SQLite/FTS5、导入与幂等测试。
M2：FastAPI + PC开机服务/用户Agent + 实时/定时/断网同步。
M3：Android 悬浮球 + 手动授权截屏 + OCR + 三条建议。
M4：Evidence + Memory + Claude14 + Ground Check + 狗头军师。
M5：多 Agent、反馈复盘、Web/PWA。
M6：安装包、部署脚本、CI/CD、Prompt Harness、E2E。
验收：代码可构建、相关单元/集成/E2E与提示词回归通过，日志可追踪，文档同步；未验证的平台功能不得标为完成。

## 9. MVP 关键演示
PC 从合法可用的抖音会话来源采集或导入 → 本地SQLite去重 → HTTPS上传 → 云端证据与记忆更新 → Android截屏获取当前消息 → 结合历史生成狗头军师三种回复 → 用户复制发送 → 后续消息回流复盘。

## 10. 风险与决策门槛
平台登录与 DOM 稳定性；Android 屏幕采集授权与受保护画面；PC锁屏无法截图；PythonAnywhere部署限制；多端消息时间偏差；跨平台联系人误合并；私人聊天云端同步合规。每项必须有 fixture/测试、可用性矩阵、降级方案。