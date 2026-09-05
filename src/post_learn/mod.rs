//! Post-Learn 审查管道 — L1 静态审查 + 状态迁移自动化
//!
//! MCP-Learner 学习完成后，对生成的 ToolManifest 进行自动静态审查，
//! 并根据审查结果将工具写入不同的状态目录：
//!
//! - `stable/`：无 Error 且无 Warning（或仅有 Info），可直接使用
//! - `staging/`：有 Warning 但无 Error，需人工确认或 L2 验证
//! - `rejected/`：有 Error，必须修复后重新学习
//!
//! # 设计哲学
//!
//! - **按需驱动**：审查由学习完成事件触发，无轮询
//! - **极致解耦**：审查管道独立于 ManifestGenerator，可单独测试和替换
//! - **白盒可审计**：所有审查结果和状态迁移都明文记录
//! - **可演化**：L1 规则可升级，状态迁移逻辑可扩展（L2/L3 接入点）

use helix_eco_glove_core::reviewer::{ReviewReport, RULES_VERSION, Severity, StaticReviewer, ToolManifest as ReviewManifest};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::manifest::ToolManifest;

// ============================================================================
// 状态枚举
// ============================================================================

/// 工具状态（raw → staging → stable，或 rejected）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolState {
    /// 原始学习结果（未审查）
    Raw,
    /// 审查通过，稳定可用
    Stable,
    /// 有警告，待确认
    Staging,
    /// 审查失败，被拒绝
    Rejected,
}

impl ToolState {
    /// 状态对应的目录名
    pub fn dir_name(&self) -> &'static str {
        match self {
            ToolState::Raw => "raw",
            ToolState::Stable => "stable",
            ToolState::Staging => "staging",
            ToolState::Rejected => "rejected",
        }
    }
}

impl std::fmt::Display for ToolState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.dir_name())
    }
}

// ============================================================================
// 单个工具的审查结果
// ============================================================================

/// 单个工具的审查结果（含状态迁移决策）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolReviewResult {
    /// 工具名称
    pub tool_name: String,
    /// 工具版本
    pub tool_version: String,
    /// 审查报告
    pub report: ReviewReport,
    /// 状态迁移决策
    pub state: ToolState,
    /// 输出文件路径（相对于输出根目录）
    pub output_path: PathBuf,
}

impl ToolReviewResult {
    /// 是否通过审查（状态为 Stable 或 Staging）
    pub fn passed(&self) -> bool {
        matches!(self.state, ToolState::Stable | ToolState::Staging)
    }
}

// ============================================================================
// 批量审查结果
// ============================================================================

/// 批量审查结果（一个 MCP Server 的所有工具）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BatchReviewResult {
    /// MCP Server 名称
    pub server_name: String,
    /// 审查时间（Unix 时间戳）
    pub reviewed_at: String,
    /// 审查规则版本
    pub rules_version: String,
    /// 各状态的工具数量统计
    pub state_counts: HashMap<String, usize>,
    /// 所有工具的审查结果
    pub results: Vec<ToolReviewResult>,
}

impl BatchReviewResult {
    /// 获取指定状态的工具列表
    pub fn tools_by_state(&self, state: ToolState) -> Vec<&ToolReviewResult> {
        self.results.iter().filter(|r| r.state == state).collect()
    }

    /// 通过率（Stable + Staging）/ 总数
    pub fn pass_rate(&self) -> f64 {
        if self.results.is_empty() {
            return 1.0;
        }
        let passed = self.results.iter().filter(|r| r.passed()).count();
        passed as f64 / self.results.len() as f64
    }
}

// ============================================================================
// 审查管道配置
// ============================================================================

/// 审查管道配置
#[derive(Debug, Clone)]
pub struct ReviewPipelineConfig {
    /// 输出根目录
    pub output_root: PathBuf,
    /// 是否将 Warning 工具放入 staging（true）或 stable（false）
    pub warning_to_staging: bool,
    /// 是否生成审查报告 JSON 文件
    pub generate_report: bool,
}

impl Default for ReviewPipelineConfig {
    fn default() -> Self {
        Self {
            output_root: PathBuf::from("./learned_tools"),
            warning_to_staging: true,
            generate_report: true,
        }
    }
}

