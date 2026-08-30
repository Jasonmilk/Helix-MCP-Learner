//! CI-144 工具提炼层
//!
//! 将 MCP 工具定义提炼为 CI-144 标准工具定义：
//! - CIN7 意图（工具名映射）
//! - CAPABILITY-13 能力声明（参数 schema 转换）
//! - PFP 风险评级（基于工具名模式自动分配 Risk-Level）

use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::mcp::Tool;

/// PFP Risk-Level（与 CI-144 v2.0 PFP-xCF14 对齐）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    Low = 0,
    Medium = 1,
    Critical = 2,
    Catastrophic = 3,
}

impl RiskLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            RiskLevel::Low => "LOW",
            RiskLevel::Medium => "MEDIUM",
            RiskLevel::Critical => "CRITICAL",
            RiskLevel::Catastrophic => "CATASTROPHIC",
        }
    }
}

/// CI-144 工具定义（提炼后的标准格式）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ci144Tool {
    /// CIN7 意图名（从 MCP 工具名映射）
    pub intent_name: String,
    /// 原始 MCP 工具名
    pub mcp_name: String,
    /// 工具描述
    pub description: String,
    /// CAPABILITY-13 参数 schema（JSON Schema）
    pub parameters: Value,
    /// PFP Risk-Level（自动评级）
    pub risk_level: RiskLevel,
    /// PFP Modality（EXECUTIVE，因为是工具执行）
    pub modality: String,
    /// 是否需要人工确认（CRITICAL/CATASTROPHIC 级别）
    pub requires_confirmation: bool,
}

/// 风险评级规则
///
/// 基于工具名模式自动分配 PFP Risk-Level：
/// - read_*/list_*/get_*/search_* → LOW
/// - create_*/update_*/write_*/send_* → MEDIUM
/// - delete_*/remove_*/execute_*/run_* → CRITICAL
/// - *_all/*_system/*_admin/*_root → CATASTROPHIC
pub fn risk_rating(tool_name: &str) -> RiskLevel {
    let name = tool_name.to_lowercase();

    // CATASTROPHIC 模式（最高优先级）
    if name.ends_with("_all")
        || name.ends_with("_system")
        || name.ends_with("_admin")
        || name.ends_with("_root")
        || name.contains("delete_all")
        || name.contains("system_update")
        || name.contains("admin_override")
    {
        return RiskLevel::Catastrophic;
    }

    // CRITICAL 模式
    if name.starts_with("delete_")
        || name.starts_with("remove_")
        || name.starts_with("execute_")
        || name.starts_with("run_")
        || name.starts_with("drop_")
        || name.starts_with("truncate_")
        || name.contains("delete")
        || name.contains("remove")
        || name.contains("execute")
    {
        return RiskLevel::Critical;
    }

    // MEDIUM 模式
    if name.starts_with("create_")
        || name.starts_with("update_")
        || name.starts_with("write_")
        || name.starts_with("send_")
        || name.starts_with("post_")
        || name.starts_with("put_")
        || name.starts_with("patch_")
        || name.starts_with("edit_")
        || name.starts_with("modify_")
        || name.contains("create")
        || name.contains("update")
        || name.contains("write")
        || name.contains("send")
    {
        return RiskLevel::Medium;
    }

    // LOW 模式（默认，读取类操作）
    RiskLevel::Low
}

/// 将 MCP 工具提炼为 CI-144 工具定义
pub fn extract_tool(mcp_tool: &Tool) -> Ci144Tool {
    let risk = risk_rating(&mcp_tool.name);

    Ci144Tool {
        intent_name: mcp_tool.name.clone(),
        mcp_name: mcp_tool.name.clone(),
        description: mcp_tool.description.clone(),
        parameters: serde_json::to_value(&mcp_tool.inputSchema).unwrap_or(Value::Null),
        risk_level: risk,
        modality: "EXECUTIVE".to_string(),
        requires_confirmation: matches!(risk, RiskLevel::Critical | RiskLevel::Catastrophic),
    }
}

/// 批量提炼工具
pub fn extract_tools(mcp_tools: &[Tool]) -> Vec<Ci144Tool> {
    mcp_tools.iter().map(extract_tool).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::ToolInputSchema;

    fn make_tool(name: &str) -> Tool {
        Tool {
            name: name.to_string(),
            description: format!("Test tool: {}", name),
            inputSchema: ToolInputSchema::default(),
        }
    }

    #[test]
    fn test_risk_rating_low() {
        assert_eq!(risk_rating("read_file"), RiskLevel::Low);
        assert_eq!(risk_rating("list_files"), RiskLevel::Low);
        assert_eq!(risk_rating("get_user"), RiskLevel::Low);
        assert_eq!(risk_rating("search_issues"), RiskLevel::Low);
        assert_eq!(risk_rating("query"), RiskLevel::Low); // 默认 LOW
    }

    #[test]
    fn test_risk_rating_medium() {
        assert_eq!(risk_rating("create_file"), RiskLevel::Medium);
        assert_eq!(risk_rating("update_file"), RiskLevel::Medium);
        assert_eq!(risk_rating("write_file"), RiskLevel::Medium);
        assert_eq!(risk_rating("send_message"), RiskLevel::Medium);
        assert_eq!(risk_rating("post_comment"), RiskLevel::Medium);
    }

    #[test]
    fn test_risk_rating_critical() {
        assert_eq!(risk_rating("delete_file"), RiskLevel::Critical);
        assert_eq!(risk_rating("remove_file"), RiskLevel::Critical);
        assert_eq!(risk_rating("execute_command"), RiskLevel::Critical);
        assert_eq!(risk_rating("run_script"), RiskLevel::Critical);
        assert_eq!(risk_rating("drop_table"), RiskLevel::Critical);
    }

    #[test]
    fn test_risk_rating_catastrophic() {
        assert_eq!(risk_rating("delete_all"), RiskLevel::Catastrophic);
        assert_eq!(risk_rating("system_update"), RiskLevel::Catastrophic);
        assert_eq!(risk_rating("admin_override"), RiskLevel::Catastrophic);
        assert_eq!(risk_rating("remove_all"), RiskLevel::Catastrophic);
    }

    #[test]
    fn test_extract_tool() {
        let tool = make_tool("read_file");
        let ci144 = extract_tool(&tool);
        assert_eq!(ci144.intent_name, "read_file");
        assert_eq!(ci144.risk_level, RiskLevel::Low);
        assert_eq!(ci144.modality, "EXECUTIVE");
        assert!(!ci144.requires_confirmation);
    }

    #[test]
    fn test_extract_tool_critical_requires_confirmation() {
        let tool = make_tool("delete_file");
        let ci144 = extract_tool(&tool);
        assert_eq!(ci144.risk_level, RiskLevel::Critical);
        assert!(ci144.requires_confirmation);
    }

    #[test]
    fn test_extract_tools_batch() {
        let tools = vec![
            make_tool("read_file"),
            make_tool("create_file"),
            make_tool("delete_file"),
        ];
        let ci144_tools = extract_tools(&tools);
        assert_eq!(ci144_tools.len(), 3);
        assert_eq!(ci144_tools[0].risk_level, RiskLevel::Low);
        assert_eq!(ci144_tools[1].risk_level, RiskLevel::Medium);
        assert_eq!(ci144_tools[2].risk_level, RiskLevel::Critical);
    }
}
