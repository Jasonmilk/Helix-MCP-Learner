//! 多 MCP Server 配置管理
//!
//! 支持 TOML 配置文件，管理多个 MCP Server，提供批量学习功能。

use crate::ci144::Ci144Tool;
use crate::manifest::ManifestGenerator;
use crate::mcp::{McpClient, Tool};
use crate::proxy::McpServerConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// 配置文件错误
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("TOML serialize error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),
    #[error("MCP error: {0}")]
    Mcp(String),
    #[error("Duplicate server name: {0}")]
    DuplicateServer(String),
    #[error("Duplicate tool name: {0} (from servers: {1}, {2})")]
    DuplicateTool(String, String, String),
}

/// MCP-Learner 配置文件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnerConfig {
    /// 全局配置
    #[serde(default)]
    pub global: GlobalConfig,
    /// MCP Server 列表
    #[serde(default)]
    pub servers: Vec<McpServerConfig>,
    /// 输出配置
    #[serde(default)]
    pub output: OutputConfig,
}

/// 全局配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// 学习结果输出目录
    #[serde(default = "default_output_dir")]
    pub output_dir: String,
    /// 是否在学习后自动启动代理
    #[serde(default)]
    pub auto_start_proxy: bool,
    /// 工具名冲突处理策略
    #[serde(default = "default_conflict_strategy")]
    pub conflict_strategy: ConflictStrategy,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            output_dir: default_output_dir(),
            auto_start_proxy: false,
            conflict_strategy: default_conflict_strategy(),
        }
    }
}

fn default_output_dir() -> String {
    "./plugins".to_string()
}

fn default_conflict_strategy() -> ConflictStrategy {
    ConflictStrategy::Error
}

/// 工具名冲突处理策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConflictStrategy {
    /// 报错并停止
    Error,
    /// 跳过冲突的工具
    Skip,
    /// 重命名（server_tool）
    Rename,
    /// 覆盖（后加载的覆盖先加载的）
    Overwrite,
}

