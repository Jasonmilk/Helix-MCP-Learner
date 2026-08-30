# Helix-MCP-Learner 生长记录

> **版本**：v1.1
> **日期**：2026-08-30
> **所属方法论**：phyt-DNA 方法论 v1.0
> **规则**：仅保留最近 3 条记录，超则归档至 `docs/archive/growth/`（已版本化，永不删除）

---

## [2026-08-30] P1 完成 — MCP-Learner 最小验证通过，17 测试全绿

### 触发条件
P1 五个任务（T1-T5）全部完成，集成测试和性能测试通过，用户授权启动方向 A（ECO-Glove / MCP-Learner）。

### 变更性质
- **T1 MCP Client 基础层**：stdio 传输 + JSON-RPC 2.0 协议 + initialize + tools/list + tools/call
- **T2 CI-144 工具提炼层**：MCP 工具 → CIN7 意图 + CAPABILITY-13 参数 schema + PFP Risk-Level 自动评级
- **T3 Manifest 生成器**：学习结果 → Tentacle 可加载的插件 Manifest（含 CI-144 扩展元数据）
- **T4 端到端验证**：mock MCP Server（Python，4 工具）→ 学习 → 生成 4 Manifest + 1 索引 → 验证内容
- **T5 效率对比 + 确定性验证**：性能测试 + 确定性测试 + 文档完善

### 关键成果
- **测试**：17 个全绿（14 单元 + 3 集成）
- **性能**：学习 4 工具 ~107ms（一次性），MCP 调用 ~239μs，Manifest 加载 ~131μs，风险评级 ~1.3μs
- **CI-144 重封装开销**：<1%（Manifest 加载 vs MCP 调用）
- **风险评级规则**：read/list/get/search=LOW，create/update/write/send=MEDIUM，delete/remove/execute/run=CRITICAL，*_all/*_system/*_admin=CATASTROPHIC
- **确定性验证**：12 个工具名 × 3 次评级，结果完全一致

### 兼容性
- 生成的 Manifest 符合 Tentacle 格式（name/version/executable/integrity/parameters_schema/security_level）
- MCP 协议兼容 JSON-RPC 2.0 规范
- CI-144 元数据扩展（ci144 字段）不影响 Tentacle 解析

### 验收
- `cargo test --workspace` 全绿：17 passed
- 端到端测试：mock MCP Server → 学习 → 生成 → 验证
- 性能测试：CI-144 重封装开销 <1%
- 确定性测试：相同输入相同输出

### 状态
🧬 P1 已完成，P2 预览中（OS Glove + 多 MCP Server + MCP 代理执行体）

---

## [2026-08-30] 项目初始化 — phyt-DNA 方法论骨架 + P1 规划

### 触发条件
Helix 生态 6 个核心项目全部完成（1060+ tests），ECO-Glove 愿景文档（helix-eco-glove-vision.md）明确启动条件已满足（Cellrix P1 完成），用户授权启动方向 A：MCP-Learner 最小验证。

### 变更性质
- **项目初始化**：创建 Helix-MCP-Learner 独立项目，Rust 语言，MIT/Apache 2.0 双协议
- **方法论骨架**：DNA.md（6 条不可变原则）、RNA.md（三层加载协议 + 7 条 AI 协作铁律）、PLAN.md（P1 导航牌 + 5 个任务拆分）、GROWTH.md（本记录）
- **P1 规划**：T1 MCP Client 基础层 → T2 CI-144 工具提炼 → T3 Manifest 生成器 → T4 端到端验证 → T5 效率对比 + 确定性验证
- **关键决策**：先实现 stdio 传输（本地 MCP Server），风险评级用工具名模式匹配，学习结果先输出 JSON Manifest

### 兼容性
新项目，零历史代码；与 Helix 生态通过 CI-144 工具定义（Manifest）解耦。

### 验收
- DNA/RNA/PLAN/GROWTH 四文档建立
- P1 任务拆分清晰（T1-T5）
- 关键决策点列出（D1-D4）
- 下一阶段预览（P2 OS Glove + 多 MCP Server）

### 状态
🧬 已完成，P1 T1 待启动

---
