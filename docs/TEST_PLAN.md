# Test & Evaluation Plan

Unit：CanonicalMessage（含非空来源引用）、dedupe、debounce、state transition（可信当前状态）、evidence ledger（非空证据来源）、schema validation；NBA 对缺失/未知证据 fail-closed，边界/拒绝证据压过阶段映射且不产生回复候选。
Integration：Adapter → Gateway → DB → Engine；LLM mock 与失败恢复。
E2E：网页 fixture 新消息 → 自动分析 → UI建议；Android截图 → OCR → 建议。

Prompt regression 至少覆盖：polite rejection、reciprocal interest、short reply ambiguity、busy-not-cooling、concrete invitation、cancelled date、explicit boundary、sarcasm、long-term conversation、ambiguous signal、OCR error、duplicate messages。

指标：Groundedness、unsupported-inference rate、state consistency、boundary recall、action usefulness、reply naturalness、latency、token cost。Prompt 改动保存 old/new 对比；关键指标退化则阻止合并。
