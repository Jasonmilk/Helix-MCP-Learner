//! 增量学习与版本管理
//!
//! 提供工具变化检测、增量学习、版本管理和废弃检测功能。

use crate::ci144::Ci144Tool;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// 学习结果版本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningVersion {
    /// 版本号（语义化版本）
    pub version: String,
    /// 学习时间
    pub timestamp: u64,
    /// MCP Server 名称
    pub server_name: String,
    /// 工具数量
    pub tool_count: usize,
    /// 工具列表
    pub tools: Vec<Ci144Tool>,
}

/// 工具变化类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolChangeType {
    /// 新增工具
    Added,
    /// 工具变更（描述或 schema 变化）
    Modified,
    /// 工具未变更
    Unchanged,
    /// 工具被删除（废弃）
    Removed,
}

/// 工具变化记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolChange {
    /// 工具名称
    pub tool_name: String,
    /// 变化类型
    pub change_type: ToolChangeType,
    /// 旧版本（如果有）
    pub old_version: Option<String>,
    /// 新版本（如果有）
    pub new_version: Option<String>,
}

/// 增量学习结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncrementalLearnResult {
    /// 变化检测结果
    pub changes: Vec<ToolChange>,
    /// 新增工具
    pub added_tools: Vec<Ci144Tool>,
    /// 变更工具
    pub modified_tools: Vec<Ci144Tool>,
    /// 未变更工具
    pub unchanged_tools: Vec<Ci144Tool>,
    /// 废弃工具
    pub removed_tools: Vec<Ci144Tool>,
    /// 是否有变化
    pub has_changes: bool,
}

/// 增量学习器
pub struct IncrementalLearner {
    /// 已学习的版本历史
    versions: HashMap<String, Vec<LearningVersion>>,
}

impl IncrementalLearner {
    /// 创建新的增量学习器
    pub fn new() -> Self {
        Self {
            versions: HashMap::new(),
        }
    }

    /// 从目录加载已有的学习结果
    pub fn from_directory(dir: &Path) -> Result<Self, String> {
        let mut learner = Self::new();

        if !dir.exists() {
            return Ok(learner);
        }

        // 扫描所有 server 子目录
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let server_name = path.file_name().unwrap().to_string_lossy().to_string();
                    // 查找索引文件
                    let index_path = path.join(format!("{}_index.json", server_name));
                    if index_path.exists() {
                        if let Ok(content) = std::fs::read_to_string(&index_path) {
                            if let Ok(version) = serde_json::from_str::<LearningVersion>(&content) {
                                learner.versions.entry(server_name).or_default().push(version);
                            }
                        }
                    }
                }
            }
        }

        Ok(learner)
    }

    /// 检测工具变化
    pub fn detect_changes(
        &self,
        server_name: &str,
        new_tools: &[Ci144Tool],
    ) -> IncrementalLearnResult {
        let old_tools = self
            .versions
            .get(server_name)
            .and_then(|versions| versions.last())
            .map(|version| version.tools.clone())
            .unwrap_or_default();

        let old_tool_map: HashMap<String, &Ci144Tool> =
            old_tools.iter().map(|t| (t.intent_name.clone(), t)).collect();
        let new_tool_map: HashMap<String, &Ci144Tool> =
            new_tools.iter().map(|t| (t.intent_name.clone(), t)).collect();

        let mut changes = Vec::new();
        let mut added_tools = Vec::new();
        let mut modified_tools = Vec::new();
        let mut unchanged_tools = Vec::new();
        let mut removed_tools = Vec::new();

        // 检测新增和变更
        for tool in new_tools {
            match old_tool_map.get(&tool.intent_name) {
                None => {
                    changes.push(ToolChange {
                        tool_name: tool.intent_name.clone(),
                        change_type: ToolChangeType::Added,
                        old_version: None,
                        new_version: Some(tool.intent_name.clone()),
                    });
                    added_tools.push(tool.clone());
                }
                Some(old_tool) => {
                    if is_tool_modified(old_tool, tool) {
                        changes.push(ToolChange {
                            tool_name: tool.intent_name.clone(),
                            change_type: ToolChangeType::Modified,
                            old_version: Some(old_tool.intent_name.clone()),
                            new_version: Some(tool.intent_name.clone()),
                        });
                        modified_tools.push(tool.clone());
                    } else {
                        changes.push(ToolChange {
                            tool_name: tool.intent_name.clone(),
                            change_type: ToolChangeType::Unchanged,
                            old_version: Some(old_tool.intent_name.clone()),
                            new_version: Some(tool.intent_name.clone()),
                        });
                        unchanged_tools.push(tool.clone());
                    }
                }
            }
        }

        // 检测删除
        for tool in &old_tools {
            if !new_tool_map.contains_key(&tool.intent_name) {
                changes.push(ToolChange {
                    tool_name: tool.intent_name.clone(),
                    change_type: ToolChangeType::Removed,
                    old_version: Some(tool.intent_name.clone()),
                    new_version: None,
                });
                removed_tools.push(tool.clone());
            }
        }

        let has_changes = !added_tools.is_empty() || !modified_tools.is_empty() || !removed_tools.is_empty();

        IncrementalLearnResult {
            changes,
            added_tools,
            modified_tools,
            unchanged_tools,
            removed_tools,
            has_changes,
        }
    }

    /// 记录新版本
    pub fn record_version(&mut self, server_name: &str, tools: Vec<Ci144Tool>) -> LearningVersion {
        let version = LearningVersion {
            version: format!("1.0.{}", self.versions.get(server_name).map(|v| v.len()).unwrap_or(0)),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            server_name: server_name.to_string(),
            tool_count: tools.len(),
            tools,
        };

        self.versions
            .entry(server_name.to_string())
            .or_default()
            .push(version.clone());

        version
    }

    /// 获取某个 server 的最新版本
    pub fn latest_version(&self, server_name: &str) -> Option<&LearningVersion> {
        self.versions.get(server_name).and_then(|v| v.last())
    }

    /// 获取某个 server 的版本历史
    pub fn version_history(&self, server_name: &str) -> &[LearningVersion] {
        self.versions.get(server_name).map(|v| v.as_slice()).unwrap_or(&[])
    }
}

