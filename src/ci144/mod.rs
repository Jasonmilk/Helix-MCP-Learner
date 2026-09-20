//! CI-144 工具提炼层
//!
//! 将 MCP 工具定义提炼为 CI-144 标准工具定义：
//! - CIN7 意图（工具名映射）
//! - CAPABILITY-13 能力声明（参数 schema 转换）
//! - PFP 风险评级（**声明优先**，名称关键词规则仅为被标记的 fallback）
//!
//! # 未知必须有表示
//!
//! 派生值绝不用默认值承载未知：无证据时返回 `RiskLevel::Unknown` 并附
//! `FieldProvenance`（来源 + 信任度），而不是伪装成 `RiskLevel::Low`。

use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::mcp::{Tool, ToolAnnotations};

/// PFP Risk-Level（与 CI-144 v2.0 PFP-xCF14 对齐）
///
/// **Append-Only**：判别值一旦发布就不可变。
/// `Unknown` 追加在末尾（`= 4`），既有变体绝不重编号或重排序。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    Low = 0,
    Medium = 1,
    Critical = 2,
    Catastrophic = 3,
    /// 未知：既无 server 声明证据，名称关键词规则也未命中。
    ///
    /// 它**不是** `Low`：不得映射为 `SecurityLevel::Normal`，
    /// 不得获得任何能力（见 `crate::manifest`）。
    Unknown = 4,
}

impl RiskLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            RiskLevel::Low => "LOW",
            RiskLevel::Medium => "MEDIUM",
            RiskLevel::Critical => "CRITICAL",
            RiskLevel::Catastrophic => "CATASTROPHIC",
            RiskLevel::Unknown => "UNKNOWN",
        }
    }

    /// 是否需要人工确认（**单一事实来源**：提炼层与 Manifest 层共用本条规则）
    ///
    /// `Unknown` 也必须人工确认：无证据 ⇒ 不得默认放行。
    pub fn requires_confirmation(&self) -> bool {
        matches!(
            self,
            RiskLevel::Critical | RiskLevel::Catastrophic | RiskLevel::Unknown
        )
    }
}

/// 派生值的来源类别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProvenanceOrigin {
    /// 来自 server 的显式声明（当前唯一通道：MCP `ToolAnnotations`）
    Declared,
    /// 由本地规则推断（命中的规则见 `FieldProvenance.rule`）
    Inferred,
    /// 无任何证据
    Unknown,
}

/// 字段级溯源：一个派生值**从哪来** + **有多可信**
///
/// "声明"不等于"可信"：MCP 规范要求客户端把 `ToolAnnotations` 视为不可信，
/// 除非它来自受信任的 server。因此来源（`source`）与信任度（`trusted`）分开记录。
///
/// 有了它，`RiskLevel::Unknown` 与"关键词猜出来的 `RiskLevel::Low`"
/// 在**数据上**可区分，而不只是靠注释说明。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldProvenance {
    /// 来源类别
    pub origin: ProvenanceOrigin,
    /// 命中的推断规则 id（仅 `origin == Inferred` 时存在）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
    /// 证据来源标识（哪条声明通道 / 哪张规则表 + 哪个 server）
    pub source: String,
    /// 该来源是否受信任
    pub trusted: bool,
}

impl FieldProvenance {
    /// 无证据
    pub fn unknown() -> Self {
        Self {
            origin: ProvenanceOrigin::Unknown,
            rule: None,
            source: "none".to_string(),
            trusted: false,
        }
    }

    /// 来自声明（`trusted` 由调用方按来源判定；MCP annotations 默认 `false`）
    pub fn declared(source: impl Into<String>, trusted: bool) -> Self {
        Self {
            origin: ProvenanceOrigin::Declared,
            rule: None,
            source: source.into(),
            trusted,
        }
    }

    /// 来自本地规则推断
    pub fn inferred(rule: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            origin: ProvenanceOrigin::Inferred,
            rule: Some(rule.into()),
            source: source.into(),
            trusted: false,
        }
    }
}