// ============================================================================
// 审查管道
// ============================================================================

/// Post-Learn 审查管道
///
/// 接收 MCP-Learner 生成的 ToolManifest 列表，进行 L1 静态审查，
/// 并根据审查结果将工具写入不同的状态目录。
pub struct ReviewPipeline {
    reviewer: StaticReviewer,
    config: ReviewPipelineConfig,
}

impl ReviewPipeline {
    /// 创建新的审查管道
    pub fn new(config: ReviewPipelineConfig) -> Self {
        Self {
            reviewer: StaticReviewer::new(),
            config,
        }
    }

    /// 使用默认配置创建审查管道
    pub fn with_output_root(output_root: impl Into<PathBuf>) -> Self {
        Self::new(ReviewPipelineConfig {
            output_root: output_root.into(),
            ..Default::default()
        })
    }

    /// 审查单个工具 Manifest
    pub fn review_tool(&self, manifest: &ToolManifest) -> ToolReviewResult {
        // 转换为 reviewer 的 ToolManifest（最小子集）
        let review_manifest = ReviewManifest {
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            description: manifest.description.clone(),
            parameters_schema: manifest.parameters_schema.clone(),
            risk_level: manifest.ci144.pfp_risk_level.clone(),
            platform: "generic".to_string(), // MCP 学习的工具通常跨平台，标记为 generic
            host_os: Vec::new(),          // 空列表表示支持所有平台
            tags: manifest.tags.clone(),
        };

        let report = self.reviewer.review(&review_manifest);

        // 状态迁移决策
        let state = if report.errors().len() > 0 {
            ToolState::Rejected
        } else if self.config.warning_to_staging && report.warnings().len() > 0 {
            ToolState::Staging
        } else {
            ToolState::Stable
        };

        let output_path = PathBuf::from(state.dir_name()).join(format!("{}.manifest.json", manifest.name));

        ToolReviewResult {
            tool_name: manifest.name.clone(),
            tool_version: manifest.version.clone(),
            report,
            state,
            output_path,
        }
    }

    /// 批量审查工具 Manifest 列表
    pub fn review_batch(
        &self,
        server_name: &str,
        manifests: &[ToolManifest],
    ) -> BatchReviewResult {
        let results: Vec<ToolReviewResult> = manifests
            .iter()
            .map(|m| self.review_tool(m))
            .collect();

        let mut state_counts = HashMap::new();
        for result in &results {
            let key = result.state.to_string();
            *state_counts.entry(key).or_insert(0) += 1;
        }

        BatchReviewResult {
            server_name: server_name.to_string(),
            reviewed_at: now_unix(),
            rules_version: RULES_VERSION.to_string(),
            state_counts,
            results,
        }
    }

    /// 审查并写入状态目录（完整管道）
    pub fn process_and_write(
        &self,
        server_name: &str,
        manifests: &[ToolManifest],
    ) -> Result<BatchReviewResult, std::io::Error> {
        // 1. 确保 MCP 代理执行体存在，并计算其真实哈希
        let proxy_hash = self.ensure_mcp_proxy_exists()?;

        // 2. 更新所有 manifest 的 integrity.hash 为真实哈希
        let mut manifests = manifests.to_vec();
        for manifest in &mut manifests {
            manifest.integrity.hash = proxy_hash.clone();
        }

        // 3. 批量审查
        let batch_result = self.review_batch(server_name, &manifests);

        // 4. 创建状态目录
        for state in [ToolState::Stable, ToolState::Staging, ToolState::Rejected] {
            let dir = self.config.output_root.join(state.dir_name());
            std::fs::create_dir_all(&dir)?;
        }

        // 5. 将每个工具写入对应状态目录
        for (i, result) in batch_result.results.iter().enumerate() {
            let manifest = &manifests[i];
            let full_path = self.config.output_root.join(&result.output_path);

            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let json = serde_json::to_string_pretty(manifest)?;
            std::fs::write(&full_path, json)?;

            tracing::info!(
                "工具 {} → {} (errors: {}, warnings: {})",
                result.tool_name,
                result.state,
                result.report.errors().len(),
                result.report.warnings().len()
            );
        }

        // 6. 生成审查报告
        if self.config.generate_report {
            let report_path = self
                .config
                .output_root
                .join(format!("{}_review_report.json", server_name));
            let report_json = serde_json::to_string_pretty(&batch_result)?;
            std::fs::write(&report_path, report_json)?;
        }

        Ok(batch_result)
    }

