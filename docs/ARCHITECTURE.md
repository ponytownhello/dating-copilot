# Architecture

```text
Android App / Browser MV3 / Screenshot
→ Message Gateway → CanonicalMessage → SQLite
→ Evidence Engine → Relationship Memory
→ Claude-14 Engine → Ground/Critic
→ State Machine + Trend → Next Best Action
→ Advice + Reply Candidates
```

## CanonicalMessage
最低字段：id, platform, conversation_id, participant_id, direction, timestamp, type, content, source_ref, dedupe_key。

## Adapters
`adapters/soul`、`adapters/xiaohongshu`、`adapters/douyin` 只负责 DetectConversation / ExtractMessages / ObserveNewMessages / ExtractContact / Metadata。浏览器优先 MutationObserver，DOM selector 集中配置并使用 fixture 回归。Android 输入为 Screenshot、Share、NotificationListener，后续评估 Accessibility；全部归一化为 CanonicalMessage。

## Evidence Ledger
Evidence 保存 source_message_ids、fact、type、confidence。空白 source/message/evidence id 会被拒绝；`ExplicitRejection` 与一般 `Boundary` 是不同类型。Inference 保存 supporting_evidence_ids、counter_evidence_ids、unknowns；无来源事实不得进入长期记忆。

## Relationship
Stage 与 Trend 分离。Stage: UNKNOWN → ACQUAINTANCE → ACTIVE_CHAT → FAMILIAR → WARMING → FLIRTING → INVITATION_READY → DATE_PLANNED → DATED → POST_DATE。Trend: WARMING / STABLE / COOLING / BOUNDARY。LLM 只提议目标阶段；裁决器从可信当前状态出发校验一步前进，并对被引用的边界证据拒绝前进、强制 BOUNDARY 趋势。

## Next Best Action
WAIT / CONTINUE / CHANGE / FLIRT / INVITE / CONFIRM / FOLLOW_UP / COOL_DOWN / STOP。输出 action、reason、evidence_ids、avoid、reply_candidates。NBA 必须能在 Ledger 中找到全部被引证据；STOP/COOL_DOWN 不输出任何回复候选。

## Automation
新消息 3–8 秒 debounce → 合并同轮 → 去重 → 快模型提取 → 必要时强模型推理 → 状态更新 → App/扩展显示建议。

## Security
本地优先；API key 安全存储；日志脱敏；截图可配置分析后删除；支持联系人/会话级删除和导出。