impl Default for FieldProvenance {
    /// 缺省即"未知"，绝不缺省为"已知"
    fn default() -> Self {
        Self::unknown()
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
    /// PFP Risk-Level（声明优先评级）
    pub risk_level: RiskLevel,
    /// PFP Modality（EXECUTIVE，因为是工具执行）
    pub modality: String,
    /// 是否需要人工确认（CRITICAL/CATASTROPHIC/UNKNOWN）
    pub requires_confirmation: bool,
    /// 风险评级的来源与信任度。
    ///
    /// `#[serde(default)]` 保证旧的持久化版本（`LearningVersion`）仍可解析；
    /// 旧数据默认落到 `Unknown` —— 这比默认成 `Low` 诚实。
    /// 该字段**总是被序列化**：未知必须有表示，不能被省略掉。
    #[serde(default)]
    pub risk_provenance: FieldProvenance,
    /// server 声明的原始注解（证据本体，原样保留以便随时复核）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_annotations: Option<ToolAnnotations>,
}

/// 名称关键词的匹配方式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NameMatchKind {
    /// 以该片段开头
    Prefix,
    /// 以该片段结尾
    Suffix,
    /// 包含该片段（子串）
    Contains,
    /// 包含该片段，且该片段是一个完整的 `_`/`-`/`.`/空格 分隔 token
    ///
    /// 比 `Contains` 更精确：`macos_read_file` 命中 `read`，而
    /// `thread_info` **不会**命中 `read`（token 是 `thread`）。
    Token,
}

/// 名称关键词规则表（**推断 fallback，不是事实来源**）
///
/// 表序即优先级，第一条命中即返回。
/// 命中规则 id 会写入 `FieldProvenance.rule`，因此"关键词猜出来的等级"
/// 与"server 声明出来的等级"在数据上可区分。
///
/// 这是唯一的关键词事实来源（0 硬编码散布）：新增/调整启发式只改这张表。
const NAME_KEYWORD_RULES: &[(&str, NameMatchKind, RiskLevel, &str)] = &[
    // ---- CATASTROPHIC（最高优先级）----
    ("_all", NameMatchKind::Suffix, RiskLevel::Catastrophic, "name.suffix._all"),
    ("_system", NameMatchKind::Suffix, RiskLevel::Catastrophic, "name.suffix._system"),
    ("_admin", NameMatchKind::Suffix, RiskLevel::Catastrophic, "name.suffix._admin"),
    ("_root", NameMatchKind::Suffix, RiskLevel::Catastrophic, "name.suffix._root"),
    ("delete_all", NameMatchKind::Contains, RiskLevel::Catastrophic, "name.contains.delete_all"),
    ("system_update", NameMatchKind::Contains, RiskLevel::Catastrophic, "name.contains.system_update"),
    ("admin_override", NameMatchKind::Contains, RiskLevel::Catastrophic, "name.contains.admin_override"),
    // ---- CRITICAL ----
    ("delete_", NameMatchKind::Prefix, RiskLevel::Critical, "name.prefix.delete_"),
    ("remove_", NameMatchKind::Prefix, RiskLevel::Critical, "name.prefix.remove_"),
    ("execute_", NameMatchKind::Prefix, RiskLevel::Critical, "name.prefix.execute_"),
    ("run_", NameMatchKind::Prefix, RiskLevel::Critical, "name.prefix.run_"),
    ("drop_", NameMatchKind::Prefix, RiskLevel::Critical, "name.prefix.drop_"),
    ("truncate_", NameMatchKind::Prefix, RiskLevel::Critical, "name.prefix.truncate_"),
    ("delete", NameMatchKind::Contains, RiskLevel::Critical, "name.contains.delete"),
    ("remove", NameMatchKind::Contains, RiskLevel::Critical, "name.contains.remove"),
    ("execute", NameMatchKind::Contains, RiskLevel::Critical, "name.contains.execute"),
    // ---- MEDIUM ----
    ("create_", NameMatchKind::Prefix, RiskLevel::Medium, "name.prefix.create_"),
    ("update_", NameMatchKind::Prefix, RiskLevel::Medium, "name.prefix.update_"),
    ("write_", NameMatchKind::Prefix, RiskLevel::Medium, "name.prefix.write_"),
    ("send_", NameMatchKind::Prefix, RiskLevel::Medium, "name.prefix.send_"),
    ("post_", NameMatchKind::Prefix, RiskLevel::Medium, "name.prefix.post_"),
    ("put_", NameMatchKind::Prefix, RiskLevel::Medium, "name.prefix.put_"),
    ("patch_", NameMatchKind::Prefix, RiskLevel::Medium, "name.prefix.patch_"),
    ("edit_", NameMatchKind::Prefix, RiskLevel::Medium, "name.prefix.edit_"),
    ("modify_", NameMatchKind::Prefix, RiskLevel::Medium, "name.prefix.modify_"),
    ("create", NameMatchKind::Contains, RiskLevel::Medium, "name.contains.create"),
    ("update", NameMatchKind::Contains, RiskLevel::Medium, "name.contains.update"),
    ("write", NameMatchKind::Contains, RiskLevel::Medium, "name.contains.write"),
    ("send", NameMatchKind::Contains, RiskLevel::Medium, "name.contains.send"),
    // ---- LOW（**显式**规则）----
    // 旧实现里 LOW 是"什么都没命中"的默认分支，也就是本缺陷的安全倒置。
    // 现在 LOW 必须像其他等级一样被显式命中，否则结果就是 Unknown。
    ("read", NameMatchKind::Token, RiskLevel::Low, "name.token.read"),
    ("list", NameMatchKind::Token, RiskLevel::Low, "name.token.list"),
    ("get", NameMatchKind::Token, RiskLevel::Low, "name.token.get"),
    ("search", NameMatchKind::Token, RiskLevel::Low, "name.token.search"),
];

