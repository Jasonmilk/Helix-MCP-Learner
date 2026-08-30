//! macOS Glove 最小版本
//!
//! 提供 macOS 系统工具，作为 MCP Server 运行，可被 MCP-Learner 学习。
//! 包含：文件系统、进程管理、AppleScript 执行。

use crate::mcp::Tool;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::process::Command;

/// macOS Glove 工具名称
pub const GLOVE_NAME: &str = "macos";

/// macOS Glove 工具执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GloveResult {
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
}

/// 获取 macOS Glove 的工具列表
pub fn tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "macos_read_file".to_string(),
            description: "Read a file from the macOS filesystem".to_string(),
            inputSchema: crate::mcp::ToolInputSchema {
                schema_type: "object".to_string(),
                properties: {
                    let mut props = std::collections::HashMap::new();
                    props.insert("path".to_string(), serde_json::json!({"type": "string", "description": "File path to read"}));
                    props
                },
                required: vec!["path".to_string()],
            },
        },
        Tool {
            name: "macos_write_file".to_string(),
            description: "Write content to a file on the macOS filesystem".to_string(),
            inputSchema: crate::mcp::ToolInputSchema {
                schema_type: "object".to_string(),
                properties: {
                    let mut props = std::collections::HashMap::new();
                    props.insert("path".to_string(), serde_json::json!({"type": "string", "description": "File path to write"}));
                    props.insert("content".to_string(), serde_json::json!({"type": "string", "description": "Content to write"}));
                    props
                },
                required: vec!["path".to_string(), "content".to_string()],
            },
        },
        Tool {
            name: "macos_list_directory".to_string(),
            description: "List files in a directory on macOS".to_string(),
            inputSchema: crate::mcp::ToolInputSchema {
                schema_type: "object".to_string(),
                properties: {
                    let mut props = std::collections::HashMap::new();
                    props.insert("directory".to_string(), serde_json::json!({"type": "string", "description": "Directory path to list"}));
                    props
                },
                required: vec!["directory".to_string()],
            },
        },
        Tool {
            name: "macos_execute_command".to_string(),
            description: "Execute a shell command on macOS".to_string(),
            inputSchema: crate::mcp::ToolInputSchema {
                schema_type: "object".to_string(),
                properties: {
                    let mut props = std::collections::HashMap::new();
                    props.insert("command".to_string(), serde_json::json!({"type": "string", "description": "Command to execute"}));
                    props.insert("args".to_string(), serde_json::json!({"type": "array", "items": {"type": "string"}, "description": "Command arguments"}));
                    props
                },
                required: vec!["command".to_string()],
            },
        },
        Tool {
            name: "macos_list_processes".to_string(),
            description: "List running processes on macOS".to_string(),
            inputSchema: crate::mcp::ToolInputSchema {
                schema_type: "object".to_string(),
                properties: std::collections::HashMap::new(),
                required: vec![],
            },
        },
        Tool {
            name: "macos_run_applescript".to_string(),
            description: "Execute an AppleScript on macOS".to_string(),
            inputSchema: crate::mcp::ToolInputSchema {
                schema_type: "object".to_string(),
                properties: {
                    let mut props = std::collections::HashMap::new();
                    props.insert("script".to_string(), serde_json::json!({"type": "string", "description": "AppleScript code to execute"}));
                    props
                },
                required: vec!["script".to_string()],
            },
        },
    ]
}

/// 执行 macOS Glove 工具
pub async fn execute_tool(tool_name: &str, arguments: &Value) -> GloveResult {
    match tool_name {
        "macos_read_file" => execute_read_file(arguments),
        "macos_write_file" => execute_write_file(arguments),
        "macos_list_directory" => execute_list_directory(arguments),
        "macos_execute_command" => execute_execute_command(arguments),
        "macos_list_processes" => execute_list_processes(),
        "macos_run_applescript" => execute_run_applescript(arguments),
        _ => GloveResult {
            success: false,
            output: None,
            error: Some(format!("Unknown tool: {}", tool_name)),
        },
    }
}

/// 执行 read_file
fn execute_read_file(arguments: &Value) -> GloveResult {
    let path = match arguments.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return GloveResult {
            success: false,
            output: None,
            error: Some("Missing 'path' argument".to_string()),
        },
    };

    match std::fs::read_to_string(path) {
        Ok(content) => GloveResult {
            success: true,
            output: Some(content),
            error: None,
        },
        Err(e) => GloveResult {
            success: false,
            output: None,
            error: Some(format!("Failed to read file: {}", e)),
        },
    }
}

/// 执行 write_file
fn execute_write_file(arguments: &Value) -> GloveResult {
    let path = match arguments.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return GloveResult {
            success: false,
            output: None,
            error: Some("Missing 'path' argument".to_string()),
        },
    };

    let content = match arguments.get("content").and_then(|v| v.as_str()) {
        Some(c) => c,
        None => return GloveResult {
            success: false,
            output: None,
            error: Some("Missing 'content' argument".to_string()),
        },
    };

    match std::fs::write(path, content) {
        Ok(_) => GloveResult {
            success: true,
            output: Some(format!("File written: {}", path)),
            error: None,
        },
        Err(e) => GloveResult {
            success: false,
            output: None,
            error: Some(format!("Failed to write file: {}", e)),
        },
    }
}

