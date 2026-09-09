# Hermes-Android 架构说明

## 三层架构概览

Hermes-Android 采用三层分离架构，确保 AI 代理对 Android 设备的控制安全、可控、可审计。

```
┌─────────────────┐       ┌──────────────────────┐       ┌──────────────────┐
│  Hermes Agent   │──────▶│  Android Control     │──────▶│  Android Agent   │
│  (决策层)        │       │  Gateway (鉴权/策略)  │       │  (执行层)         │
└─────────────────┘       └──────────────────────┘       └──────────────────┘
      ▲                           ▲                            ▲
      │                           │                            │
  LLM 推理                   策略引擎 + 审批               设备 API 调用
  工具选择                   审计日志                      沙箱隔离
  意图理解                   速率限制                      结果回传
```

### 第一层：Hermes Agent（决策层）

- **职责**：接收用户自然语言指令，通过 LLM 推理选择合适的工具并构造参数
- **原则**：不直接操作设备；所有操作必须经过 Gateway
- **crate**：`hermes-core`（核心推理与工具调度逻辑）、`hermes-cli`（命令行交互界面）

### 第二层：Android Control Gateway（鉴权 / 策略 / 审批 / 审计）

Gateway 是整个系统的安全中枢，负责在 Agent 请求到达设备之前完成全部安全检查。

#### Gateway 模块清单

| 模块 | 职责 |
|------|------|
| Authenticator | 验证请求来源身份与会话有效性 |
| Tool Registry | 注册可用工具及其 manifest，提供工具发现 |
| Policy Engine | 根据 `policies/default.yaml` 评估请求是否允许 |
| Approval Manager | 对需要人工审批的请求发起审批流程 |
| Rate Limiter | 限制单位时间内的请求频率，防止滥用 |
| Idempotency Store | 基于幂等键去重，防止重复执行 |
| Executor Router | 将合法请求路由到对应的 Android Agent 实例 |
| Audit Logger | 将所有请求与结果写入 JSONL 审计日志 |
| Result Normalizer | 统一不同 Agent 返回的结果格式 |
| Emergency Stop | 全局紧急停止开关，一键阻断所有写操作 |

- **crate**：`gateway`
- **网络绑定**：仅监听 `127.0.0.1`，不接受外部网络连接

### 第三层：Android Agent（执行层）

- **职责**：在真实设备或模拟器上执行具体操作并返回结构化结果
- **原则**：只接受来自 Gateway 的合法请求；自身不做任何鉴权判断
- **crate**：`android-agent-fake`（当前阶段使用的模拟器实现）

## Crate 与模块对应关系

| Crate | 对应架构层 | 说明 |
|-------|-----------|------|
| `hermes-core` | 决策层 | LLM 集成、工具调度、会话管理 |
| `hermes-cli` | 决策层 | CLI 子命令：tools / call / approve / stop |
| `gateway` | 中间层 | 上述十个模块的实现 |
| `android-agent-fake` | 执行层 | Phase 0 模拟器，模拟设备响应 |

## 网络约束

- Gateway HTTP 服务 **仅绑定 `127.0.0.1`**，禁止暴露到局域网或公网
- Agent → Gateway 通信走本地回环
- mTLS / 证书认证为后续阶段目标（见 `roadmap.md` Phase 4）
