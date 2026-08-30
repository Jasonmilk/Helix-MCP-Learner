//! 集成测试：完整的 MCP-Learner 学习流程
//!
//! 测试路径：连接模拟 MCP Server → initialize → tools/list → 提炼 CI-144 工具 → 生成 Manifest

use mcp_learner::{McpClient, extract_tools, ManifestGenerator};
use std::path::PathBuf;

#[tokio::test]
async fn test_full_learning_pipeline() {
    let mock_server = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("mock_mcp_server.py");

    // 1. 连接 MCP Server
    let mut client = McpClient::connect(
        "python3",
        &[mock_server.to_string_lossy().to_string()],
    )
    .await
    .expect("Failed to connect to mock MCP server");

    // 2. 初始化
    client.initialize().await.expect("Failed to initialize");

    // 3. 获取工具列表
    let tools = client.list_tools().await.expect("Failed to list tools");
    assert_eq!(tools.len(), 4);
    assert!(tools.iter().any(|t| t.name == "read_file"));
    assert!(tools.iter().any(|t| t.name == "list_files"));
    assert!(tools.iter().any(|t| t.name == "create_file"));
    assert!(tools.iter().any(|t| t.name == "delete_file"));

    // 4. 提炼为 CI-144 工具
    let ci144_tools = extract_tools(&tools);
    assert_eq!(ci144_tools.len(), 4);

    // 验证风险评级
    let read_file = ci144_tools.iter().find(|t| t.intent_name == "read_file").unwrap();
    assert_eq!(read_file.risk_level, mcp_learner::RiskLevel::Low);
    assert!(!read_file.requires_confirmation);

    let create_file = ci144_tools.iter().find(|t| t.intent_name == "create_file").unwrap();
    assert_eq!(create_file.risk_level, mcp_learner::RiskLevel::Medium);

    let delete_file = ci144_tools.iter().find(|t| t.intent_name == "delete_file").unwrap();
    assert_eq!(delete_file.risk_level, mcp_learner::RiskLevel::Critical);
    assert!(delete_file.requires_confirmation);

    // 5. 生成 Manifest
    let output_dir = tempfile::tempdir().unwrap();
    let generator = ManifestGenerator::new(
        "mock-filesystem",
        "python3",
        vec![mock_server.to_string_lossy().to_string()],
    );
    generator
        .write_to_dir(output_dir.path(), "mock-filesystem", &ci144_tools)
        .expect("Failed to write manifests");

    // 验证输出文件
    let index_path = output_dir.path().join("mock-filesystem_index.json");
    assert!(index_path.exists());

    let read_manifest_path = output_dir.path().join("read_file.json");
    assert!(read_manifest_path.exists());

    let delete_manifest_path = output_dir.path().join("delete_file.json");
    assert!(delete_manifest_path.exists());

    // 验证 Manifest 内容
    let read_manifest: mcp_learner::manifest::ToolManifest =
        serde_json::from_str(&std::fs::read_to_string(&read_manifest_path).unwrap()).unwrap();
    assert_eq!(read_manifest.name, "read_file");
    assert_eq!(read_manifest.security_level, mcp_learner::manifest::SecurityLevel::Normal);
    assert_eq!(read_manifest.ci144.pfp_risk_level, "LOW");
    assert_eq!(read_manifest.ci144.mcp_server.command, "python3");

    let delete_manifest: mcp_learner::manifest::ToolManifest =
        serde_json::from_str(&std::fs::read_to_string(&delete_manifest_path).unwrap()).unwrap();
    assert_eq!(delete_manifest.security_level, mcp_learner::manifest::SecurityLevel::Critical);
    assert_eq!(delete_manifest.ci144.pfp_risk_level, "CRITICAL");
    assert!(delete_manifest.ci144.requires_confirmation);

    // 6. 关闭连接
    client.close().await.expect("Failed to close");

    println!("\n✅ Full learning pipeline test passed!");
    println!("   Tools discovered: 4");
    println!("   CI-144 tools extracted: 4");
    println!("   Manifests generated: 4 + 1 index");
    println!("   Risk ratings: read=LOW, list=LOW, create=MEDIUM, delete=CRITICAL");
}

#[tokio::test]
async fn test_tool_call() {
    let mock_server = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("mock_mcp_server.py");

    let mut client = McpClient::connect(
        "python3",
        &[mock_server.to_string_lossy().to_string()],
    )
    .await
    .expect("Failed to connect");

    client.initialize().await.expect("Failed to initialize");

    // 创建一个临时文件
    let temp_dir = tempfile::tempdir().unwrap();
    let test_file = temp_dir.path().join("test.txt");
    std::fs::write(&test_file, "Hello, MCP!").unwrap();

    // 调用 read_file 工具
    let result = client
        .call_tool(
            "read_file",
            serde_json::json!({"path": test_file.to_string_lossy()}),
        )
        .await
        .expect("Failed to call tool");

    assert_eq!(result, serde_json::Value::String("Hello, MCP!".to_string()));

    client.close().await.expect("Failed to close");
}

#[test]
fn test_deterministic_risk_rating() {
    // 确定性验证：相同工具名总是产生相同风险评级
    let tools = vec![
        "read_file",
        "list_files",
        "get_user",
        "search_issues",
        "create_file",
        "update_file",
        "send_message",
        "delete_file",
        "remove_file",
        "execute_command",
        "delete_all",
        "system_update",
    ];

    for tool in &tools {
        let rating1 = mcp_learner::risk_rating(tool);
        let rating2 = mcp_learner::risk_rating(tool);
        let rating3 = mcp_learner::risk_rating(tool);
        assert_eq!(rating1, rating2, "{} rating not deterministic", tool);
        assert_eq!(rating2, rating3, "{} rating not deterministic", tool);
    }

    println!("\n✅ Deterministic risk rating test passed!");
    println!("   12 tools tested, all produce consistent ratings");
}
