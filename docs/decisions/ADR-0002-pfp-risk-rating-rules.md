# ADR-0002: PFP 风险评级规则（工具名模式匹配）

- **状态**: Accepted
- **日期**: 2026-08-30
- **决策者**: Jasonmilk
- **关联**: CI-144 v2.0 PFP-xCF14 §Risk-Level

## 背景

MCP-Learner 学习 MCP Server 的工具时，需要自动为每个工具分配 PFP Risk-Level，以便 Tuck 安全闸门进行决策。

### 候选方案

| 方案 | 描述 | 准确性 | 可解释性 | 性能 |
|---|---|---|---|---|
| A. 工具名模式匹配 | 基于工具名前缀/后缀匹配规则 | 中等 | ✅ 高 | ✅ 极快（~1.3μs） |
| B. LLM 语义分析 | 调用大模型分析工具描述 | 高 | ❌ 低 | ❌ 慢（秒级） |
| C. 人工标注 | 每个工具手动标注风险等级 | 高 | ✅ 高 | ❌ 不可扩展 |

## 决策

**选择方案 A（工具名模式匹配）作为 P1 自动评级机制。**

## 评级规则

| 工具名模式 | Risk-Level | 例子 |
|---|---|---|
| `read_*` / `list_*` / `get_*` / `search_*` / `query_*` | LOW | read_file, list_issues, get_user |
| `create_*` / `update_*` / `write_*` / `send_*` / `post_*` / `put_*` / `patch_*` / `edit_*` / `modify_*` | MEDIUM | create_file, update_file, send_message |
| `delete_*` / `remove_*` / `execute_*` / `run_*` / `drop_*` / `truncate_*` | CRITICAL | delete_file, remove_file, execute_command |
| `*_all` / `*_system` / `*_admin` / `*_root` / `delete_all` / `system_update` / `admin_override` | CATASTROPHIC | delete_all, system_update, admin_override |

### 优先级

CATASTROPHIC 模式优先匹配，然后是 CRITICAL、MEDIUM，最后默认 LOW。

### 安全约束

- CRITICAL/CATASTROPHIC 级别的工具学习后，需要人工确认才能启用
- 学习结果直接生成 Tuck 策略规则，工具调用前自动过安全闸门

## 理由

### 1. 确定性优先（phyt-DNA 哲学）

工具名模式匹配是确定性的：
- 相同工具名 → 相同风险评级
- 可测试、可复现、可审计
- 不依赖外部服务（LLM）

### 2. 极致节能

模式匹配性能极快（~1.3μs/次）：
- 无网络调用
- 无 LLM 推理
- 纯字符串匹配

### 3. 可解释性

规则清晰透明：
- 开发者可以理解为什么某个工具被评为 CRITICAL
- 可以手动覆盖评级（未来版本）
- 审计日志可以记录评级依据

### 4. 物理事实优先

风险评级基于工具的**实际操作类型**（读取/写入/删除/执行），而不是 AI 语义推理。

## 后果

### 正面

- ✅ 确定性、可复现、可审计
- ✅ 极致节能（~1.3μs/次）
- ✅ 可解释、可覆盖
- ✅ 基于物理事实（操作类型）

### 负面

- ⚠️ 非标准命名的工具可能被误判（如 `fetch_data` 会被评为 LOW，但实际可能是写入操作）
- ⚠️ 需要人工确认 CRITICAL/CATASTROPHIC 级别工具

### 演进路径

P2 阶段可以考虑：
- 支持人工覆盖评级（配置文件）
- 结合工具描述进行二次确认
- 学习用户的修正行为，优化评级规则

## 参考

- CI-144 v2.0 PFP-xCF14 §Risk-Level
- Tuck 安全闸门策略规则
- phyt-DNA 方法论 v1.0 §确定性优先 §物理事实优先
