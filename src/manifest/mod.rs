//! Tentacle 插件 Manifest 生成器
//!
//! 将 CI-144 工具定义转换为 Tentacle 可加载的插件 Manifest。
//! MCP 工具通过通用 MCP 代理执行体（mcp_proxy）调用。

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use crate::ci144::{Ci144Tool, FieldProvenance, RiskLevel};
use crate::mcp::ToolAnnotations;

/// Tentacle 安全等级映射
///
/// **Append-Only**：`Unknown` 追加在末尾，既有变体绝不重编号或重排序。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SecurityLevel {
    Normal,
    Critical,
    /// 未知：无证据 ⇒ 无能力（既不是 Normal，也不能靠"少给权限"糊过去）
    Unknown,
}

impl From<RiskLevel> for SecurityLevel {
    fn from(risk: RiskLevel) -> Self {
        match risk {
            RiskLevel::Low | RiskLevel::Medium => SecurityLevel::Normal,
            RiskLevel::Critical | RiskLevel::Catastrophic => SecurityLevel::Critical,
            // 无证据 ⇒ 无能力：不得映射为 Normal（那正是本缺陷的安全倒置）
            RiskLevel::Unknown => SecurityLevel::Unknown,
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
    /// 风险等级的**来源与信任度**。
    ///
    /// 与 `pfp_risk_level` 一一对应；使"关键词猜出来的 LOW"与
    /// "server 声明的 LOW"、以及"UNKNOWN"在数据上可区分。
    /// `#[serde(default)]` 让旧 Manifest 仍可解析（缺省落到 Unknown，而不是 Low）。
    #[serde(default)]
    pub risk_provenance: FieldProvenance,
    /// server 声明的原始 `ToolAnnotations`（证据本体，原样保留以便复核）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_annotations: Option<ToolAnnotations>,
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
            // 无证据 ⇒ 无能力。
            // 不是"少给权限"：`filesystem: ["read"]` 仍然是**给**，
            // 一个猜出来的最低权限就会变成事实上的默认授权。
            RiskLevel::Unknown => Permission::default(),
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
                // 兜底强制：即便上游 Ci144Tool 把 requires_confirmation 设成 false，
                // CRITICAL/CATASTROPHIC/UNKNOWN 依然必须人工确认（单一事实来源见 RiskLevel）
                requires_confirmation: tool.requires_confirmation
                    || tool.risk_level.requires_confirmation(),
                risk_provenance: tool.risk_provenance.clone(),
                mcp_annotations: tool.mcp_annotations.clone(),
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
    use crate::ci144::{Ci144Tool, FieldProvenance, ProvenanceOrigin, RiskLevel};
    use crate::mcp::{Tool, ToolAnnotations, ToolInputSchema};

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
            requires_confirmation: risk.requires_confirmation(),
            risk_provenance: FieldProvenance::unknown(),
            mcp_annotations: None,
        }
    }

    /// 从 MCP Tool 一路提炼到 Manifest（真实链路，而非手工构造）
    fn manifest_for(mcp_tool: &Tool) -> ToolManifest {
        let generator =
            ManifestGenerator::new("test-server", "npx", vec!["-y".to_string()]);
        generator.generate_tool(&crate::ci144::extract_tool_with_namespace(mcp_tool, "test-server"))
    }

    fn mcp_tool(name: &str, annotations: Option<ToolAnnotations>) -> Tool {
        Tool {
            name: name.to_string(),
            description: format!("Test tool: {}", name),
            title: None,
            annotations,
            inputSchema: ToolInputSchema::default(),
            output_schema: None,
        }
    }

    #[test]
    fn test_security_level_mapping() {
        assert_eq!(SecurityLevel::from(RiskLevel::Low), SecurityLevel::Normal);
        assert_eq!(SecurityLevel::from(RiskLevel::Medium), SecurityLevel::Normal);
        assert_eq!(SecurityLevel::from(RiskLevel::Critical), SecurityLevel::Critical);
        assert_eq!(SecurityLevel::from(RiskLevel::Catastrophic), SecurityLevel::Critical);
        // 未知不等于正常
        assert_eq!(SecurityLevel::from(RiskLevel::Unknown), SecurityLevel::Unknown);
    }

