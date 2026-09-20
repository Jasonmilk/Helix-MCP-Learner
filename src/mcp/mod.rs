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

/// MCP `ToolAnnotations`（MCP 2025-06-18 规范声明）
///
/// # 这是**声明**，不是事实
///
/// MCP 规范原文明确要求：客户端 **必须（MUST）** 将 tool annotations 视为
/// **不可信（untrusted）**，除非它们来自受信任的 server。
/// 因此一个 annotation 是**带来源的证据**，其可信度由来源决定，
/// 上游消费方必须能看到"它来自谁"（见 `FieldProvenance.source` / `trusted`）。
///
/// # `None` ≠ `false`
///
/// 每个 hint 都是 `Option<bool>`：
/// - `Some(true)`  = server 明确声明"是"
/// - `Some(false)` = server 明确声明"否"
/// - `None`        = **server 没有说** —— 这不是"否"，更不是"安全"
///
/// 所以绝不能给 hint 设 `#[serde(default = "false")]`：那会把"沉默"伪造为"声明"。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolAnnotations {
    /// 人类可读标题
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// 只读提示：`Some(true)` ⇒ 该工具不会修改其环境
    #[serde(default, rename = "readOnlyHint", skip_serializing_if = "Option::is_none")]
    pub read_only_hint: Option<bool>,
    /// 破坏性提示：`Some(true)` ⇒ 该工具可能执行破坏性更新
    #[serde(default, rename = "destructiveHint", skip_serializing_if = "Option::is_none")]
    pub destructive_hint: Option<bool>,
    /// 幂等提示：仅**参考**，不单独决定风险等级
    #[serde(default, rename = "idempotentHint", skip_serializing_if = "Option::is_none")]
    pub idempotent_hint: Option<bool>,
    /// 开放世界提示：仅**参考**，不单独决定风险等级
    #[serde(default, rename = "openWorldHint", skip_serializing_if = "Option::is_none")]
    pub open_world_hint: Option<bool>,
}

/// MCP 工具定义
///
/// 保留 MCP 2025-06-18 声明的字段，而不是丢弃它们：
/// 下游的风险评级必须建立在 server 的声明之上（见 `crate::ci144::assess_risk`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// 人类可读标题（MCP 2025-06-18 `title`；可选）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// server 的声明（不可信证据，见 `ToolAnnotations` 文档）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub annotations: Option<ToolAnnotations>,
    #[serde(default)]
    pub inputSchema: ToolInputSchema,
    /// 工具输出的 JSON Schema（MCP 2025-06-18 `outputSchema`；可选）
    #[serde(default, rename = "outputSchema", skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<Value>,
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
    /// 结构化输出（MCP 2025-06-18 `structuredContent`；可选）
    #[serde(default, rename = "structuredContent")]
    structured_content: Option<Value>,
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

        // 提取 text 内容（保持既有优先级不变）
        for content in &response.content {
            if let Some(text) = content.get("text") {
                return Ok(text.clone());
            }
        }

        // 无 text 时回退到结构化输出（MCP 2025-06-18 `structuredContent`）
        if let Some(structured) = response.structured_content {
            return Ok(structured);
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

    /// 旧 server 的 payload（没有 title / annotations / outputSchema）必须仍能解析，
    /// 且缺失的 **hint 是 `None`（"server 没有说"）而不是 `Some(false)`**。
    #[test]
    fn test_absent_annotation_is_none_not_false() {
        let json = r#"{"name": "zap", "description": "unconventional"}"#;
        let tool: Tool = serde_json::from_str(json).unwrap();

        assert!(tool.title.is_none());
        assert!(tool.output_schema.is_none());
        assert!(tool.annotations.is_none(), "缺失的 annotations 必须是 None");

        // 显式构造"空声明"：所有 hint 都缺省 ⇒ 全部 None，而不是 false
        let empty = ToolAnnotations::default();
        assert_eq!(empty.read_only_hint, None);
        assert_eq!(empty.destructive_hint, None);
        assert_eq!(empty.idempotent_hint, None);
        assert_eq!(empty.open_world_hint, None);

        // 只有 server 明确写了 false，才是 false
        let json = r#"{"name": "zap", "annotations": {"readOnlyHint": false}}"#;
        let tool: Tool = serde_json::from_str(json).unwrap();
        assert_eq!(
            tool.annotations.unwrap().read_only_hint,
            Some(false),
            "显式 false 与缺省 None 必须可区分"
        );
    }

    /// MCP 2025-06-18 声明的字段必须被保留（而不是丢弃）。
    #[test]
    fn test_full_mcp_2025_06_18_tool_deserialization() {
        let json = r#"{
            "name": "清理",
            "title": "Cleanup",
            "description": "Clean up stale data",
            "annotations": {
                "title": "Cleanup annotations",
                "readOnlyHint": false,
                "destructiveHint": true,
                "idempotentHint": true,
                "openWorldHint": false
            },
            "inputSchema": {"type": "object"},
            "outputSchema": {"type": "object", "properties": {"removed": {"type": "integer"}}}
        }"#;

        let tool: Tool = serde_json::from_str(json).unwrap();
        assert_eq!(tool.name, "清理");
        assert_eq!(tool.title.as_deref(), Some("Cleanup"));

        let ann = tool.annotations.expect("annotations 必须被保留");
        assert_eq!(ann.title.as_deref(), Some("Cleanup annotations"));
        assert_eq!(ann.read_only_hint, Some(false));
        assert_eq!(ann.destructive_hint, Some(true));
        assert_eq!(ann.idempotent_hint, Some(true));
        assert_eq!(ann.open_world_hint, Some(false));

        let out = tool.output_schema.expect("outputSchema 必须被保留");
        assert!(out.get("properties").is_some());
    }

    /// 既有序列化字段的线上格式不得改变：可选新字段在缺失时**不输出**，
    /// 旧字段名（`inputSchema` 等）保持不变。
    #[test]
    fn test_tool_serialization_wire_format_unchanged_when_absent() {
        let json = r#"{
            "name": "read_file",
            "description": "Read a file",
            "inputSchema": {"type": "object", "properties": {}, "required": []}
        }"#;
        let tool: Tool = serde_json::from_str(json).unwrap();
        let out = serde_json::to_value(&tool).unwrap();
        let obj = out.as_object().unwrap();

        assert_eq!(obj.len(), 3, "缺失的可选字段不得出现在线上格式里: {}", out);
        assert!(!obj.contains_key("title"));
        assert!(!obj.contains_key("annotations"));
        assert!(!obj.contains_key("outputSchema"));
        assert!(obj.contains_key("inputSchema"), "既有字段名不得改变");
        assert!(obj.contains_key("description"));
    }

    #[test]
    fn test_tool_call_response_structured_content() {
        let json = r#"{
            "content": [{"type": "text", "text": "{\"ok\":true}"}],
            "structuredContent": {"ok": true},
            "isError": false
        }"#;
        let resp: ToolsCallResponse = serde_json::from_str(json).unwrap();
        assert_eq!(
            resp.structured_content,
            Some(serde_json::json!({"ok": true}))
        );

        // 旧 server：没有 structuredContent ⇒ None
        let resp: ToolsCallResponse =
            serde_json::from_str(r#"{"content": [], "isError": false}"#).unwrap();
        assert!(resp.structured_content.is_none());
    }
}
