# AGENTS.md — Dating Copilot Engineering Constitution

## Mission
构建跨平台、证据驱动、可解释、隐私优先的 AI Dating Copilot。

## Agent 分工
- **Codex Lead**：架构、实现、测试、CI/CD、Review、E2E、重构；不要充当“恋爱大师”。
- **Claude/Reasoning Agent**：Claude-14、多假设解释、对话语义、回复自然度、Prompt设计。
- **Cheap Worker Models**（Qwen/GLM/DeepSeek 等）：OCR纠错、分类、摘要、结构化提取、批量 eval。
- **Critic/Ground Agent**：逐结论检查 evidence_id；无证据推断标记 UNSUPPORTED。

## Engineering rules
1. UI/Adapter 禁止直接实现关系推断。
2. 所有来源先标准化为 CanonicalMessage。
3. LLM 输出必须 schema validate；解析失败不得更新状态。
4. Evidence 与 Inference 分表/分类型保存。
5. State Machine 是确定性核心；LLM 提议 transition，规则层裁决。
6. Prompt/Skill 必须有 version、schema、tests/evals。
7. 每个关系结论必须能追溯到 evidence IDs。
8. 默认禁止自动发送消息；建议可复制/填入，最终发送由用户确认。
9. 对方明确拒绝或边界信号出现时，NBA 应优先 STOP/COOL_DOWN。
10. 不从有限聊天武断推断疾病、创伤、智商、财富、人格障碍等敏感/不可验证属性。

## Definition of Done
不得仅凭“代码已写”宣称完成。至少满足：build + unit + integration + relevant prompt regression + E2E（适用时）通过，且文档/schema同步。

## Branch strategy
大功能使用 feature branch + PR；小型文档修订可直接提交。避免多个 Agent 同时修改同一文件。
