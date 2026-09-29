# Prompt & Model Design

Prompt Registry 分为 system / evidence / vision / reasoning / relationship / action / review。每个 Prompt 记录 id/version/schema/model-family/eval-set，禁止维护不断膨胀的超级 Prompt。

## 模型路由
Fast/Cheap：OCR纠错、分类、事实提取、摘要。Strong Reasoning：Claude-14、复杂歧义、多假设、长期趋势、回复。Critic：Ground check，可使用独立模型降低同源偏差。

## Claude-14 工程协议
1 可观察事实；2 事实/解释分离；3 支持信号；4 反证；5 替代解释；6 未知项；7 时间顺序；8 历史基线；9 主动性/互惠；10 回复投入/话题延展；11 边界/拒绝；12 阶段/趋势变化；13 最小风险可逆 NBA；14 Ground check：结论绑定证据，否则降级/删除。

“Claude-14”是 provider-neutral protocol。若项目取得既定14条权威原文，应作为新版本导入，不凭印象覆盖此工程基线。

## Vision
Vision 只回答“看到了什么”：平台、消息气泡、发送方、文本、时间、回复关系、UI状态；禁止直接从截图得出“对方喜欢用户”。

## Reply
普通情境可提供 natural / humorous / gentle_progress 候选；不编造共同经历。明确拒绝或其他边界信号时，确定性核心输出 STOP/COOL_DOWN 并清空所有回复候选，不能仅凭“自然/幽默”等风格标签判断话术安全。用户若请求回复边界信息，后续需单独设计并回归测试明确的边界确认流程。
