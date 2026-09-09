# Hermes-Android

AI 代理通过 Gateway 安全控制 Android 设备的三层系统。

## 架构概览

```
┌──────────────┐      ┌─────────────────────┐      ┌────────────────┐
│ Hermes Agent │─────▶│ Android Control     │─────▶│ Android Agent  │
│ (LLM 决策)   │      │ Gateway             │      │ (设备执行)      │
│              │      │ (鉴权/策略/审批/审计) │      │                │
└──────────────┘      └─────────────────────┘      └────────────────┘
```

- **Hermes Agent**：接收自然语言指令，通过 LLM 选择工具并构造参数
- **Gateway**：安全中枢——鉴权、策略评估、审批、速率限制、审计、紧急停止
- **Android Agent**：在设备上执行具体操作（当前为模拟器实现）

详见 [docs/architecture.md](docs/architecture.md)。

## 快速开始

### 构建

```bash
cargo build --workspace
```

### 启动模拟器 Agent

```bash
cargo run -p android-agent-fake
```

### 启动 Gateway

```bash
cargo run -p gateway
```

### CLI 使用示例

```bash
# 列出所有可用工具（默认连 127.0.0.1:8787，可用 --gateway 覆盖）
hermes-cli tools

# 调用只读工具
hermes-cli call android.device_info

# 调用带参数的工具，--args 传 JSON
hermes-cli call android.app.launch --args '{"package_name":"com.android.settings"}'

# 高风险工具：先签发一次性审批令牌，再带令牌调用
hermes-cli approve --request-id req_42 --tool android.file.delete
hermes-cli call android.file.delete --args '{"path":"/sdcard/Download/x"}' \
    --request-id req_42 --approval-token <上一步返回的 token>

# 紧急停止：拦截所有非只读操作；--release 解除
hermes-cli stop
hermes-cli stop --release
```

## 仓库结构

```
.
├── crates/
│   ├── hermes-core/          # 统一协议、Schema 校验、审计、事件与 HTTP 客户端
│   ├── gateway/              # 安全网关（鉴权/策略/审批/审计）
│   ├── android-agent-fake/   # Phase 0 模拟器 Agent
│   └── hermes-cli/           # 命令行工具
├── docs/
│   ├── architecture.md       # 三层架构说明
│   ├── protocol.md           # 通信协议规范
│   ├── roadmap.md            # 分阶段路线图
│   └── security.md           # 安全设计
├── policies/
│   └── default.yaml          # 默认安全策略
├── skills/                   # 工具技能定义
│   ├── android.device_info/
│   ├── android.battery_status/
│   ├── android.network_status/
│   ├── android.storage_status/
│   ├── android.process_list/
│   ├── android.thermal_status/
│   ├── android.app.list/
│   ├── android.app.launch/
│   ├── android.screen.capture/
│   ├── android.ui.tree/
│   └── android.file.list/
├── .github/workflows/ci.yml  # CI 配置
├── LICENSE                   # MIT 许可证
└── README.md                 # 本文件
```

## 验证

```bash
cargo test --workspace
```

本地若无 Rust 链接器或交叉编译环境，可依赖 GitHub Actions CI 完成验证。

## 许可证

本项目基于 MIT 许可证发布，详见 [LICENSE](LICENSE)。

## Legacy

`legacy/` 目录包含早期原型代码，仅供参考，不参与当前构建。