/// 判断名称是否命中某条关键词规则
fn name_matches(name: &str, kind: NameMatchKind, needle: &str) -> bool {
    match kind {
        NameMatchKind::Prefix => name.starts_with(needle),
        NameMatchKind::Suffix => name.ends_with(needle),
        NameMatchKind::Contains => name.contains(needle),
        NameMatchKind::Token => name
            .split(['_', '-', '.', ' '])
            .any(|token| token == needle),
    }
}

/// 名称关键词规则匹配：返回 `(等级, 规则 id)`；`None` 表示**没有任何规则命中**。
fn risk_rating_by_name(tool_name: &str) -> Option<(RiskLevel, &'static str)> {
    let name = tool_name.to_lowercase();
    NAME_KEYWORD_RULES
        .iter()
        .find(|(needle, kind, _, _)| name_matches(&name, *kind, needle))
        .map(|(_, _, risk, rule)| (*risk, *rule))
}

/// 名称关键词风险评级（**推断 fallback，不是事实来源**）
///
/// 命中规则表则返回该等级；**无命中返回 `RiskLevel::Unknown`**（绝不是 `Low`）。
///
/// 本函数只做名称推断，无法表达"这是猜的"；需要溯源信息时请用
/// [`assess_risk`]（它同时返回 `FieldProvenance`）。
pub fn risk_rating(tool_name: &str) -> RiskLevel {
    risk_rating_by_name(tool_name)
        .map(|(risk, _)| risk)
        .unwrap_or(RiskLevel::Unknown)
}

/// 构造带 server 标识的证据来源串
fn evidence_source(channel: &str, server_id: &str) -> String {
    if server_id.is_empty() {
        channel.to_string()
    } else {
        format!("{}#{}", server_id, channel)
    }
}