/// 输出配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    /// 是否生成索引文件
    #[serde(default = "default_true")]
    pub generate_index: bool,
    /// 是否按 server 分子目录
    #[serde(default = "default_true")]
    pub per_server_dir: bool,
    /// 输出格式（json/toml）
    #[serde(default = "default_format")]
    pub format: String,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            generate_index: true,
            per_server_dir: true,
            format: default_format(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_format() -> String {
    "json".to_string()
}

/// 批量学习结果
#[derive(Debug, Clone)]
pub struct BatchLearnResult {
    /// 成功学习的 server 数量
    pub servers_learned: usize,
    /// 失败的 server 列表
    pub failed_servers: Vec<(String, String)>,
    /// 学习到的工具总数（去重后）
    pub total_tools: usize,
    /// 按 server 分组的工具
    pub tools_by_server: HashMap<String, Vec<Ci144Tool>>,
    /// 冲突的工具列表
    pub conflicts: Vec<(String, String, String)>,
}

/// 配置管理器
pub struct ConfigManager {
    config: LearnerConfig,
}

impl ConfigManager {
    /// 从 TOML 文件加载配置
    pub fn from_file(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        let config: LearnerConfig = toml::from_str(&content)?;
        Ok(Self { config })
    }

    /// 从 TOML 字符串加载配置
    pub fn from_toml(content: &str) -> Result<Self, ConfigError> {
        let config: LearnerConfig = toml::from_str(content)?;
        Ok(Self { config })
    }

    /// 创建默认配置
    pub fn default() -> Self {
        Self {
            config: LearnerConfig {
                global: GlobalConfig::default(),
                servers: Vec::new(),
                output: OutputConfig::default(),
            },
        }
    }

    /// 获取配置引用
    pub fn config(&self) -> &LearnerConfig {
        &self.config
    }

    /// 获取 server 列表
    pub fn servers(&self) -> &[McpServerConfig] {
        &self.config.servers
    }

    /// 添加 server
    pub fn add_server(&mut self, server: McpServerConfig) -> Result<(), ConfigError> {
        if self.config.servers.iter().any(|s| s.name == server.name) {
            return Err(ConfigError::DuplicateServer(server.name));
        }
        self.config.servers.push(server);
        Ok(())
    }

    /// 保存配置到 TOML 文件
    pub fn save_to_file(&self, path: &Path) -> Result<(), ConfigError> {
        let content = toml::to_string_pretty(&self.config)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// 批量学习所有 server
    pub async fn batch_learn(&self) -> Result<BatchLearnResult, ConfigError> {
        let mut result = BatchLearnResult {
            servers_learned: 0,
            failed_servers: Vec::new(),
            total_tools: 0,
            tools_by_server: HashMap::new(),
            conflicts: Vec::new(),
        };

        let mut all_tools: HashMap<String, (String, Ci144Tool)> = HashMap::new();

        for server_config in &self.config.servers {
            match learn_server(server_config).await {
                Ok(tools) => {
                    result.servers_learned += 1;

                    for tool in &tools {
                        if let Some((existing_server, _)) = all_tools.get(&tool.intent_name) {
                            // 工具名冲突
                            match self.config.global.conflict_strategy {
                                ConflictStrategy::Error => {
                                    return Err(ConfigError::DuplicateTool(
                                        tool.intent_name.clone(),
                                        existing_server.clone(),
                                        server_config.name.clone(),
                                    ));
                                }
                                ConflictStrategy::Skip => {
                                    result.conflicts.push((
                                        tool.intent_name.clone(),
                                        existing_server.clone(),
                                        server_config.name.clone(),
                                    ));
                                    continue;
                                }
                                ConflictStrategy::Rename => {
                                    let new_name = format!("{}_{}", server_config.name, tool.intent_name);
                                    let mut renamed_tool = tool.clone();
                                    renamed_tool.intent_name = new_name.clone();
                                    all_tools.insert(new_name, (server_config.name.clone(), renamed_tool));
                                }
                                ConflictStrategy::Overwrite => {
                                    all_tools.insert(tool.intent_name.clone(), (server_config.name.clone(), tool.clone()));
                                }
                            }
                        } else {
                            all_tools.insert(tool.intent_name.clone(), (server_config.name.clone(), tool.clone()));
                        }
                    }

                    result.tools_by_server.insert(server_config.name.clone(), tools);
                }
                Err(e) => {
                    result.failed_servers.push((server_config.name.clone(), e));
                }
            }
        }

        result.total_tools = all_tools.len();
        Ok(result)
    }

    /// 生成所有 Manifest 到输出目录
    pub fn generate_all_manifests(&self, result: &BatchLearnResult) -> Result<(), ConfigError> {
        let output_dir = Path::new(&self.config.global.output_dir);

        for (server_name, tools) in &result.tools_by_server {
            let server_output_dir = if self.config.output.per_server_dir {
                output_dir.join(server_name)
            } else {
                output_dir.to_path_buf()
            };

            let generator = ManifestGenerator::new(
                server_name,
                &find_server_command(&self.config.servers, server_name),
                find_server_args(&self.config.servers, server_name),
            );

            generator
                .write_to_dir(&server_output_dir, server_name, tools)
                .map_err(|e| ConfigError::Mcp(format!("Failed to write manifests: {}", e)))?;
        }

        Ok(())
    }
}

/// 学习单个 server
async fn learn_server(config: &McpServerConfig) -> Result<Vec<Ci144Tool>, String> {
    let mut client = McpClient::connect(&config.command, &config.args)
        .await
        .map_err(|e| format!("Connect failed: {}", e))?;

    client.initialize().await.map_err(|e| format!("Initialize failed: {}", e))?;

    let tools = client.list_tools().await.map_err(|e| format!("List tools failed: {}", e))?;

    client.close().await.map_err(|e| format!("Close failed: {}", e))?;

    Ok(crate::ci144::extract_tools(&tools))
}

/// 查找 server 的 command
fn find_server_command(servers: &[McpServerConfig], name: &str) -> String {
    servers
        .iter()
        .find(|s| s.name == name)
        .map(|s| s.command.clone())
        .unwrap_or_default()
}

/// 查找 server 的 args
fn find_server_args(servers: &[McpServerConfig], name: &str) -> Vec<String> {
    servers
        .iter()
        .find(|s| s.name == name)
        .map(|s| s.args.clone())
        .unwrap_or_default()
}

/// 生成示例配置文件内容
pub fn example_config() -> String {
    r#"# Helix-MCP-Learner 配置文件示例

[global]
output_dir = "./plugins"
auto_start_proxy = false
conflict_strategy = "error"  # error | skip | rename | overwrite

[output]
generate_index = true
per_server_dir = true
format = "json"

[[servers]]
name = "filesystem"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
transport = "stdio"

[[servers]]
name = "github"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-github"]
transport = "stdio"
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_parse() {
        let config_str = r#"
[global]
output_dir = "./output"
auto_start_proxy = true
conflict_strategy = "rename"

[[servers]]
name = "test"
command = "python3"
args = ["mock.py"]
transport = "stdio"
"#;

        let config: LearnerConfig = toml::from_str(config_str).unwrap();
        assert_eq!(config.global.output_dir, "./output");
        assert!(config.global.auto_start_proxy);
        assert_eq!(config.global.conflict_strategy, ConflictStrategy::Rename);
        assert_eq!(config.servers.len(), 1);
        assert_eq!(config.servers[0].name, "test");
    }

    #[test]
    fn test_config_manager_add_server() {
        let mut manager = ConfigManager::default();
        let server = McpServerConfig {
            name: "test".to_string(),
            command: "python3".to_string(),
            args: vec![],
            transport: "stdio".to_string(),
        };

        manager.add_server(server.clone()).unwrap();
        assert_eq!(manager.servers().len(), 1);

        // 重复添加应该报错
        let result = manager.add_server(server);
        assert!(matches!(result, Err(ConfigError::DuplicateServer(_))));
    }

    #[test]
    fn test_example_config() {
        let example = example_config();
        let config: LearnerConfig = toml::from_str(&example).unwrap();
        assert_eq!(config.servers.len(), 2);
        assert_eq!(config.servers[0].name, "filesystem");
        assert_eq!(config.servers[1].name, "github");
    }

    #[test]
    fn test_conflict_strategy_deserialize() {
        let strategies = [
            ("error", ConflictStrategy::Error),
            ("skip", ConflictStrategy::Skip),
            ("rename", ConflictStrategy::Rename),
            ("overwrite", ConflictStrategy::Overwrite),
        ];
        for (s, expected) in strategies {
            let config_str = format!(
                r#"
[global]
conflict_strategy = "{}"
"#,
                s
            );
            let config: LearnerConfig = toml::from_str(&config_str).unwrap();
            assert_eq!(config.global.conflict_strategy, expected);
        }
    }
}