    /// 确保 MCP 代理执行体存在于所有状态目录中，并返回其 SHA-256 哈希
    ///
    /// Tentacle 扫描插件时会检查 executable 文件的 SHA-256 哈希。
    /// 因此需要计算 mcp_proxy.js 的真实哈希并填入 manifest 的 integrity.hash。
    fn ensure_mcp_proxy_exists(&self) -> Result<String, std::io::Error> {
        let proxy_content = r#"#!/usr/bin/env node
/**
 * MCP Proxy Executor — Helix-MCP-Learner 通用 MCP 代理执行体
 *
 * 用法：node mcp_proxy.js <tool_name> <params_json> <server_config_json>
 *
 * 这个脚本是 Tentacle 插件执行体的占位实现。
 * 实际生产环境中，应使用 Rust 实现的 tentacle-transport-mcp crate。
 */
const [,, toolName, paramsJson, serverConfigJson] = process.argv;

try {
    const params = JSON.parse(paramsJson || '{}');
    const serverConfig = JSON.parse(serverConfigJson || '{}');

    // 占位实现：输出工具调用信息
    const result = {
        ok: true,
        data: {
            tool: toolName,
            params,
            serverConfig,
            message: "MCP Proxy placeholder — replace with tentacle-transport-mcp in production"
        }
    };
    console.log(JSON.stringify(result));
} catch (e) {
    console.error(JSON.stringify({ ok: false, error: e.message }));
    process.exit(1);
}
"#;

        // 计算代理执行体的 SHA-256 哈希
        let proxy_hash = sha256_hex(proxy_content.as_bytes());

        for state in [ToolState::Stable, ToolState::Staging, ToolState::Rejected] {
            let proxy_path = self.config.output_root.join(state.dir_name()).join("mcp_proxy.js");
            if !proxy_path.exists() {
                if let Some(parent) = proxy_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&proxy_path, proxy_content)?;
                tracing::info!("创建 MCP 代理执行体: {}", proxy_path.display());
            }
        }

        Ok(proxy_hash)
    }

    /// 获取配置引用
    pub fn config(&self) -> &ReviewPipelineConfig {
        &self.config
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

fn now_unix() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", now.as_secs())
}