    /// 核心回归：未知风险的工具必须拿到**空**权限，
    /// 断言在退回"默认 Low"时会失败（那时 permissions.filesystem == ["read"]）。
    #[test]
    fn test_unknown_risk_grants_no_capabilities() {
        for name in ["清理", "zap", "rm_all_data"] {
            let manifest = manifest_for(&mcp_tool(name, None));

            assert_eq!(manifest.ci144.pfp_risk_level, "UNKNOWN", "{}", name);
            assert_eq!(manifest.security_level, SecurityLevel::Unknown, "{}", name);
            assert!(manifest.ci144.requires_confirmation, "{}", name);

            assert!(
                manifest.permissions.filesystem.is_empty(),
                "{} 必须没有任何文件系统能力",
                name
            );
            assert!(
                manifest.permissions.network.is_empty(),
                "{} 必须没有任何网络能力",
                name
            );
            assert!(!manifest.permissions.execute, "{} 必须没有执行能力", name);

            // 来源与信任度必须落盘，且如实标为未知
            assert_eq!(
                manifest.ci144.risk_provenance.origin,
                ProvenanceOrigin::Unknown
            );
            assert!(!manifest.ci144.risk_provenance.trusted);

            // 既有 tags 行为保持可用，并如实反映 unknown
            assert!(manifest.tags.contains(&"mcp".to_string()));
            assert!(manifest.tags.contains(&"risk:unknown".to_string()));
        }
    }

    /// 声明优先：destructiveHint ⇒ CRITICAL + 必须人工确认
    #[test]
    fn test_declared_destructive_hint_yields_critical_manifest() {
        let manifest = manifest_for(&mcp_tool(
            "清理",
            Some(ToolAnnotations {
                destructive_hint: Some(true),
                ..Default::default()
            }),
        ));

        assert_eq!(manifest.ci144.pfp_risk_level, "CRITICAL");
        assert_eq!(manifest.security_level, SecurityLevel::Critical);
        assert!(manifest.ci144.requires_confirmation);
        assert!(manifest.permissions.execute);
        assert_eq!(
            manifest.ci144.risk_provenance.origin,
            ProvenanceOrigin::Declared
        );
        // 原始声明必须随产物一起保留
        assert_eq!(
            manifest
                .ci144
                .mcp_annotations
                .as_ref()
                .and_then(|a| a.destructive_hint),
            Some(true)
        );
    }

    /// 关键词推断出来的 LOW 必须与声明出来的 LOW 在产物里可区分
    #[test]
    fn test_keyword_low_is_distinguishable_from_declared_low() {
        let inferred = manifest_for(&mcp_tool("read_file", None));
        assert_eq!(inferred.ci144.pfp_risk_level, "LOW");
        assert_eq!(
            inferred.ci144.risk_provenance.origin,
            ProvenanceOrigin::Inferred
        );
        assert_eq!(
            inferred.ci144.risk_provenance.rule.as_deref(),
            Some("name.token.read")
        );
        assert!(inferred.ci144.mcp_annotations.is_none());

        let declared = manifest_for(&mcp_tool(
            "read_file",
            Some(ToolAnnotations {
                read_only_hint: Some(true),
                ..Default::default()
            }),
        ));
        assert_eq!(declared.ci144.pfp_risk_level, "LOW");
        assert_eq!(
            declared.ci144.risk_provenance.origin,
            ProvenanceOrigin::Declared
        );
        assert_ne!(
            inferred.ci144.risk_provenance,
            declared.ci144.risk_provenance
        );
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

    /// Manifest 层的兜底：上游漏设 requires_confirmation 时不得放行
    #[test]
    fn test_manifest_enforces_confirmation_for_unknown() {
        let generator = ManifestGenerator::new("test-server", "npx", vec![]);
        let mut tool = make_ci144_tool("zap", RiskLevel::Unknown);
        tool.requires_confirmation = false; // 上游漏设
        let manifest = generator.generate_tool(&tool);
        assert!(manifest.ci144.requires_confirmation);
        assert_eq!(manifest.security_level, SecurityLevel::Unknown);
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

    /// 旧 Manifest（没有 risk_provenance / mcp_annotations）必须仍可解析
    #[test]
    fn test_legacy_manifest_json_still_parses() {
        let legacy = r#"{
            "name": "read_file",
            "version": "0.1.0",
            "description": "Read a file",
            "tags": ["mcp", "risk:low"],
            "executable": "mcp_proxy.js",
            "integrity": {"algorithm": "sha256", "hash": "00"},
            "parameters_schema": {"type": "object"},
            "security_level": "normal",
            "permissions": {"filesystem": ["read"]},
            "ci144": {
                "mcp_name": "read_file",
                "pfp_risk_level": "LOW",
                "pfp_modality": "EXECUTIVE",
                "requires_confirmation": false,
                "mcp_server": {"command": "npx", "args": [], "transport": "stdio"}
            },
            "timeout_ms": 30000
        }"#;
        let manifest: ToolManifest = serde_json::from_str(legacy).unwrap();
        assert_eq!(manifest.security_level, SecurityLevel::Normal);
        // 缺失的溯源按"未知"处理，而不是补成"已声明"
        assert_eq!(
            manifest.ci144.risk_provenance.origin,
            ProvenanceOrigin::Unknown
        );
        assert!(manifest.ci144.mcp_annotations.is_none());
    }
}

