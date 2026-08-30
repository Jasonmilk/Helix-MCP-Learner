//! Helix-MCP-Learner — MCP Server → CI-144 工具定义的自动学习器
//!
//! 核心哲学：去掉 MCP 的皮，提炼 CI-144 的骨。
//! MCP-Learner 不执行工具，只学习和提炼。

pub mod mcp;
pub mod ci144;
pub mod manifest;
pub mod proxy;
pub mod config;
/// Post-Learn 审查管道 — L1 静态审查 + 状态迁移自动化（raw/staging/stable/rejected）
pub mod post_learn;
/// ⚠️ DEPRECATED — 已迁移至 HelixECO-Glove (https://github.com/Jasonmilk/HelixECO-Glove)
///
/// 本模块将在 v0.3.0 移除。请使用 helix-eco-glove-macos + helix-eco-glove-tentacle-adapter。
///
/// 使用 `--no-default-features` 可完全排除本模块。
#[cfg(feature = "glove")]
#[deprecated(
    since = "0.2.0",
    note = "OS Glove 模块已迁移至 HelixECO-Glove (https://github.com/Jasonmilk/HelixECO-Glove)，将在 v0.3.0 移除。使用 --no-default-features 可完全排除。"
)]
pub mod glove;
pub mod learning;

pub use mcp::{McpClient, McpError, Tool, ToolInputSchema};
pub use ci144::{Ci144Tool, RiskLevel, risk_rating, extract_tools, extract_tool, extract_tools_with_namespace, extract_tool_with_namespace};
pub use manifest::ManifestGenerator;
pub use proxy::{McpProxy, McpServerConfig, ToolCallRequest, ToolCallResponse};
pub use config::{ConfigManager, LearnerConfig, BatchLearnResult, ConflictStrategy, example_config};
pub use learning::{IncrementalLearner, IncrementalLearnResult, ToolChange, ToolChangeType, LearningVersion};
