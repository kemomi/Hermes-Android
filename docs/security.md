# Hermes-Android 安全设计

## 沙箱路径控制

### 白名单（允许访问）

以下路径允许文件类工具读写：

| 路径模式 | 说明 |
|----------|------|
| `/sdcard/Download/**` | 用户下载目录 |
| `/sdcard/Pictures/**` | 用户图片目录 |
| `/data/local/tmp/hermes/**` | Hermes 专用临时目录 |

> 白名单外的路径一律拒绝，除非工具 manifest 显式声明且策略引擎放行。

### 黑名单（始终禁止）

以下路径在任何情况下均不得访问：

| 路径 | 原因 |
|------|------|
| `/data/system` | 系统核心数据，修改可导致设备无法启动 |
| `/data/misc` | 系统杂项配置，含密钥与认证材料 |
| `/system` | 系统分区，只读挂载，不应尝试写入 |
| `/vendor` | 厂商分区，含固件与驱动 |
| `/proc` | 内核伪文件系统，仅 Phase 5+ 预定义节点例外 |
| `/sys` | 内核参数接口，仅 Phase 6 白名单节点例外 |

黑名单优先级高于白名单：即使某路径同时匹配两条规则，黑名单生效。

## 审批机制

### 触发条件

- 工具 manifest 中 `requires_approval: true`
- 策略规则 action 为 `require_approval`
- 风险等级 ≥ medium 的写操作

### 审批流程

1. Gateway 返回 `APPROVAL_REQUIRED` 状态
2. 审批请求写入审计日志并推送事件
3. 操作员通过 `hermes-cli approve <audit_id>` 或 Web UI 批准/拒绝
4. 批准后 Agent 携带 approval_token 重试请求
5. 审批超时（默认 5 分钟）自动拒绝

### 紧急停止开关

- `hermes-cli stop` 触发全局紧急停止
- 立即阻断所有进行中的写操作
- 后续写请求返回 `POLICY_BLOCKED` 直到手动解除
- 只读查询不受影响
- 紧急停止事件记入审计日志

## 审计日志

### 格式

每行一条 JSON 记录（JSONL），字段如下：

```json
{
  "timestamp": "2026-09-07T10:30:00.123+08:00",
  "request_id": "req-a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "session_id": "sess-001",
  "device_id": "emulator-5554",
  "tool": "android.file.list",
  "arguments_hash": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "requested_by": "hermes-agent-01",
  "status": "success",
  "duration_ms": 42,
  "audit_id": "aud-xyz789"
}
```

### arguments_hash

- 对 `arguments` 字段做 JSON 规范化（key 排序、无多余空白）后计算 SHA-256
- 目的：审计日志可验证参数完整性，同时避免在日志中明文存储敏感参数
- 原始 arguments 仅在内存中存在，不落盘

### 不可篡改

- 审计文件以 append-only 模式打开
- 后续阶段可接入外部日志聚合（如 Loki / ELK）

## 传输安全

### 当前阶段（Phase 0–3）

- Gateway 仅监听 `127.0.0.1`，依赖操作系统级网络隔离
- Agent 与 Gateway 同机部署，通信走本地回环

### 后续目标（Phase 4+）

- **mTLS 双向认证**：Agent 与 Gateway 各自持有证书，握手时互相验证
- **自签 CA**：项目提供根 CA 生成脚本，用于签发 Agent / Gateway / 设备证书
- **证书轮换**：支持在线轮换，无需停机
- **传输加密**：TLS 1.3，禁用旧版协议与弱密码套件

## 安全原则总结

1. **默认拒绝**：未明确允许的操作一律禁止
2. **最小权限**：每个工具仅声明必需权限
3. **纵深防御**：manifest → 策略引擎 → 沙箱 → 审计四层校验
4. **不可绕过**：所有请求必须经过 Gateway，禁止直连 Agent
5. **可追溯**：每次操作有唯一 audit_id，可回溯完整调用链
6. **紧急制动**：一键停止所有写操作，只读不受影响
