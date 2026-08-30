//! Helix-MCP-Learner — MCP Server → CI-144 工具定义的自动学习器
//!
//! 核心哲学：去掉 MCP 的皮，提炼 CI-144 的骨。
//! MCP-Learner 不执行工具，只学习和提炼。

pub mod mcp;
pub mod ci144;
pub mod manifest;
pub mod proxy;
pub mod config;
pub mod glove;
pub mod learning;

pub use mcp::{McpClient, McpError, Tool, ToolInputSchema};
pub use ci144::{Ci144Tool, RiskLevel, risk_rating, extract_tools, extract_tool};
pub use manifest::ManifestGenerator;
pub use proxy::{McpProxy, McpServerConfig, ToolCallRequest, ToolCallResponse};
pub use config::{ConfigManager, LearnerConfig, BatchLearnResult, ConflictStrategy, example_config};
pub use learning::{IncrementalLearner, IncrementalLearnResult, ToolChange, ToolChangeType, LearningVersion};