impl Default for IncrementalLearner {
    fn default() -> Self {
        Self::new()
    }
}

/// 判断工具是否被修改
fn is_tool_modified(old: &Ci144Tool, new: &Ci144Tool) -> bool {
    // 比较描述
    if old.description != new.description {
        return true;
    }

    // 比较参数 schema（简化比较：比较 JSON 字符串）
    let old_schema = serde_json::to_string(&old.parameters).unwrap_or_default();
    let new_schema = serde_json::to_string(&new.parameters).unwrap_or_default();
    if old_schema != new_schema {
        return true;
    }

    // 比较风险等级
    if old.risk_level != new.risk_level {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ci144::RiskLevel;

    fn make_tool(name: &str, desc: &str, risk: RiskLevel) -> Ci144Tool {
        Ci144Tool {
            intent_name: name.to_string(),
            mcp_name: name.to_string(),
            description: desc.to_string(),
            parameters: serde_json::json!({"type": "object"}),
            risk_level: risk,
            modality: "EXECUTIVE".to_string(),
            requires_confirmation: false,
        }
    }

    #[test]
    fn test_detect_changes_no_changes() {
        let mut learner = IncrementalLearner::new();
        let tools = vec![make_tool("tool1", "desc1", RiskLevel::Low)];
        learner.record_version("test-server", tools.clone());

        let result = learner.detect_changes("test-server", &tools);
        assert!(!result.has_changes);
        assert_eq!(result.unchanged_tools.len(), 1);
        assert_eq!(result.added_tools.len(), 0);
        assert_eq!(result.modified_tools.len(), 0);
        assert_eq!(result.removed_tools.len(), 0);
    }

    #[test]
    fn test_detect_changes_added() {
        let mut learner = IncrementalLearner::new();
        let old_tools = vec![make_tool("tool1", "desc1", RiskLevel::Low)];
        learner.record_version("test-server", old_tools);

        let new_tools = vec![
            make_tool("tool1", "desc1", RiskLevel::Low),
            make_tool("tool2", "desc2", RiskLevel::Medium),
        ];
        let result = learner.detect_changes("test-server", &new_tools);
        assert!(result.has_changes);
        assert_eq!(result.added_tools.len(), 1);
        assert_eq!(result.added_tools[0].intent_name, "tool2");
    }

    #[test]
    fn test_detect_changes_modified() {
        let mut learner = IncrementalLearner::new();
        let old_tools = vec![make_tool("tool1", "desc1", RiskLevel::Low)];
        learner.record_version("test-server", old_tools);

        let new_tools = vec![make_tool("tool1", "modified desc", RiskLevel::Low)];
        let result = learner.detect_changes("test-server", &new_tools);
        assert!(result.has_changes);
        assert_eq!(result.modified_tools.len(), 1);
    }

    #[test]
    fn test_detect_changes_removed() {
        let mut learner = IncrementalLearner::new();
        let old_tools = vec![
            make_tool("tool1", "desc1", RiskLevel::Low),
            make_tool("tool2", "desc2", RiskLevel::Medium),
        ];
        learner.record_version("test-server", old_tools);

        let new_tools = vec![make_tool("tool1", "desc1", RiskLevel::Low)];
        let result = learner.detect_changes("test-server", &new_tools);
        assert!(result.has_changes);
        assert_eq!(result.removed_tools.len(), 1);
        assert_eq!(result.removed_tools[0].intent_name, "tool2");
    }

    #[test]
    fn test_record_version() {
        let mut learner = IncrementalLearner::new();
        let tools = vec![make_tool("tool1", "desc1", RiskLevel::Low)];

        let v1 = learner.record_version("test-server", tools.clone());
        assert_eq!(v1.version, "1.0.0");
        assert_eq!(v1.tool_count, 1);

        let v2 = learner.record_version("test-server", tools);
        assert_eq!(v2.version, "1.0.1");

        assert_eq!(learner.version_history("test-server").len(), 2);
        assert_eq!(learner.latest_version("test-server").unwrap().version, "1.0.1");
    }

    #[test]
    fn test_tool_change_type_serialization() {
        let change = ToolChange {
            tool_name: "test".to_string(),
            change_type: ToolChangeType::Added,
            old_version: None,
            new_version: Some("test".to_string()),
        };

        let json = serde_json::to_string(&change).unwrap();
        assert!(json.contains("\"change_type\":\"Added\""));
    }
}
