# Hermes-Android 通信协议

## 工具执行接口

### POST /v1/tools/execute

#### 请求示例

```json
{
  "request_id": "req-a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "session_id": "sess-001",
  "device_id": "emulator-5554",
  "tool": "android.device_info",
  "arguments": {},
  "requested_by": {
    "type": "agent",
    "user_id": "hermes-agent-01"
  },
  "reason": "用户询问设备型号"
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| request_id | string | ✅ | 全局唯一请求标识（UUID） |
| session_id | string | ✅ | 会话标识，用于关联同一轮对话 |
| device_id | string | ✅ | 目标设备标识 |
| tool | string | ✅ | 工具名称，须在 Tool Registry 中注册 |
| arguments | object | ✅ | 工具参数，须符合 input_schema |
| requested_by.type | string | ✅ | 请求来源类型：`agent` / `human` |
| requested_by.user_id | string | ✅ | 请求者标识 |
| reason | string | ❌ | 人类可读的调用原因 |

#### 响应示例

```json
{
  "request_id": "req-a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "status": "success",
  "tool": "android.device_info",
  "result": {
    "ok": true,
    "model": "Pixel 7",
    "manufacturer": "Google",
    "android_version": "14",
    "api_level": 34
  },
  "audit_id": "aud-xyz789",
  "duration_ms": 42
}
```

| 字段 | 类型 | 说明 |
|------|------|------|
| request_id | string | 与请求对应 |
| status | string | `success` / `error` / `pending_approval` |
| tool | string | 工具名称 |
| result | object | 工具返回的结构化结果 |
| audit_id | string | 审计日志条目 ID |
| duration_ms | integer | 执行耗时（毫秒） |

## 失败状态码表

| 状态码 | 含义 |
|--------|------|
| INVALID_ARGUMENT | 请求参数不符合工具的 input_schema 或缺少必填字段 |
| UNAUTHORIZED | 请求者身份验证失败或会话已过期 |
| FORBIDDEN | 策略引擎拒绝该请求（如访问黑名单路径） |
| APPROVAL_REQUIRED | 该操作需要人工审批，等待审批后重试 |
| DEVICE_OFFLINE | 目标设备不可达或已断开连接 |
| TIMEOUT | 工具执行超过 max_execution_time_ms 限制 |
| EXECUTION_FAILED | 工具在执行过程中发生内部错误 |
| POLICY_BLOCKED | 请求被安全策略明确阻止（如通用 shell 命令） |
| DUPLICATE_REQUEST | 相同幂等键的请求已在处理或已完成 |
| ROLLBACK_REQUIRED | 操作部分失败，需要回滚到一致状态 |

## 事件协议

Gateway 支持通过 SSE（Server-Sent Events）推送实时事件。

### 事件 JSON 示例

```json
{
  "event_type": "tool.executed",
  "timestamp": "2026-09-07T10:30:00+08:00",
  "request_id": "req-a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "device_id": "emulator-5554",
  "payload": {
    "tool": "android.battery_status",
    "status": "success",
    "duration_ms": 15
  }
}
```

### 推荐事件类型

| 事件类型 | 触发时机 |
|----------|---------|
| tool.executed | 工具执行完成（成功或失败） |
| approval.requested | 需要人工审批的操作已提交 |
| approval.resolved | 审批请求已被批准或拒绝 |
| device.connected | 新设备上线 |
| device.disconnected | 设备离线 |
| emergency_stop.activated | 紧急停止开关被触发 |
| policy.updated | 策略文件热加载完成 |
| rate_limit.triggered | 速率限制被触发 |
