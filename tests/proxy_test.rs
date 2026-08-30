//! MCP 代理执行体集成测试
//!
//! 验证：McpProxy 可以管理 MCP Server 生命周期，并成功调用工具。

use mcp_learner::{McpProxy, McpServerConfig, ToolCallRequest};
use std::path::PathBuf;

#[tokio::test]
async fn test_proxy_end_to_end_tool_call() {
    let mock_server = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("mock_mcp_server.py");

    // 创建代理
    let proxy = McpProxy::new();

    // 添加 MCP Server
    let config = McpServerConfig {
        name: "mock-fs".to_string(),
        command: "python3".to_string(),
        args: vec![mock_server.to_string_lossy().to_string()],
        transport: "stdio".to_string(),
    };
    proxy.add_server(config);

    // 等待异步添加完成
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // 验证 server 已添加
    let servers = proxy.list_servers().await;
    assert_eq!(servers.len(), 1);
    assert_eq!(servers[0], "mock-fs");

    // 获取工具列表
    let tools = proxy.list_tools("mock-fs").await.unwrap();
    assert_eq!(tools.len(), 4);
    assert!(tools.iter().any(|t| t.name == "read_file"));
    assert!(tools.iter().any(|t| t.name == "create_file"));

    // 创建临时文件
    let temp_dir = tempfile::tempdir().unwrap();
    let test_file = temp_dir.path().join("proxy_test.txt");
    std::fs::write(&test_file, "Hello from proxy!").unwrap();

    // 调用 read_file 工具
    let request = ToolCallRequest {
        server: "mock-fs".to_string(),
        tool: "read_file".to_string(),
        arguments: serde_json::json!({"path": test_file.to_string_lossy()}),
    };

    let response = proxy.call_tool(request).await;
    assert!(response.success, "Tool call failed: {:?}", response.error);
    assert_eq!(response.result, Some(serde_json::Value::String("Hello from proxy!".to_string())));
    assert!(response.duration_ms < 5000, "Tool call took too long: {}ms", response.duration_ms);

    println!("\n✅ Proxy end-to-end test passed!");
    println!("   Server: mock-fs");
    println!("   Tools: 4");
    println!("   Tool call: read_file -> 'Hello from proxy!'");
    println!("   Duration: {}ms", response.duration_ms);

    // 关闭代理
    proxy.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_proxy_tool_not_found() {
    let proxy = McpProxy::new();

    // 调用不存在的 server
    let request = ToolCallRequest {
        server: "non-existent".to_string(),
        tool: "any_tool".to_string(),
        arguments: serde_json::json!({}),
    };

    let response = proxy.call_tool(request).await;
    assert!(!response.success);
    assert!(response.error.unwrap().contains("not found"));
}

#[tokio::test]
async fn test_proxy_multiple_tool_calls() {
    let mock_server = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("mock_mcp_server.py");

    let proxy = McpProxy::new();
    let config = McpServerConfig {
        name: "mock-fs".to_string(),
        command: "python3".to_string(),
        args: vec![mock_server.to_string_lossy().to_string()],
        transport: "stdio".to_string(),
    };
    proxy.add_server(config);
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let temp_dir = tempfile::tempdir().unwrap();

    // 连续调用多个工具
    for i in 0..5 {
        let file_path = temp_dir.path().join(format!("file_{}.txt", i));
        let content = format!("Content {}", i);

        // create_file
        let create_request = ToolCallRequest {
            server: "mock-fs".to_string(),
            tool: "create_file".to_string(),
            arguments: serde_json::json!({"path": file_path.to_string_lossy(), "content": content}),
        };
        let create_response = proxy.call_tool(create_request).await;
        assert!(create_response.success, "create_file failed: {:?}", create_response.error);

        // read_file
        let read_request = ToolCallRequest {
            server: "mock-fs".to_string(),
            tool: "read_file".to_string(),
            arguments: serde_json::json!({"path": file_path.to_string_lossy()}),
        };
        let read_response = proxy.call_tool(read_request).await;
        assert!(read_response.success);
        assert_eq!(read_response.result, Some(serde_json::Value::String(format!("Content {}", i))));
    }

    println!("\n✅ Multiple tool calls test passed!");
    println!("   5 create_file + 5 read_file = 10 calls");

    proxy.shutdown().await.unwrap();
}
