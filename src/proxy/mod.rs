//! MCP 代理执行体
//!
//! 管理 MCP Server 的生命周期，提供工具调用接口。
//! 让 Tentacle 可以直接执行学习后的工具，而不需要每次都启动 MCP Server。

use crate::mcp::{McpClient, Tool};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// MCP Server 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Server 名称（唯一标识）
    pub name: String,
    /// 启动命令
    pub command: String,
    /// 启动参数
    #[serde(default)]
    pub args: Vec<String>,
    /// 传输方式（当前仅支持 stdio）
    #[serde(default = "default_transport")]
    pub transport: String,
}

fn default_transport() -> String {
    "stdio".to_string()
}

/// 工具调用请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    /// MCP Server 名称
    pub server: String,
    /// 工具名称
    pub tool: String,
    /// 工具参数
    pub arguments: serde_json::Value,
}

/// 工具调用响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResponse {
    /// 是否成功
    pub success: bool,
    /// 结果内容
    pub result: Option<serde_json::Value>,
    /// 错误信息
    pub error: Option<String>,
    /// 耗时（毫秒）
    pub duration_ms: u64,
}

/// 单个 MCP Server 的运行时状态
struct ServerRuntime {
    config: McpServerConfig,
    client: Option<McpClient>,
    tools: Vec<Tool>,
    connected: bool,
}

impl ServerRuntime {
    fn new(config: McpServerConfig) -> Self {
        Self {
            config,
            client: None,
            tools: Vec::new(),
            connected: false,
        }
    }

    async fn connect(&mut self) -> Result<(), String> {
        if self.connected {
            return Ok(());
        }

        let mut client = McpClient::connect(&self.config.command, &self.config.args)
            .await
            .map_err(|e| format!("Failed to connect: {}", e))?;

        client.initialize().await.map_err(|e| format!("Failed to initialize: {}", e))?;

        let tools = client.list_tools().await.map_err(|e| format!("Failed to list tools: {}", e))?;

        self.client = Some(client);
        self.tools = tools;
        self.connected = true;

        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), String> {
        if let Some(client) = self.client.take() {
            client.close().await.map_err(|e| format!("Failed to close: {}", e))?;
        }
        self.connected = false;
        self.tools.clear();
        Ok(())
    }

    async fn call_tool(&mut self, tool_name: &str, arguments: serde_json::Value) -> Result<serde_json::Value, String> {
        if !self.connected {
            self.connect().await?;
        }

        let client = self.client.as_mut().ok_or("Not connected")?;
        client.call_tool(tool_name, arguments).await.map_err(|e| format!("Tool call failed: {}", e))
    }

    fn has_tool(&self, tool_name: &str) -> bool {
        self.tools.iter().any(|t| t.name == tool_name)
    }
}

/// MCP 代理执行体
///
/// 管理多个 MCP Server 的生命周期，提供统一的工具调用接口。
pub struct McpProxy {
    servers: Arc<Mutex<HashMap<String, ServerRuntime>>>,
}

