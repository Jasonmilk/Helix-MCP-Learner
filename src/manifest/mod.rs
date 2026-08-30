//! Tentacle 插件 Manifest 生成器
//!
//! 将 CI-144 工具定义转换为 Tentacle 可加载的插件 Manifest。
//! MCP 工具通过通用 MCP 代理执行体（mcp_proxy）调用。

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use crate::ci144::{Ci144Tool, RiskLevel};

/// Tentacle 安全等级映射
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SecurityLevel {
    Normal,
    Critical,
}

impl From<RiskLevel> for SecurityLevel {
    fn from(risk: RiskLevel) -> Self {
        match risk {
            RiskLevel::Low | RiskLevel::Medium => SecurityLevel::Normal,
            RiskLevel::Critical | RiskLevel::Catastrophic => SecurityLevel::Critical,
        }
    }
}

/// 完整性校验
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Integrity {
    pub algorithm: String,
    pub hash: String,
}

/// 权限声明
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Permission {
    #[serde(default)]
    pub network: Vec<String>,
    #[serde(default)]
    pub filesystem: Vec<String>,
    #[serde(default)]
    pub execute: bool,
}

/// 单个工具的 Manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
    /// 执行体：通用 MCP 代理
    pub executable: String,
    /// 完整性校验（MCP 代理的 SHA-256）
    pub integrity: Integrity,
    /// 参数 Schema
    pub parameters_schema: Value,
    /// 安全等级
    pub security_level: SecurityLevel,
    /// 权限
    #[serde(default)]
    pub permissions: Permission,
    /// CI-144 扩展元数据
    #[serde(default)]
    pub ci144: Ci144Metadata,
    /// 执行超时（毫秒）
    #[serde(default = "default_timeout")]
    pub timeout_ms: u32,
}

fn default_timeout() -> u32 {
    30000
}

/// CI-144 扩展元数据
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Ci144Metadata {
    /// 原始 MCP 工具名
    pub mcp_name: String,
    /// PFP Risk-Level
    pub pfp_risk_level: String,
    /// PFP Modality
    pub pfp_modality: String,
    /// 是否需要人工确认
    pub requires_confirmation: bool,
    /// MCP Server 配置
    pub mcp_server: McpServerConfig,
}

/// MCP Server 配置
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// 启动命令
    pub command: String,
    /// 启动参数
    pub args: Vec<String>,
    /// 传输方式
    pub transport: String,
}

/// 工具集合（一个 MCP Server 对应一个工具集合）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSet {
    /// MCP Server 名称
    pub server_name: String,
    /// 工具列表
    pub tools: Vec<ToolManifest>,
    /// 生成时间
    pub generated_at: String,
    /// 学习器版本
    pub learner_version: String,
}

/// Manifest 生成器
pub struct ManifestGenerator {
    /// MCP 代理执行体文件名
    proxy_executable: String,
    /// MCP 代理 SHA-256 哈希
    proxy_hash: String,
    /// MCP Server 配置
    server_config: McpServerConfig,
}

impl ManifestGenerator {
    /// 创建新的 Manifest 生成器
    pub fn new(
        server_name: &str,
        command: &str,
        args: Vec<String>,
    ) -> Self {
        Self {
            proxy_executable: "mcp_proxy.js".to_string(),
            // 占位哈希，实际使用时替换为真实代理的哈希
            proxy_hash: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            server_config: McpServerConfig {
                command: command.to_string(),
                args,
                transport: "stdio".to_string(),
            },
        }
    }

    /// 设置 MCP 代理执行体信息
    pub fn with_proxy(mut self, executable: &str, hash: &str) -> Self {
        self.proxy_executable = executable.to_string();
        self.proxy_hash = hash.to_string();
        self
    }

    /// 生成单个工具的 Manifest
    pub fn generate_tool(&self, tool: &Ci144Tool) -> ToolManifest {
        let security_level = SecurityLevel::from(tool.risk_level);

        // 根据风险等级设置权限
        let permissions = match tool.risk_level {
            RiskLevel::Low => Permission {
                filesystem: vec!["read".to_string()],
                ..Default::default()
            },
            RiskLevel::Medium => Permission {
                filesystem: vec!["read".to_string(), "write".to_string()],
                network: vec!["*".to_string()],
                ..Default::default()
            },
            RiskLevel::Critical | RiskLevel::Catastrophic => Permission {
                filesystem: vec!["*".to_string()],
                network: vec!["*".to_string()],
                execute: true,
            },
        };

        ToolManifest {
            name: tool.intent_name.clone(),
            version: "0.1.0".to_string(),
            description: tool.description.clone(),
            tags: vec![
                "mcp".to_string(),
                format!("risk:{}", tool.risk_level.as_str().to_lowercase()),
            ],
            executable: self.proxy_executable.clone(),
            integrity: Integrity {
                algorithm: "sha256".to_string(),
                hash: self.proxy_hash.clone(),
            },
            parameters_schema: tool.parameters.clone(),
            security_level,
            permissions,
            ci144: Ci144Metadata {
                mcp_name: tool.mcp_name.clone(),
                pfp_risk_level: tool.risk_level.as_str().to_string(),
                pfp_modality: tool.modality.clone(),
                requires_confirmation: tool.requires_confirmation,
                mcp_server: self.server_config.clone(),
            },
            timeout_ms: 30000,
        }
    }

