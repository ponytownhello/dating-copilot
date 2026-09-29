# Dating Copilot

跨平台 AI 约会/社交副驾。目标不是“自动代聊”，而是从 Soul、小红书、抖音等聊天上下文中提取可验证事实，维护长期关系状态，并给用户明确、可解释的下一步建议。

## 核心闭环

```text
Soul / 小红书 / 抖音 / Android截图与通知
                  ↓
        Platform Adapter / Vision
                  ↓
            CanonicalMessage
                  ↓
 Evidence → Memory → Claude-14 Reasoning
                  ↓
       Ground Check / Counter Evidence
                  ↓
 Relationship State + Trend
                  ↓
            Next Best Action
                  ↓
 自然回复 / 幽默回复 / 轻推进 / 邀约 / 等待 / 风险提醒
                  ↓
              用户确认发送
```

## 产品原则

1. Evidence first：事实与推断严格分离。
2. 不输出“好感度 87%”式假精确结论；展示证据、反证、未知项与趋势。
3. Claude-14 是模型无关的 reasoning protocol，可由 Claude/GPT/Gemini/GLM/DeepSeek/Qwen 执行。
4. 自动采集、自动分析、自动生成建议；默认不自动发送，保留 Human-in-the-loop。
5. 平台适配与 Dating Core 解耦；网页 DOM 变化只影响对应 Adapter。
6. 聊天数据本地优先、最小化保存、可删除、敏感字段加密。
7. 所有 Prompt 版本化；Prompt 变更必须跑回归评测。

## 推荐技术栈

- Android：Kotlin + Jetpack Compose + Material 3
- Web：Chrome/Edge Manifest V3 Extension
- PC Gateway：Rust + localhost HTTP/WebSocket
- Storage：SQLite（Android 可用 Room）
- OCR/Vision：ML Kit + 可插拔 Vision LLM
- AI：Provider Router，便宜模型做提取/分类，强模型做复杂推理
- Tests：Unit + Integration + Adapter fixtures + Prompt Evals + E2E

详见 `docs/`、`AGENTS.md` 与 `skills/`。
