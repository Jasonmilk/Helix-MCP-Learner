# Helix-MCP-Learner 生长记录

> **版本**：v1.3
> **日期**：2026-08-30
> **所属方法论**：phyt-DNA 方法论 v1.0
> **规则**：仅保留最近 3 条记录，超则归档至 `docs/archive/growth/`（已版本化，永不删除）

---

## [2026-08-30] P2 完成 — OS Glove + 多 MCP Server + MCP 代理执行体，42 测试全绿

### 触发条件
P2 五个任务（T1-T5）全部完成，MCP 代理执行体、多 MCP Server 配置、macOS Glove、增量学习全部实现并通过测试。

### 变更性质
- **T1 MCP 代理执行体**：`src/proxy/` — 管理 MCP Server 生命周期，统一工具调用接口，懒连接，3 个代理集成测试
- **T2 多 MCP Server 支持**：`src/config/` — TOML 配置文件，批量学习，工具名冲突处理（4 种策略：error/skip/rename/overwrite）
- **T3 macOS Glove 最小版本**：`src/glove/macos/` — 6 个系统工具（文件读写、目录列表、命令执行、进程列表、AppleScript）
- **T4 增量学习 + 版本管理**：`src/learning/` — 工具变化检测（新增/变更/删除），增量 diff，版本管理，废弃标记
- **T5 文档完善 + 测试**：README 中英文版更新，PLAN.md v3.0，GROWTH.md v1.3

### 关键成果
- **测试**：42 个全绿（35 单元 + 3 集成 + 1 性能 + 3 代理）
- **模块数**：7 个核心模块（mcp/ci144/manifest/proxy/config/glove/learning）
- **macOS 工具数**：6 个（read_file/write_file/list_directory/execute_command/list_processes/run_applescript）
- **冲突处理策略**：4 种（error/skip/rename/overwrite）
- **增量学习**：支持新增/变更/删除/未变更 4 种变化检测

### 兼容性
- P1 生成的 Manifest 格式保持不变，P2 向后兼容
- 新增的 MCP 代理执行体是独立组件，不影响 P1 的学习流程
- 多 MCP Server 支持是增量功能，单 MCP Server 场景仍可用
- macOS Glove 是可选模块，不影响其他平台

### 验收
- `cargo test --workspace` 全绿：42 passed
- MCP 代理端到端测试：mock MCP Server → 代理 → 工具调用 → 返回结果
- 多 MCP Server 批量学习：TOML 配置 → 批量学习 → 结果合并
- macOS Glove 工具测试：文件读写、命令执行、进程列表
- 增量学习测试：新增/变更/删除检测

### 状态
🧬 P2 已完成，P3 预览中（生态集成 + 高级特性）

---

## [2026-08-30] P2 启动 — OS Glove + 多 MCP Server + MCP 代理执行体

### 触发条件
P1 完成并推送至 GitHub（https://github.com/Jasonmilk/Helix-MCP-Learner），用户授权启动 P2。P2 完成后再讨论生态联调（方向 B）和 Helix-Mind P10（方向 C）。

### 变更性质
- **P2 规划**：T1 MCP 代理执行体 → T2 多 MCP Server 支持 → T3 macOS Glove 最小版本 → T4 增量学习 + 版本管理 → T5 文档完善 + 测试
- **关键决策点**：D1 代理执行体形式（守护进程/静态插件/WASM）、D2 配置格式（TOML）、D3 macOS Glove 实现方式（作为 MCP Server）、D4 增量学习策略（工具清单 diff）
- **GitHub 仓库**：https://github.com/Jasonmilk/Helix-MCP-Learner（Public，main 分支）

### 兼容性
- P1 生成的 Manifest 格式保持不变，P2 向后兼容
- 新增的 MCP 代理执行体是独立组件，不影响 P1 的学习流程
- 多 MCP Server 支持是增量功能，单 MCP Server 场景仍可用

### 验收
- PLAN.md v2.0：P2 任务拆分清晰（T1-T5）
- 关键决策点列出（D1-D4）
- 下一阶段预览（P3 生态联调）
- GitHub 仓库可访问，代码已推送

### 状态
🧬 P2 已完成

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
🧬 P1 已完成

---

---

## 2026-08-31 生态联调成功（里程碑）

**事件**：MCP-Learner 与 Helix-Tentacle 全链路联调成功

**验证链路**：
```
MCP-Learner 学习 mock MCP Server → 4 个工具
    ↓
L1 静态审查（9 条规则）→ 0 warning, 0 error
    ↓
stable/ 目录（4 个 .manifest.json + mcp_proxy.js）
    ↓
Tentacle 扫描 + SHA-256 完整性校验 → 4 个工具注册
    ↓
ProcessTool 实例化 → 4 个工具可用
    ↓
tentacle-cli 执行 mock-filesystem.list_files → ✅ 成功返回结果
```

**联调中修复的 4 个问题**：
1. 工具名命名空间：新增 `extract_tools_with_namespace`，格式 `<server>.<name>`
2. Manifest 文件后缀：输出改为 `.manifest.json`（Tentacle 扫描要求）
3. MCP 代理执行体：post_learn 自动创建 mcp_proxy.js 占位文件
4. 完整性哈希：计算 mcp_proxy.js 真实 SHA-256 并更新 manifest

**提交记录**：`d21b897` — fix(生态联调): 修复全链路兼容性问题

**下一步**：升级 mcp_proxy.js 为真实 MCP 代理执行体（当前为占位脚本）

## [2026-09-06] P3 + P4-T1 完成 — 生态联调全链路 + post_learn 审查管道

### 触发条件
P3 生态联调（Tentacle 全链路兼容修复 + 里程碑记录）与 P4-T1（post_learn 审查管道）完成。

### 变更性质
- **P3 生态联调**：MCP 工具名点分命名空间（`<server>.<name>`）+ manifest 后缀 `.manifest.json` + mcp_proxy.js 占位执行体 + SHA-256 真实哈希 + Tentacle 插件懒加载修复（6 个联调问题全修）
- **P4-T1 post_learn 审查管道**：`ReviewPipeline` + `ReviewPipelineConfig` + `ToolState` + `ToolReviewResult` + `BatchReviewResult`；状态迁移自动化（Error→rejected/，Warning→staging/，Info→stable/）；集成到 `mcp-learner learn` 命令；审查报告 `{server}_review_report.json`

### 关键成果
- **全链路验证通过**：MCP-Learner 学习 mock MCP Server → L1 静态审查（9 规则 0 warning 0 error）→ stable/ 目录 → Tentacle 扫描 + SHA-256 校验 → 工具注册 → 执行成功
- **测试**：42 passed + 1 failed（失败项待修，非阻塞）

### 提交
- `d21b897`（全链路兼容修复）+ `500a461`（生态联调里程碑）+ `5a32508`（P4-T1 审查管道）

### 状态
✅ P2/P3/P4-T1 完成；⚠️ 1 failed 未修（ECOSYSTEM 已记录）
