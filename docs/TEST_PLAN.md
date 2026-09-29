# Test & Evaluation Plan

Unit：CanonicalMessage、dedupe、debounce、state transition、evidence ledger、schema validation。
Integration：Adapter → Gateway → DB → Engine；LLM mock 与失败恢复。
E2E：网页 fixture 新消息 → 自动分析 → UI建议；Android截图 → OCR → 建议。

Prompt regression 至少覆盖：polite rejection、reciprocal interest、short reply ambiguity、busy-not-cooling、concrete invitation、cancelled date、explicit boundary、sarcasm、long-term conversation、ambiguous signal、OCR error、duplicate messages。

指标：Groundedness、unsupported-inference rate、state consistency、boundary recall、action usefulness、reply naturalness、latency、token cost。Prompt 改动保存 old/new 对比；关键指标退化则阻止合并。