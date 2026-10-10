# 部署与实时/延迟混合同步规范

## 运行模式
Windows 开机启动服务；用户登录后启动用户会话采集器；可监听则实时入库，无法监听则可配置周期扫描，离线缓存与重启后补偿。Android 通过用户授权的悬浮窗截屏独立实时分析。
云端异步处理新消息、增量记忆更新和延迟到达的历史消息；手机实时建议不能依赖 PC 在线。

## API 草案
- POST /api/v1/sync/messages — 幂等批次上传，返回 accepted/duplicate/rejected IDs
- GET /api/v1/sync/status — 水位线与任务状态
- POST /api/v1/analysis/realtime — 当前消息与联系人上下文，返回证据化回复
- POST /api/v1/analysis/deep — 创建深度分析任务
- GET /api/v1/contacts/{id}/memory — 有权限的历史记忆查询
- DELETE /api/v1/contacts/{id}/data — 删除联系人相关数据及派生索引

## 状态
SyncBatch: queued → uploading → acknowledged；失败 retry/backoff，服务端 ACK 才推进本地水位线。消息 sent_at/captured_at/ingested_at 分离；时间戳统一 ISO8601 含时区。跨设备来源保留 provenance。
晚到历史触发 affected-window 重算，旧分析标 superseded，不静默覆盖。

## PythonAnywhere 可行性验收
1. FastAPI 部署与 HTTPS 鉴权。
2. 所需国内/国际模型 API 实测可达。
3. 定时任务/后台 Worker 在具体套餐内可用。
4. SQLite 并发、备份恢复、磁盘配额。
5. Android/PC 双端断网恢复、重复上传与延迟重算 E2E。
若不可用，迁移同一 FastAPI 项目至 VPS；禁止把部署平台写入核心业务逻辑。