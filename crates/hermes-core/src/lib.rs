//! hermes-core：Hermes-Android 的协议与基础设施内核。
//!
//! - [`protocol`]：统一调用协议（POST /v1/tools/execute）与标准化失败状态。
//! - [`schema`]：JSON Schema 最小子集校验（type / pattern / required / additionalProperties）。
//! - [`audit`]：JSONL 审计日志，含 arguments_hash。
//! - [`events`]：设备主动上报的事件模型。
//! - [`http_client`]：最小 HTTP/1.1 客户端，供 CLI 与集成测试调用 Gateway。

pub mod audit;
pub mod events;
pub mod http_client;
pub mod protocol;
pub mod schema;