/// 声明优先的风险评级：返回 `(等级, 溯源)`
///
/// 判定顺序（确定性优先，声明优先于猜测）：
/// 1. `annotations.destructiveHint == Some(true)` ⇒ **至少** `Critical`
///    （下限语义：名称规则表明 `Catastrophic` 时取 `Catastrophic`，绝不降级）
/// 2. `annotations.readOnlyHint == Some(true)` ⇒ `Low`
/// 3. 名称关键词规则 ⇒ 命中等级，且溯源标记为 `Inferred(rule)`
/// 4. 以上皆无 ⇒ `Unknown`（不是 `Low`）
///
/// `destructiveHint` 优先于 `readOnlyHint`：两者同时为 `true` 是自相矛盾的声明，
/// 取更保守的一方。
///
/// 注意 `readOnlyHint` 按规范是**直接**给出 `Low`（规范只对 `destructiveHint`
/// 写了 "at least"）。因此 `delete_all` + `readOnlyHint: true` 会得到 `Low`；
/// 该处不对称是规范给定的，未被本实现擅自"改进"。由于此时
/// `FieldProvenance.trusted == false`，下游可以据此拒绝这条不可信声明。
///
/// `openWorldHint` / `idempotentHint` 只作为参考证据被保留（见
/// `Ci144Tool::mcp_annotations`），**不单独决定**等级：它们既不构成
/// "低风险"的证据，也不足以单独推断出高风险。
///
/// 所有来自 `annotations` 的结论 `trusted == false`，因为 MCP 规范要求
/// 客户端把 tool annotations 视为不可信，除非 server 受信任。
pub fn assess_risk(tool: &Tool, server_id: &str) -> (RiskLevel, FieldProvenance) {
    let annotations = tool.annotations.as_ref();

    // 1) 声明：破坏性 ⇒ **至少** CRITICAL
    //
    // "至少" 是关键：声明是**下限**，不是上限。若名称规则已表明更高的
    // CATASTROPHIC，绝不能因为一条声明把它**降级**（降级＝新的安全倒置）。
    if annotations.and_then(|a| a.destructive_hint) == Some(true) {
        if let Some((RiskLevel::Catastrophic, rule)) = risk_rating_by_name(&tool.name) {
            // 最终等级由名称规则决定，所以如实标为 Inferred（证据本体仍在
            // `mcp_annotations` 里，声明保证了 ≥ CRITICAL）
            return (
                RiskLevel::Catastrophic,
                FieldProvenance::inferred(
                    rule,
                    evidence_source("tool.name_keyword_rules", server_id),
                ),
            );
        }
        return (
            RiskLevel::Critical,
            FieldProvenance::declared(
                evidence_source("tool.annotations.destructiveHint", server_id),
                false,
            ),
        );
    }

    // 2) 声明：只读 ⇒ LOW（注意：只有 Some(true) 才算证据，None ≠ false）
    if annotations.and_then(|a| a.read_only_hint) == Some(true) {
        return (
            RiskLevel::Low,
            FieldProvenance::declared(
                evidence_source("tool.annotations.readOnlyHint", server_id),
                false,
            ),
        );
    }

    // 3) fallback：名称关键词规则（标记为 Inferred）
    if let Some((risk, rule)) = risk_rating_by_name(&tool.name) {
        return (
            risk,
            FieldProvenance::inferred(
                rule,
                evidence_source("tool.name_keyword_rules", server_id),
            ),
        );
    }

    // 4) 无证据 ⇒ 未知
    (RiskLevel::Unknown, FieldProvenance::unknown())
}

/// 将 MCP 工具提炼为 CI-144 工具定义
///
/// 注意：此路径没有 server 标识，`FieldProvenance.source` 只到通道级。
/// 需要可审计的 server 归属时请用 [`extract_tool_with_namespace`]。
pub fn extract_tool(mcp_tool: &Tool) -> Ci144Tool {
    extract_tool_inner(mcp_tool, "")
}

/// 提炼内核：`server_id` 进入证据来源，使"哪个 server 声明的"可追溯
fn extract_tool_inner(mcp_tool: &Tool, server_id: &str) -> Ci144Tool {
    let (risk, risk_provenance) = assess_risk(mcp_tool, server_id);

    Ci144Tool {
        intent_name: mcp_tool.name.clone(),
        mcp_name: mcp_tool.name.clone(),
        description: mcp_tool.description.clone(),
        parameters: serde_json::to_value(&mcp_tool.inputSchema).unwrap_or(Value::Null),
        risk_level: risk,
        modality: "EXECUTIVE".to_string(),
        requires_confirmation: risk.requires_confirmation(),
        risk_provenance,
        // 原始声明原样保留：派生结论永远可以回到证据复核
        mcp_annotations: mcp_tool.annotations.clone(),
    }
}

