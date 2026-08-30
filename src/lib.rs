//! Helix-MCP-Learner — MCP Server → CI-144 工具定义的自动学习器
//!
//! 核心哲学：去掉 MCP 的皮，提炼 CI-144 的骨。
//! MCP-Learner 不执行工具，只学习和提炼。

pub mod mcp;
pub mod ci144;
pub mod manifest;
pub mod proxy;
pub mod config;
/// ⚠️ DEPRECATED — 已迁移至 HelixECO-Glove (https://github.com/Jasonmilk/HelixECO-Glove)
///
/// 本模块将在 v0.3.0 移除。请使用 helix-eco-glove-macos + helix-eco-glove-tentacle-adapter。
pub mod glove;
pub mod learning;

pub use mcp::{McpClient, McpError, Tool, ToolInputSchema};
pub use ci144::{Ci144Tool, RiskLevel, risk_rating, extract_tools, extract_tool};
pub use manifest::ManifestGenerator;
pub use proxy::{McpProxy, McpServerConfig, ToolCallRequest, ToolCallResponse};
pub use config::{ConfigManager, LearnerConfig, BatchLearnResult, ConflictStrategy, example_config};
pub use learning::{IncrementalLearner, IncrementalLearnResult, ToolChange, ToolChangeType, LearningVersion};