impl McpProxy {
    /// 创建新的 MCP 代理
    pub fn new() -> Self {
        Self {
            servers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 从配置列表创建代理
    pub fn from_configs(configs: Vec<McpServerConfig>) -> Self {
        let proxy = Self::new();
        for config in configs {
            proxy.add_server(config);
        }
        proxy
    }

    /// 添加 MCP Server（不立即连接）
    pub fn add_server(&self, config: McpServerConfig) {
        let servers = self.servers.clone();
        // 注意：这里不能直接 await，因为 add_server 是同步方法
        // 我们只是将配置存入，连接会在第一次调用时懒加载
        tokio::spawn(async move {
            let mut servers = servers.lock().await;
            servers.insert(config.name.clone(), ServerRuntime::new(config));
        });
    }

    /// 移除 MCP Server
    pub async fn remove_server(&self, name: &str) -> Result<(), String> {
        let mut servers = self.servers.lock().await;
        if let Some(mut runtime) = servers.remove(name) {
            runtime.disconnect().await?;
        }
        Ok(())
    }

    /// 连接到指定 MCP Server
    pub async fn connect_server(&self, name: &str) -> Result<(), String> {
        let mut servers = self.servers.lock().await;
        let runtime = servers.get_mut(name).ok_or(format!("Server '{}' not found", name))?;
        runtime.connect().await
    }

    /// 断开指定 MCP Server
    pub async fn disconnect_server(&self, name: &str) -> Result<(), String> {
        let mut servers = self.servers.lock().await;
        let runtime = servers.get_mut(name).ok_or(format!("Server '{}' not found", name))?;
        runtime.disconnect().await
    }

    /// 调用工具
    pub async fn call_tool(&self, request: ToolCallRequest) -> ToolCallResponse {
        let start = std::time::Instant::now();

        let result = {
            let mut servers = self.servers.lock().await;
            let runtime = match servers.get_mut(&request.server) {
                Some(r) => r,
                None => {
                    return ToolCallResponse {
                        success: false,
                        result: None,
                        error: Some(format!("Server '{}' not found", request.server)),
                        duration_ms: start.elapsed().as_millis() as u64,
                    };
                }
            };

            // 检查工具是否存在
            if !runtime.has_tool(&request.tool) {
                // 尝试重新连接以获取最新工具列表
                if let Err(e) = runtime.connect().await {
                    return ToolCallResponse {
                        success: false,
                        result: None,
                        error: Some(format!("Failed to refresh tools: {}", e)),
                        duration_ms: start.elapsed().as_millis() as u64,
                    };
                }
                if !runtime.has_tool(&request.tool) {
                    return ToolCallResponse {
                        success: false,
                        result: None,
                        error: Some(format!("Tool '{}' not found on server '{}'", request.tool, request.server)),
                        duration_ms: start.elapsed().as_millis() as u64,
                    };
                }
            }

            runtime.call_tool(&request.tool, request.arguments).await
        };

        match result {
            Ok(value) => ToolCallResponse {
                success: true,
                result: Some(value),
                error: None,
                duration_ms: start.elapsed().as_millis() as u64,
            },
            Err(e) => ToolCallResponse {
                success: false,
                result: None,
                error: Some(e),
                duration_ms: start.elapsed().as_millis() as u64,
            },
        }
    }

    /// 获取所有已注册的 Server 名称
    pub async fn list_servers(&self) -> Vec<String> {
        let servers = self.servers.lock().await;
        servers.keys().cloned().collect()
    }

    /// 获取指定 Server 的工具列表
    pub async fn list_tools(&self, server_name: &str) -> Result<Vec<Tool>, String> {
        let mut servers = self.servers.lock().await;
        let runtime = servers.get_mut(server_name).ok_or(format!("Server '{}' not found", server_name))?;

        if !runtime.connected {
            runtime.connect().await?;
        }

        Ok(runtime.tools.clone())
    }

    /// 关闭所有 Server 连接
    pub async fn shutdown(&self) -> Result<(), String> {
        let mut servers = self.servers.lock().await;
        for (_, mut runtime) in servers.drain() {
            let _ = runtime.disconnect().await;
        }
        Ok(())
    }
}

impl Default for McpProxy {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_serialization() {
        let config = McpServerConfig {
            name: "test-server".to_string(),
            command: "python3".to_string(),
            args: vec!["mock.py".to_string()],
            transport: "stdio".to_string(),
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: McpServerConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, "test-server");
        assert_eq!(deserialized.command, "python3");
        assert_eq!(deserialized.args, vec!["mock.py"]);
        assert_eq!(deserialized.transport, "stdio");
    }

    #[test]
    fn test_tool_call_request_serialization() {
        let request = ToolCallRequest {
            server: "test".to_string(),
            tool: "read_file".to_string(),
            arguments: serde_json::json!({"path": "/tmp/test.txt"}),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"server\":\"test\""));
        assert!(json.contains("\"tool\":\"read_file\""));
    }

    #[test]
    fn test_tool_call_response() {
        let response = ToolCallResponse {
            success: true,
            result: Some(serde_json::json!("Hello")),
            error: None,
            duration_ms: 100,
        };

        assert!(response.success);
        assert_eq!(response.duration_ms, 100);
    }

    #[tokio::test]
    async fn test_proxy_lifecycle() {
        let proxy = McpProxy::new();

        // 初始状态：没有 server
        let servers = proxy.list_servers().await;
        assert!(servers.is_empty());

        // 添加 server
        let config = McpServerConfig {
            name: "test".to_string(),
            command: "echo".to_string(),
            args: vec![],
            transport: "stdio".to_string(),
        };
        proxy.add_server(config);

        // 等待异步添加完成
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let servers = proxy.list_servers().await;
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0], "test");

        // 移除 server
        proxy.remove_server("test").await.unwrap();
        let servers = proxy.list_servers().await;
        assert!(servers.is_empty());
    }
}