    /// 生成工具集合
    pub fn generate_set(&self, server_name: &str, tools: &[Ci144Tool]) -> ToolSet {
        let tool_manifests: Vec<ToolManifest> = tools
            .iter()
            .map(|t| self.generate_tool(t))
            .collect();

        ToolSet {
            server_name: server_name.to_string(),
            tools: tool_manifests,
            generated_at: chrono_now(),
            learner_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// 将工具集合写入目录（每个工具一个 JSON 文件 + 一个索引文件）
    pub fn write_to_dir(
        &self,
        output_dir: &std::path::Path,
        server_name: &str,
        tools: &[Ci144Tool],
    ) -> Result<(), std::io::Error> {
        std::fs::create_dir_all(output_dir)?;

        let tool_set = self.generate_set(server_name, tools);

        // 写入索引文件
        let index_path = output_dir.join(format!("{}_index.json", server_name));
        let index_json = serde_json::to_string_pretty(&tool_set)?;
        std::fs::write(&index_path, index_json)?;

        // 写入每个工具的 Manifest
        for tool in &tool_set.tools {
            let tool_path = output_dir.join(format!("{}.json", tool.name));
            let tool_json = serde_json::to_string_pretty(tool)?;
            std::fs::write(&tool_path, tool_json)?;
        }

        Ok(())
    }
}

/// 获取当前时间（简化实现，避免 chrono 依赖）
fn chrono_now() -> String {
    // 使用 std::time 生成简单时间戳
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", now.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ci144::{Ci144Tool, RiskLevel};

    fn make_ci144_tool(name: &str, risk: RiskLevel) -> Ci144Tool {
        Ci144Tool {
            intent_name: name.to_string(),
            mcp_name: name.to_string(),
            description: format!("Test tool: {}", name),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"}
                },
                "required": ["path"]
            }),
            risk_level: risk,
            modality: "EXECUTIVE".to_string(),
            requires_confirmation: matches!(risk, RiskLevel::Critical | RiskLevel::Catastrophic),
        }
    }

    #[test]
    fn test_security_level_mapping() {
        assert_eq!(SecurityLevel::from(RiskLevel::Low), SecurityLevel::Normal);
        assert_eq!(SecurityLevel::from(RiskLevel::Medium), SecurityLevel::Normal);
        assert_eq!(SecurityLevel::from(RiskLevel::Critical), SecurityLevel::Critical);
        assert_eq!(SecurityLevel::from(RiskLevel::Catastrophic), SecurityLevel::Critical);
    }

    #[test]
    fn test_generate_tool() {
        let generator = ManifestGenerator::new("test-server", "npx", vec!["-y".to_string(), "mcp-server-test".to_string()]);
        let tool = make_ci144_tool("read_file", RiskLevel::Low);
        let manifest = generator.generate_tool(&tool);

        assert_eq!(manifest.name, "read_file");
        assert_eq!(manifest.security_level, SecurityLevel::Normal);
        assert_eq!(manifest.ci144.pfp_risk_level, "LOW");
        assert_eq!(manifest.ci144.mcp_server.command, "npx");
        assert!(!manifest.ci144.requires_confirmation);
    }

    #[test]
    fn test_generate_tool_critical() {
        let generator = ManifestGenerator::new("test-server", "npx", vec![]);
        let tool = make_ci144_tool("delete_file", RiskLevel::Critical);
        let manifest = generator.generate_tool(&tool);

        assert_eq!(manifest.security_level, SecurityLevel::Critical);
        assert_eq!(manifest.ci144.pfp_risk_level, "CRITICAL");
        assert!(manifest.ci144.requires_confirmation);
        assert!(manifest.permissions.execute);
    }

    #[test]
    fn test_generate_set() {
        let generator = ManifestGenerator::new("test-server", "npx", vec![]);
        let tools = vec![
            make_ci144_tool("read_file", RiskLevel::Low),
            make_ci144_tool("delete_file", RiskLevel::Critical),
        ];
        let set = generator.generate_set("test-server", &tools);

        assert_eq!(set.server_name, "test-server");
        assert_eq!(set.tools.len(), 2);
        assert_eq!(set.tools[0].name, "read_file");
        assert_eq!(set.tools[1].name, "delete_file");
    }

    #[test]
    fn test_write_to_dir() {
        let dir = tempfile::tempdir().unwrap();
        let generator = ManifestGenerator::new("test-server", "npx", vec![]);
        let tools = vec![make_ci144_tool("read_file", RiskLevel::Low)];

        generator.write_to_dir(dir.path(), "test-server", &tools).unwrap();

        // 验证索引文件存在
        let index_path = dir.path().join("test-server_index.json");
        assert!(index_path.exists());

        // 验证工具 Manifest 存在
        let tool_path = dir.path().join("read_file.json");
        assert!(tool_path.exists());

        // 验证内容可解析
        let content = std::fs::read_to_string(&tool_path).unwrap();
        let manifest: ToolManifest = serde_json::from_str(&content).unwrap();
        assert_eq!(manifest.name, "read_file");
    }
}
