# Hermes-Android 路线图

> ⚠️ **安全红线**：在任何阶段均**禁止**实现 `shell.exec`、`adb.shell`、`root.exec` 等通用命令执行器。所有设备操作必须通过具名工具完成，每个工具拥有独立的 manifest、schema 与策略规则。

## Phase 0：模拟器基础（当前）

**目标**：搭建三层架构骨架，使用假设备验证端到端流程。

| 工具 | 说明 |
|------|------|
| android-agent-fake | 模拟设备响应，不连接真实硬件 |

**验收要点**：
- [ ] Gateway 启动并监听 127.0.0.1
- [ ] hermes-cli 可通过 tools / call 子命令与 Gateway 交互
- [ ] 策略引擎正确加载 policies/default.yaml
- [ ] 审计日志写入 JSONL 文件
- [ ] cargo test --workspace 全部通过

## Phase 1：只读设备能力

**目标**：接入真实设备（或高级模拟器），实现全部只读查询工具。

| 工具 | 风险 |
|------|------|
| android.device_info | low |
| android.battery_status | low |
| android.network_status | low |
| android.storage_status | low |
| android.process_list | low |
| android.thermal_status | low |
| android.app.list | low |
| android.screen.capture | medium |
| android.ui.tree | medium |
| android.file.list | medium |

**验收要点**：
- [ ] 所有只读工具在真机/模拟器上返回正确数据
- [ ] 沙箱路径白名单生效，黑名单路径被拒绝
- [ ] 幂等键去重正常工作
- [ ] 速率限制可配置并生效

## Phase 2：普通用户态控制

**目标**：支持非破坏性的用户态写操作。

| 工具 | 说明 |
|------|------|
| android.app.launch | 启动应用 |
| android.app.stop | 停止应用（需审批） |
| android.file.write | 在白名单路径下写入文件（需审批） |
| android.settings.display | 调整亮度等显示设置（需审批） |
| android.notification.send | 发送本地通知 |

**验收要点**：
- [ ] 写操作默认 require_approval
- [ ] 审批流程可通过 hermes-cli approve 完成
- [ ] 紧急停止开关可一键阻断所有写操作
- [ ] 审计日志包含 arguments_hash

## Phase 3：Root 用户态

**目标**：在已 root 设备上支持受限的 root 级操作。

| 工具 | 说明 |
|------|------|
| android.root.package_manage | 安装/卸载 APK（需审批 + 双重确认） |
| android.root.service_control | 启停系统服务（需审批） |
| android.root.file_access | 访问受保护路径（需审批 + 路径白名单） |

**验收要点**：
- [ ] 所有 root 工具默认 require_approval
- [ ] 未 root 设备上 root 工具返回 FORBIDDEN
- [ ] 操作超时自动回滚
- [ ] 审计日志记录完整调用链

## Phase 4：审批与审计强化

**目标**：完善安全基础设施。

| 能力 | 说明 |
|------|------|
| mTLS 双向认证 | Agent ↔ Gateway 通信加密与身份互验 |
| 证书管理 | 自签 CA + 设备证书签发流程 |
| 审计仪表盘 | JSONL 日志可视化查询 |
| 审批 Web UI | 浏览器端审批界面（可选） |
| 策略热加载 | 修改 YAML 后无需重启 |

**验收要点**：
- [ ] mTLS 握手成功，无证书请求被拒绝
- [ ] 审计日志可追溯任意 request_id 的完整生命周期
- [ ] 策略变更后 5 秒内生效

## Phase 5：内核只读观测

**目标**：在不修改内核的前提下读取内核暴露的只读信息。

| 工具 | 说明 |
|------|------|
| android.kernel.dmesg | 读取内核日志缓冲区（只读） |
| android.kernel.cpuinfo | 读取 /proc/cpuinfo（只读） |
| android.kernel.meminfo | 读取 /proc/meminfo（只读） |

**验收要点**：
- [ ] 仅读取预定义的 /proc、/sys 节点
- [ ] 任何写尝试被 Policy Engine 拦截
- [ ] 输出经过脱敏处理

## Phase 6：有限内核写能力

**目标**：在严格审批下允许极少数内核参数调整。

| 工具 | 说明 |
|------|------|
| android.kernel.write_param | 写入白名单内的 sysfs 节点（需审批 + 双重确认） |

**验收要点**：
- [ ] 写入目标必须在编译期白名单中
- [ ] 每次写入前自动备份原值
- [ ] 超时或失败时自动恢复原值
- [ ] 全量审计，含 diff 记录

---

## 跨阶段约束

1. **禁止通用 Shell**：`shell.exec`、`adb.shell`、`root.exec` 在任何阶段均不得实现
2. **最小权限**：每个工具仅声明所需的最小权限集
3. **默认拒绝**：未注册的工具一律拒绝（allow_unknown_tools: false）
4. **审计不可关闭**：require_audit 始终为 true
5. **网络隔离**：Gateway 仅监听 127.0.0.1，直到 Phase 4 引入 mTLS
