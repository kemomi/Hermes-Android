//! Android Control Gateway：鉴权、策略、审批、限频、幂等、审计与执行路由的中枢。
//!
//! 模块对应蓝图 Gateway 十组件：
//! - [`registry`] Tool Registry / 部分 Authenticator（工具发现）
//! - [`policy`] Policy Engine
//! - [`approval`] Approval Manager
//! - [`rate_limit`] Rate Limiter
//! - [`idempotency`] Idempotency Store
//! - [`router`] Executor Router
//! - [`emergency_stop`] Emergency Stop
//! - [`handler`] 编排上述组件的核心处理器，并调用 hermes-core 的 Audit Logger 与 Result Normalizer
//! - [`http_server`] 对外 HTTP/1.1 接口（仅回环监听）

pub mod approval;
pub mod emergency_stop;
pub mod handler;
pub mod http_server;
pub mod idempotency;
pub mod policy;
pub mod rate_limit;
pub mod registry;
pub mod router;

pub use handler::{ApprovalTicket, Gateway};