/// 将 MCP 工具提炼为 CI-144 工具定义（带命名空间前缀）
///
/// 生成符合 Helix 点分命名空间规范的工具名：`<namespace>.<original_name>`
/// 例如：namespace="mock-filesystem", original="read_file" → "mock-filesystem.read_file"
///
/// 这确保 MCP-Learner 学习的工具能通过 L1 审查的 R009（点分命名空间）规则，
/// 直接进入 stable/ 而不是 staging/。
///
/// `namespace` 同时作为证据来源中的 server 标识。
pub fn extract_tool_with_namespace(mcp_tool: &Tool, namespace: &str) -> Ci144Tool {
    let mut tool = extract_tool_inner(mcp_tool, namespace);
    // 将 namespace 中的非字母数字字符替换为下划线，确保工具名合法
    let safe_namespace: String = namespace
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    tool.intent_name = format!("{}.{}", safe_namespace, mcp_tool.name);
    tool
}

/// 批量提炼工具
pub fn extract_tools(mcp_tools: &[Tool]) -> Vec<Ci144Tool> {
    mcp_tools.iter().map(extract_tool).collect()
}

/// 批量提炼工具（带命名空间前缀）
pub fn extract_tools_with_namespace(mcp_tools: &[Tool], namespace: &str) -> Vec<Ci144Tool> {
    mcp_tools.iter().map(|t| extract_tool_with_namespace(t, namespace)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::{ToolAnnotations, ToolInputSchema};

    fn make_tool(name: &str) -> Tool {
        Tool {
            name: name.to_string(),
            description: format!("Test tool: {}", name),
            title: None,
            annotations: None,
            inputSchema: ToolInputSchema::default(),
            output_schema: None,
        }
    }

    fn make_tool_with_annotations(name: &str, annotations: ToolAnnotations) -> Tool {
        Tool {
            annotations: Some(annotations),
            ..make_tool(name)
        }
    }

    #[test]
    fn test_risk_rating_low() {
        assert_eq!(risk_rating("read_file"), RiskLevel::Low);
        assert_eq!(risk_rating("list_files"), RiskLevel::Low);
        assert_eq!(risk_rating("get_user"), RiskLevel::Low);
        assert_eq!(risk_rating("search_issues"), RiskLevel::Low);
        // 带命名空间前缀的名字也能被读取类规则识别
        assert_eq!(risk_rating("macos_read_file"), RiskLevel::Low);
    }

    /// 回归：`query` 以前断言为 LOW，靠的正是"什么都没命中 ⇒ Low"这个缺陷默认值。
    /// 现在它必须如实报 Unknown。
    #[test]
    fn test_risk_rating_unknown_for_unmatched_name() {
        assert_eq!(risk_rating("query"), RiskLevel::Unknown);
        assert_eq!(risk_rating("zap"), RiskLevel::Unknown);
        assert_eq!(risk_rating("清理"), RiskLevel::Unknown);
        assert_eq!(risk_rating("rm_all_data"), RiskLevel::Unknown);
        // Token 匹配不会被 "thread" 里的 "read" 骗到
        assert_eq!(risk_rating("thread_info"), RiskLevel::Unknown);
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

    /// Append-Only：既有判别值永不变动，`Unknown` 追加在末尾
    #[test]
    fn test_risk_level_discriminants_are_append_only() {
        assert_eq!(RiskLevel::Low as u8, 0);
        assert_eq!(RiskLevel::Medium as u8, 1);
        assert_eq!(RiskLevel::Critical as u8, 2);
        assert_eq!(RiskLevel::Catastrophic as u8, 3);
        assert_eq!(RiskLevel::Unknown as u8, 4);

        // 线上表示同样 Append-Only：serde 用**变体名**序列化，
        // 既有变体的名字一旦发布就不可更改（改名等于破坏兼容）。
        assert_eq!(serde_json::to_string(&RiskLevel::Low).unwrap(), "\"Low\"");
        assert_eq!(serde_json::to_string(&RiskLevel::Medium).unwrap(), "\"Medium\"");
        assert_eq!(serde_json::to_string(&RiskLevel::Critical).unwrap(), "\"Critical\"");
        assert_eq!(
            serde_json::to_string(&RiskLevel::Catastrophic).unwrap(),
            "\"Catastrophic\""
        );
        assert_eq!(
            serde_json::to_string(&RiskLevel::Unknown).unwrap(),
            "\"Unknown\""
        );

        // 反序列化旧数据（只有 4 个变体）必须仍然成立
        let legacy: RiskLevel = serde_json::from_str("\"Catastrophic\"").unwrap();
        assert_eq!(legacy, RiskLevel::Catastrophic);
    }

    /// 缺陷回归：非常规名称（非英文/代号/缩写）不得被静默评为 Low
    #[test]
    fn test_unconventional_name_is_unknown_not_low() {
        for name in ["清理", "zap", "rm_all_data", "SFTP", "myProperNoun"] {
            let (risk, provenance) = assess_risk(&make_tool(name), "test-server");
            assert_eq!(risk, RiskLevel::Unknown, "{} 不得被默认评为 Low", name);
            assert_eq!(provenance.origin, ProvenanceOrigin::Unknown);
            assert!(!provenance.trusted);
            assert!(risk.requires_confirmation(), "Unknown 必须要求人工确认");

            let ci144 = extract_tool(&make_tool(name));
            assert_eq!(ci144.risk_level, RiskLevel::Unknown);
            assert!(ci144.requires_confirmation);
        }
    }

    /// 声明优先：`destructiveHint: Some(true)` 压过名称（哪怕名字看起来无害）
    #[test]
    fn test_annotation_destructive_hint_wins_over_name() {
        let tool = make_tool_with_annotations(
            "read_file",
            ToolAnnotations {
                destructive_hint: Some(true),
                ..Default::default()
            },
        );
        let (risk, provenance) = assess_risk(&tool, "test-server");

        assert_eq!(risk, RiskLevel::Critical);
        assert_eq!(provenance.origin, ProvenanceOrigin::Declared);
        assert!(provenance.source.contains("destructiveHint"));
        assert!(provenance.source.contains("test-server"));
        // MCP 规范：annotations 默认不可信
        assert!(!provenance.trusted);

        let ci144 = extract_tool(&tool);
        assert_eq!(ci144.risk_level, RiskLevel::Critical);
        assert!(ci144.requires_confirmation);
    }

    /// 非常规名称 + 破坏性声明 ⇒ 依然 CRITICAL（声明救得回来）
    #[test]
    fn test_annotation_rescues_unconventional_name() {
        let tool = make_tool_with_annotations(
            "清理",
            ToolAnnotations {
                destructive_hint: Some(true),
                ..Default::default()
            },
        );
        let ci144 = extract_tool(&tool);
        assert_eq!(ci144.risk_level, RiskLevel::Critical);
        assert!(ci144.requires_confirmation);
        assert_eq!(
            ci144.risk_provenance.origin,
            ProvenanceOrigin::Declared
        );
    }

    /// 声明优先：`readOnlyHint: Some(true)` ⇒ LOW（且溯源为 Declared，不是猜的）
    #[test]
    fn test_annotation_read_only_hint_gives_declared_low() {
        let tool = make_tool_with_annotations(
            "清理",
            ToolAnnotations {
                read_only_hint: Some(true),
                ..Default::default()
            },
        );
        let (risk, provenance) = assess_risk(&tool, "test-server");
        assert_eq!(risk, RiskLevel::Low);
        assert_eq!(provenance.origin, ProvenanceOrigin::Declared);
        assert!(provenance.source.contains("readOnlyHint"));
    }

    /// `destructiveHint` 是**下限**："at least Critical" 不得把名称已表明的
    /// CATASTROPHIC 降级为 CRITICAL
    #[test]
    fn test_destructive_hint_never_downgrades_catastrophic() {
        let tool = make_tool_with_annotations(
            "delete_all",
            ToolAnnotations {
                destructive_hint: Some(true),
                ..Default::default()
            },
        );
        let (risk, provenance) = assess_risk(&tool, "test-server");
        assert_eq!(
            risk,
            RiskLevel::Catastrophic,
            "声明是下限，绝不能把 delete_all 降级为 Critical"
        );
        // 最终等级由名称规则给出，如实标为 Inferred；声明本体仍随产物保留
        assert_eq!(provenance.origin, ProvenanceOrigin::Inferred);
        assert_eq!(provenance.rule.as_deref(), Some("name.suffix._all"));
        assert_eq!(
            extract_tool(&tool)
                .mcp_annotations
                .and_then(|a| a.destructive_hint),
            Some(true)
        );

        // 名称只表明 Critical 时，声明提供 Critical 下限（不放大）
        let tool = make_tool_with_annotations(
            "delete_file",
            ToolAnnotations {
                destructive_hint: Some(true),
                ..Default::default()
            },
        );
        assert_eq!(assess_risk(&tool, "test-server").0, RiskLevel::Critical);
    }

    /// 缺省 ≠ false：`readOnlyHint: None` 不是"低风险"的许可
    #[test]
    fn test_absent_read_only_hint_is_not_low_risk_permission() {
        for hint in [None, Some(false)] {
            let tool = make_tool_with_annotations(
                "zap",
                ToolAnnotations {
                    read_only_hint: hint,
                    title: Some("Declared title only".to_string()),
                    ..Default::default()
                },
            );
            let (risk, provenance) = assess_risk(&tool, "test-server");
            assert_eq!(
                risk,
                RiskLevel::Unknown,
                "readOnlyHint={:?} 不构成低风险证据",
                hint
            );
            assert_eq!(provenance.origin, ProvenanceOrigin::Unknown);
        }
    }

    /// `openWorldHint` / `idempotentHint` 只参考，不单独决定等级
    #[test]
    fn test_reference_hints_do_not_decide() {
        let tool = make_tool_with_annotations(
            "zap",
            ToolAnnotations {
                open_world_hint: Some(true),
                idempotent_hint: Some(true),
                ..Default::default()
            },
        );
        let (risk, _) = assess_risk(&tool, "test-server");
        assert_eq!(risk, RiskLevel::Unknown);

        // 但证据本体被保留下来，供下游复核
        assert!(extract_tool(&tool).mcp_annotations.is_some());
    }

    /// 关键词 fallback 必须被**标记**：Inferred + 规则 id，且与声明可区分
    #[test]
    fn test_keyword_fallback_is_marked_as_inferred() {
        let (risk, provenance) = assess_risk(&make_tool("read_file"), "test-server");
        assert_eq!(risk, RiskLevel::Low);
        assert_eq!(provenance.origin, ProvenanceOrigin::Inferred);
        assert_eq!(provenance.rule.as_deref(), Some("name.token.read"));
        assert!(!provenance.trusted);

        // 与"声明出来的 LOW"在数据上必须不同
        let declared = make_tool_with_annotations(
            "read_file",
            ToolAnnotations {
                read_only_hint: Some(true),
                ..Default::default()
            },
        );
        let (risk2, provenance2) = assess_risk(&declared, "test-server");
        assert_eq!(risk2, RiskLevel::Low);
        assert_ne!(
            provenance, provenance2,
            "推论出来的 LOW 与声明出来的 LOW 必须可区分"
        );
        assert_eq!(provenance2.origin, ProvenanceOrigin::Declared);
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

    /// server 归属进入证据来源（可审计"谁声明的"）
    #[test]
    fn test_namespace_qualified_provenance_source() {
        let tool = make_tool_with_annotations(
            "清理",
            ToolAnnotations {
                destructive_hint: Some(true),
                ..Default::default()
            },
        );
        let ci144 = extract_tool_with_namespace(&tool, "my-server");
        assert_eq!(ci144.intent_name, "my-server.清理");
        assert_eq!(
            ci144.risk_provenance.source,
            "my-server#tool.annotations.destructiveHint"
        );
    }

    /// 旧持久化数据（没有 risk_provenance 字段）必须仍可解析，且落到 Unknown
    #[test]
    fn test_legacy_ci144_tool_json_still_parses() {
        let legacy = r#"{
            "intent_name": "read_file",
            "mcp_name": "read_file",
            "description": "Read a file",
            "parameters": {"type": "object"},
            "risk_level": "Low",
            "modality": "EXECUTIVE",
            "requires_confirmation": false
        }"#;
        let parsed: Ci144Tool = serde_json::from_str(legacy).unwrap();
        assert_eq!(parsed.risk_level, RiskLevel::Low);
        assert_eq!(parsed.risk_provenance, FieldProvenance::unknown());
        assert!(parsed.mcp_annotations.is_none());
    }
}