/// 执行 list_directory
fn execute_list_directory(arguments: &Value) -> GloveResult {
    let directory = match arguments.get("directory").and_then(|v| v.as_str()) {
        Some(d) => d,
        None => return GloveResult {
            success: false,
            output: None,
            error: Some("Missing 'directory' argument".to_string()),
        },
    };

    match std::fs::read_dir(directory) {
        Ok(entries) => {
            let files: Vec<String> = entries
                .filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect();
            GloveResult {
                success: true,
                output: Some(files.join("\n")),
                error: None,
            }
        }
        Err(e) => GloveResult {
            success: false,
            output: None,
            error: Some(format!("Failed to list directory: {}", e)),
        },
    }
}

/// 执行 execute_command
fn execute_execute_command(arguments: &Value) -> GloveResult {
    let command = match arguments.get("command").and_then(|v| v.as_str()) {
        Some(c) => c,
        None => return GloveResult {
            success: false,
            output: None,
            error: Some("Missing 'command' argument".to_string()),
        },
    };

    let args: Vec<String> = arguments
        .get("args")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    match Command::new(command).args(&args).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let result = if output.status.success() {
                GloveResult {
                    success: true,
                    output: Some(if stderr.is_empty() { stdout } else { format!("{}\n{}", stdout, stderr) }),
                    error: None,
                }
            } else {
                GloveResult {
                    success: false,
                    output: Some(stdout),
                    error: Some(if stderr.is_empty() { format!("Command failed with status: {}", output.status) } else { stderr }),
                }
            };
            result
        }
        Err(e) => GloveResult {
            success: false,
            output: None,
            error: Some(format!("Failed to execute command: {}", e)),
        },
    }
}

/// 执行 list_processes
fn execute_list_processes() -> GloveResult {
    match Command::new("ps").args(["-axo", "pid,comm"]).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            GloveResult {
                success: true,
                output: Some(stdout),
                error: None,
            }
        }
        Err(e) => GloveResult {
            success: false,
            output: None,
            error: Some(format!("Failed to list processes: {}", e)),
        },
    }
}

/// 执行 run_applescript
fn execute_run_applescript(arguments: &Value) -> GloveResult {
    let script = match arguments.get("script").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return GloveResult {
            success: false,
            output: None,
            error: Some("Missing 'script' argument".to_string()),
        },
    };

    match Command::new("osascript").args(["-e", script]).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            if output.status.success() {
                GloveResult {
                    success: true,
                    output: Some(stdout),
                    error: None,
                }
            } else {
                GloveResult {
                    success: false,
                    output: Some(stdout),
                    error: Some(if stderr.is_empty() { "AppleScript failed".to_string() } else { stderr }),
                }
            }
        }
        Err(e) => GloveResult {
            success: false,
            output: None,
            error: Some(format!("Failed to execute AppleScript: {}", e)),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tools_count() {
        let tools = tools();
        assert_eq!(tools.len(), 6);
    }

    #[test]
    fn test_tool_names() {
        let tools = tools();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"macos_read_file"));
        assert!(names.contains(&"macos_write_file"));
        assert!(names.contains(&"macos_list_directory"));
        assert!(names.contains(&"macos_execute_command"));
        assert!(names.contains(&"macos_list_processes"));
        assert!(names.contains(&"macos_run_applescript"));
    }

    #[tokio::test]
    async fn test_execute_unknown_tool() {
        let result = execute_tool("unknown_tool", &serde_json::json!({})).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Unknown tool"));
    }

    #[tokio::test]
    async fn test_read_file_missing_path() {
        let result = execute_tool("macos_read_file", &serde_json::json!({})).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Missing 'path'"));
    }

    #[tokio::test]
    async fn test_read_write_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let test_file = temp_dir.path().join("test.txt");

        // 写入文件
        let write_result = execute_tool(
            "macos_write_file",
            &serde_json::json!({"path": test_file.to_string_lossy(), "content": "Hello macOS!"}),
        )
        .await;
        assert!(write_result.success);

        // 读取文件
        let read_result = execute_tool(
            "macos_read_file",
            &serde_json::json!({"path": test_file.to_string_lossy()}),
        )
        .await;
        assert!(read_result.success);
        assert_eq!(read_result.output.unwrap(), "Hello macOS!");
    }

    #[tokio::test]
    async fn test_list_directory() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::fs::write(temp_dir.path().join("file1.txt"), "test").unwrap();
        std::fs::write(temp_dir.path().join("file2.txt"), "test").unwrap();

        let result = execute_tool(
            "macos_list_directory",
            &serde_json::json!({"directory": temp_dir.path().to_string_lossy()}),
        )
        .await;
        assert!(result.success);
        let output = result.output.unwrap();
        assert!(output.contains("file1.txt"));
        assert!(output.contains("file2.txt"));
    }

    #[tokio::test]
    async fn test_execute_command() {
        let result = execute_tool(
            "macos_execute_command",
            &serde_json::json!({"command": "echo", "args": ["Hello World"]}),
        )
        .await;
        assert!(result.success);
        assert!(result.output.unwrap().contains("Hello World"));
    }
}
