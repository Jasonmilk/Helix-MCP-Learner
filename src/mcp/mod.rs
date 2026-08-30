//! MCP Client 基础层
//!
//! 负责与 MCP Server 通信（stdio 传输 + JSON-RPC 2.0 协议）。
//! 支持 tools/list 和 tools/call 两个核心方法。

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::process::{Command, Stdio};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command as TokioCommand;

/// MCP 错误类型
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Protocol error: {0}")]
    Protocol(String),
    #[error("Server error: {0}")]
    Server(String),
    #[error("Timeout")]
    Timeout,
}

/// JSON-RPC 2.0 请求
#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
}

/// JSON-RPC 2.0 响应
#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    id: u64,
    #[serde(default)]
    result: Option<Value>,
    #[serde(default)]
    error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 错误
#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
    #[serde(default)]
    data: Option<Value>,
}

/// MCP 工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub inputSchema: ToolInputSchema,
}

/// 工具输入 Schema（JSON Schema 子集）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInputSchema {
    #[serde(rename = "type", default = "default_type")]
    pub schema_type: String,
    #[serde(default)]
    pub properties: std::collections::HashMap<String, Value>,
    #[serde(default)]
    pub required: Vec<String>,
}

fn default_type() -> String {
    "object".to_string()
}

impl Default for ToolInputSchema {
    fn default() -> Self {
        Self {
            schema_type: "object".to_string(),
            properties: std::collections::HashMap::new(),
            required: Vec::new(),
        }
    }
}

/// tools/list 响应
#[derive(Debug, Deserialize)]
struct ToolsListResponse {
    tools: Vec<Tool>,
}

/// tools/call 响应
#[derive(Debug, Deserialize)]
struct ToolsCallResponse {
    #[serde(default)]
    content: Vec<Value>,
    #[serde(default)]
    isError: bool,
}

/// MCP Client（stdio 传输）
pub struct McpClient {
    child: tokio::process::Child,
    stdin: tokio::process::ChildStdin,
    stdout: BufReader<tokio::process::ChildStdout>,
    next_id: u64,
}

impl McpClient {
    /// 启动 MCP Server 并建立连接
    pub async fn connect(command: &str, args: &[String]) -> Result<Self, McpError> {
        let mut child = TokioCommand::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;

        let stdin = child.stdin.take().ok_or_else(|| {
            McpError::Protocol("Failed to capture stdin".to_string())
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            McpError::Protocol("Failed to capture stdout".to_string())
        })?;

        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            next_id: 1,
        })
    }

    /// 发送 JSON-RPC 请求并等待响应
    async fn call(&mut self, method: &str, params: Option<Value>) -> Result<Value, McpError> {
        let id = self.next_id;
        self.next_id += 1;

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params,
        };

        let request_json = serde_json::to_string(&request)?;
        self.stdin.write_all(request_json.as_bytes()).await?;
        self.stdin.write_all(b"\n").await?;
        self.stdin.flush().await?;

        // 读取响应（按行读取，直到找到匹配的 id）
        let mut line = String::new();
        loop {
            line.clear();
            let n = self.stdout.read_line(&mut line).await?;
            if n == 0 {
                return Err(McpError::Protocol("Server closed connection".to_string()));
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let response: JsonRpcResponse = match serde_json::from_str(trimmed) {
                Ok(r) => r,
                Err(_) => continue, // 跳过非 JSON 行（可能是 server 日志）
            };

            if response.id == id {
                if let Some(err) = response.error {
                    return Err(McpError::Server(format!("{}: {}", err.code, err.message)));
                }
                return Ok(response.result.unwrap_or(Value::Null));
            }
        }
    }

    /// 初始化 MCP 会话（initialize）
    pub async fn initialize(&mut self) -> Result<(), McpError> {
        let params = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "mcp-learner",
                "version": "0.1.0"
            }
        });
        self.call("initialize", Some(params)).await?;

        // 发送 initialized 通知
        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });
        let notification_json = serde_json::to_string(&notification)?;
        self.stdin.write_all(notification_json.as_bytes()).await?;
        self.stdin.write_all(b"\n").await?;
        self.stdin.flush().await?;

        Ok(())
    }

    /// 获取工具列表（tools/list）
    pub async fn list_tools(&mut self) -> Result<Vec<Tool>, McpError> {
        let result = self.call("tools/list", None).await?;
        let response: ToolsListResponse = serde_json::from_value(result)?;
        Ok(response.tools)
    }

    /// 调用工具（tools/call）
    pub async fn call_tool(&mut self, name: &str, arguments: Value) -> Result<Value, McpError> {
        let params = serde_json::json!({
            "name": name,
            "arguments": arguments
        });
        let result = self.call("tools/call", Some(params)).await?;
        let response: ToolsCallResponse = serde_json::from_value(result)?;

        if response.isError {
            return Err(McpError::Server(format!("Tool {} returned error", name)));
        }

        // 提取 text 内容
        for content in &response.content {
            if let Some(text) = content.get("text") {
                return Ok(text.clone());
            }
        }

        Ok(Value::Null)
    }

    /// 关闭连接
    pub async fn close(mut self) -> Result<(), McpError> {
        drop(self.stdin);
        let _ = self.child.wait().await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_deserialization() {
        let json = r#"{
            "name": "read_file",
            "description": "Read a file",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string"}
                },
                "required": ["path"]
            }
        }"#;
        let tool: Tool = serde_json::from_str(json).unwrap();
        assert_eq!(tool.name, "read_file");
        assert_eq!(tool.inputSchema.required, vec!["path"]);
        assert!(tool.inputSchema.properties.contains_key("path"));
    }

    #[test]
    fn test_tool_default_schema() {
        let json = r#"{"name": "simple_tool"}"#;
        let tool: Tool = serde_json::from_str(json).unwrap();
        assert_eq!(tool.name, "simple_tool");
        assert_eq!(tool.inputSchema.schema_type, "object");
        assert!(tool.inputSchema.properties.is_empty());
    }
}
