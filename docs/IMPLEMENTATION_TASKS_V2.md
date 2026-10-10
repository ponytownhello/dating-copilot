# Dating Copilot V2.0 — Codex 执行任务与验收

## P0 — 基础
- [ ] 定义 CanonicalMessage、Evidence、AnalysisVersion、SyncBatch、ContactLink JSON Schema
- [ ] SQLite migrations、FTS5、来源索引、去重/幂等/冲突处理
- [ ] 导入 CSV/JSON/截图 OCR 的最小可运行路径
- [ ] FastAPI /sync/messages、/sync/status、/analysis/realtime、/contacts/{id}/memory
- [ ] Provider Router：模型/额度/失败回退配置，不写死免费套餐
- [ ] Skills：evidence-extractor、claude14、doghead、ground-check
- [ ] Android：悬浮球、MediaProjection 授权、OCR、复制建议、暂停采集
- [ ] Windows：Service 调度 + 用户会话采集 Agent，实时监听可用平台，定时补偿
- [ ] 本地与云端隐私设置、会话级删除、凭证安全存储

## P1 — 完善
- [ ] 平台适配器能力矩阵（抖音、Soul、小红书、微信、QQ）
- [ ] 断网/重启/重复上传/乱序到达/晚到历史的回归测试
- [ ] 分析版本化与增量重算
- [ ] 三档 Agent 调度、来源审查、人工裁决
- [ ] Vue 3/PWA 浏览器、搜索、画像、同步与隐私管理
- [ ] Advice Feedback、24项指标的计算与局限说明
- [ ] PythonAnywhere 实际部署探测与 VPS fallback

## P2 — 扩展
- [ ] 更多风格 Skills 与多模型评测
- [ ] Windows 桌面悬浮窗
- [ ] Android 变化检测优化与通知监听（需独立授权）
- [ ] 大规模数据性能评测、可选向量检索

## Harness / Definition of Done
每个功能需有：设计、schema、测试数据、单测、集成测试、相关 E2E、Prompt eval、隐私审查、CI 验证。不得声称通过未运行的测试。平台能力必须先实测再承诺。