/// 计算数据的 SHA-256 哈希，返回十六进制字符串
fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    format!("{:x}", result)
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ci144::{Ci144Tool, RiskLevel};
    use crate::manifest::{Ci144Metadata, Integrity, Permission, SecurityLevel};
    use tempfile::tempdir;

    fn valid_manifest() -> ToolManifest {
        ToolManifest {
            name: "mcp.filesystem.read_file".to_string(),
            version: "1.0.0".to_string(),
            description: "Read a file from the filesystem and return its content as a string".to_string(),
            tags: vec!["mcp".to_string(), "filesystem".to_string()],
            executable: "mcp_proxy".to_string(),
            integrity: Integrity {
                algorithm: "sha256".to_string(),
                hash: "abc123".to_string(),
            },
            parameters_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "File path to read"}
                },
                "required": ["path"]
            }),
            security_level: SecurityLevel::Normal,
            permissions: Permission::default(),
            ci144: Ci144Metadata {
                mcp_name: "read_file".to_string(),
                pfp_risk_level: "low".to_string(),
                pfp_modality: "cognitive".to_string(),
                requires_confirmation: false,
                mcp_server: Default::default(),
            },
            timeout_ms: 30000,
        }
    }

    fn invalid_manifest() -> ToolManifest {
        let mut m = valid_manifest();
        m.name = String::new(); // R001: 名称为空 → Error
        m.description = String::new(); // R003: 描述为空 → Error
        m
    }

    fn warning_manifest() -> ToolManifest {
        let mut m = valid_manifest();
        m.name = "readfile".to_string(); // R009: 不符合命名空间 → Warning
        m.version = "v1".to_string(); // R002: 不符合 SemVer → Warning
        m
    }

    #[test]
    fn test_valid_manifest_goes_to_stable() {
        let pipeline = ReviewPipeline::with_output_root("/tmp/test_review");
        let manifest = valid_manifest();
        let result = pipeline.review_tool(&manifest);

        assert_eq!(result.state, ToolState::Stable);
        assert!(result.passed());
        assert!(result.report.errors().is_empty());
    }

    #[test]
    fn test_invalid_manifest_goes_to_rejected() {
        let pipeline = ReviewPipeline::with_output_root("/tmp/test_review");
        let manifest = invalid_manifest();
        let result = pipeline.review_tool(&manifest);

        assert_eq!(result.state, ToolState::Rejected);
        assert!(!result.passed());
        assert!(!result.report.errors().is_empty());
    }

    #[test]
    fn test_warning_manifest_goes_to_staging() {
        let pipeline = ReviewPipeline::with_output_root("/tmp/test_review");
        let manifest = warning_manifest();
        let result = pipeline.review_tool(&manifest);

        assert_eq!(result.state, ToolState::Staging);
        assert!(result.passed());
        assert!(result.report.errors().is_empty());
        assert!(!result.report.warnings().is_empty());
    }

    #[test]
    fn test_warning_can_go_to_stable_if_configured() {
        let config = ReviewPipelineConfig {
            output_root: PathBuf::from("/tmp/test_review"),
            warning_to_staging: false, // Warning 也进 stable
            generate_report: false,
        };
        let pipeline = ReviewPipeline::new(config);
        let manifest = warning_manifest();
        let result = pipeline.review_tool(&manifest);

        assert_eq!(result.state, ToolState::Stable);
    }

    #[test]
    fn test_batch_review() {
        let pipeline = ReviewPipeline::with_output_root("/tmp/test_review");
        let manifests = vec![valid_manifest(), invalid_manifest(), warning_manifest()];
        let result = pipeline.review_batch("test_server", &manifests);

        assert_eq!(result.results.len(), 3);
        assert_eq!(result.state_counts.get("stable"), Some(&1));
        assert_eq!(result.state_counts.get("rejected"), Some(&1));
        assert_eq!(result.state_counts.get("staging"), Some(&1));
        assert!((result.pass_rate() - 2.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn test_process_and_write_creates_directories() {
        let dir = tempdir().unwrap();
        let pipeline = ReviewPipeline::with_output_root(dir.path());
        let manifests = vec![valid_manifest(), invalid_manifest(), warning_manifest()];

        let result = pipeline.process_and_write("test_server", &manifests).unwrap();

        // 验证目录创建
        assert!(dir.path().join("stable").exists());
        assert!(dir.path().join("staging").exists());
        assert!(dir.path().join("rejected").exists());

        // 验证文件写入
        assert!(dir.path().join("stable/mcp.filesystem.read_file.manifest.json").exists());
        assert!(dir.path().join("staging/readfile.manifest.json").exists());
        // rejected 的工具名为空，文件名是 .json，检查 rejected 目录非空
        assert!(dir.path().join("rejected").read_dir().unwrap().count() >= 1);

        // 验证审查报告生成
        assert!(dir.path().join("test_server_review_report.json").exists());

        assert_eq!(result.results.len(), 3);
    }

    #[test]
    fn test_tool_state_dir_names() {
        assert_eq!(ToolState::Raw.dir_name(), "raw");
        assert_eq!(ToolState::Stable.dir_name(), "stable");
        assert_eq!(ToolState::Staging.dir_name(), "staging");
        assert_eq!(ToolState::Rejected.dir_name(), "rejected");
    }

    #[test]
    fn test_batch_result_tools_by_state() {
        let pipeline = ReviewPipeline::with_output_root("/tmp/test_review");
        let manifests = vec![valid_manifest(), invalid_manifest(), warning_manifest()];
        let result = pipeline.review_batch("test_server", &manifests);

        assert_eq!(result.tools_by_state(ToolState::Stable).len(), 1);
        assert_eq!(result.tools_by_state(ToolState::Rejected).len(), 1);
        assert_eq!(result.tools_by_state(ToolState::Staging).len(), 1);
    }
}
