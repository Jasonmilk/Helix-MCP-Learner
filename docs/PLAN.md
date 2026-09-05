# Helix-MCP-Learner 开发导航牌（PLAN）

> **版本**：v3.2（P3 生态联调 + P4-T1 审查管道完成，2026-09-06）
> **状态**：✅ P2/P3/P4-T1 完成 — 生态联调全链路畅通（Tentacle 集成）+ post_learn 审查管道；⚠️ 1 个失败测试未修
> **分支**：main
> **所属方法论**：phyt-DNA 方法论 v1.0（方法论锚点项目 https://github.com/Jasonmilk/phyt-DNA）
> **规则**：本文件只含当前阶段 + 下一阶段预览 + 阶段总览地图。完成阶段 → GROWTH.md。总行数 ≤150，超出触发历史迁移。

---

## 1. 当前阶段：P3 预览（待启动）

> **状态**：⏳ 待启动。
> **目标**：生态集成 + 高级特性。与 Tentacle/Anaphase/Mind 联调，验证端到端工具学习→执行闭环。
> **前置依赖**：P2 完成（MCP 代理 + 多 Server + macOS Glove + 增量学习，42 测试全绿）。

### 1.1 P3 任务预览

| 任务 | 内容 | 状态 |
|---|---|---|
| T1 | Tentacle 集成：MCP 代理作为 Tentacle 插件，验证学习→执行闭环 | ⏳ 预览 |
| T2 | Anaphase 集成：参数化 Manifest 的意图识别 + 参数填充 | ⏳ 预览 |
| T3 | 真实 MCP Server 验证：用 npx 官方 MCP Server（filesystem/github）验证 | ⏳ 预览 |
| T4 | 性能压测：高并发工具调用 + 批量学习性能 | ⏳ 预览 |
| T5 | 文档完善 + 示例 + 提交 | ⏳ 预览 |

---

## 2. 已完成阶段：P2 — OS Glove + 多 MCP Server + MCP 代理执行体

> **完成时间**：2026-08-30
> **测试**：42 个全绿（35 单元 + 3 集成 + 1 性能 + 3 代理）

### 2.1 任务完成情况

| 任务 | 内容 | 状态 |
|---|---|---|
| T1 | MCP 代理执行体（mcp-proxy）：通用 MCP 工具调用代理，可被 Tentacle 加载执行 | ✅ 完成 |
| T2 | 多 MCP Server 支持：配置管理 + 批量学习 + 结果合并/去重 | ✅ 完成 |
| T3 | macOS Glove 最小版本：文件/进程/AppleScript 系统 API 适配（6 个工具） | ✅ 完成 |
| T4 | 增量学习 + 版本管理：工具变化检测 + 增量学习 + 废弃标记 | ✅ 完成 |
| T5 | 文档完善 + 集成测试 + 性能验证 | ✅ 完成 |

### 2.2 关键决策（已确认）

| # | 决策点 | 最终方案 |
|---|---|---|
| D1 | MCP 代理执行体形式 | 独立 Library Crate，可被多组件复用（非 Tentacle 静态插件） |
| D2 | 多 MCP Server 配置格式 | TOML |
| D3 | macOS Glove 实现方式 | 直接生成 CI-144 工具 + 可作为 MCP Server 被学习 |
| D4 | 增量学习策略 | 工具清单 diff（新增/变更/删除检测） |

---

## 3. 阶段总览地图

| 阶段 | 内容 | 状态 |
|---|---|---|
| P0 | 项目初始化 + DNA/RNA/PLAN/GROWTH 方法论骨架 | ✅ 完成 |
| P1 | MCP-Learner 最小验证（stdio + mock-server + 端到端，17 测试） | ✅ 完成 |
| **P2** | **OS Glove + 多 MCP Server + MCP 代理执行体（42 测试）** | **✅ 完成** |
| P3 | 生态集成 + 高级特性（Tentacle/Anaphase/Mind 联调） | ⏳ 预览 |

### 1.5 下一阶段预览：P3 — 生态联调 + 高级特性

- 与 Tentacle 深度集成（插件热加载）
- 与 Tuck 安全闸门联动（自动生成策略规则）
- 与 Anaphase 编排层对接（意图识别 + 参数填充）
- Linux Glove / 鸿蒙 Glove
- MCP Server 远程传输（SSE/HTTP）
- 学习结果持久化（SQLite）

---

## 2. 阶段总览（地图，不展开）

| 阶段 | 内容 | 状态 |
|---|---|---|
| P1 | MCP-Learner 最小验证（stdio + mock-server + 端到端） | ✅ 已完成 |
| P2 | OS Glove + 多 MCP Server + MCP 代理执行体 | ✅ 已完成 |
| P3 | 生态联调 + 高级特性（Tentacle 集成 + 全链路修复） | ✅ 已完成 |
| P4-T1 | post_learn 审查管道（raw/staging/stable/rejected 状态迁移自动化） | ✅ 已完成 |

---

## 3. 活跃决策与契约指针

| 项 | 指针 |
|---|---|
| MCP 协议 | JSON-RPC 2.0 over stdio/SSE（modelcontextprotocol.io） |
| CI-144 协议家族 | CommonIntents/BIND-19 v2.0-alpha（PFP-xCF14 + SAP-xCF14） |
| Tentacle 插件 Manifest | Helix-Tentacle `crates/tentacle-core/src/manifest.rs` |
| PFP 风险评级 | `src/ci144/mod.rs` risk_rating() 函数 |
| ECO-Glove 愿景 | Helix-Mind `docs/vision/helix-eco-glove-vision.md` |
| ADR 决策记录 | `docs/decisions/`（ADR-0001~0004） |

---

## 4. 文档生态 SOP（phyt-DNA v1.0）

PLAN 是导航牌不是历史档案；阶段收尾时完成记录追加 GROWTH.md 并从 PLAN 移除；GROWTH ≤3 条超则归档；PLAN ≤150 行超则触发历史迁移。提交信息必须包含 ADR 关联 `(ADR-NNNN §Tx)`。详见 `docs/DNA.md` 和 `docs/RNA.md`。
