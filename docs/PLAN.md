# Helix-MCP-Learner 开发导航牌（PLAN）

> **版本**：v1.1（P1 完成，2026-08-30）
> **状态**：✅ P1 MCP-Learner 最小验证（已完成）
> **分支**：main
> **所属方法论**：phyt-DNA 方法论 v1.0（方法论锚点项目 https://github.com/Jasonmilk/phyt-DNA）
> **规则**：本文件只含当前阶段 + 下一阶段预览 + 阶段总览地图。完成阶段 → GROWTH.md。总行数 ≤150，超出触发历史迁移。

---

## 1. 当前阶段：P1 — MCP-Learner 最小验证 ✅ 已完成

> **状态**：✅ 已完成。17 个测试全绿（14 单元 + 3 集成），性能验证通过。
> **目标**：验证"MCP Server → 学习 → CI-144 工具定义 → Tentacle 执行"的完整链路。
> **前置依赖**：Tentacle 插件系统已完成（P0-P5），CI-144 v2.0 已冻结（PFP-xCF14 + SAP-xCF14）。

### 1.1 任务拆分

| 任务 | 内容 | 状态 |
|---|---|---|
| T1 | MCP Client 基础层（stdio 传输 + JSON-RPC 2.0 + tools/list + tools/call） | ✅ 完成 |
| T2 | CI-144 工具提炼层（MCP 工具 → CIN7 意图 + CAPABILITY-13 能力 + PFP 风险评级） | ✅ 完成 |
| T3 | Tentacle 插件 Manifest 生成器（学习结果 → 可加载的插件 Manifest） | ✅ 完成 |
| T4 | 最小验证：mock-mcp-server 学习 → 生成 → 验证 Manifest | ✅ 完成 |
| T5 | 效率对比 + 确定性验证 + 文档完善 | ✅ 完成 |

### 1.2 代码真相源

- **MCP Client（T1 ✅）**：`src/mcp/` — stdio 传输、JSON-RPC 2.0 协议、tools/list、tools/call
- **CI-144 提炼（T2 ✅）**：`src/ci144/` — 工具名映射、参数 schema 转换、PFP 风险评级规则
- **Manifest 生成器（T3 ✅）**：`src/manifest/` — Tentacle 插件 Manifest JSON 生成
- **CLI 入口（T4 ✅）**：`src/main.rs` — `mcp-learner learn --command <cmd> --output <dir>`
- **测试（T5 ✅）**：`tests/` — 集成测试、确定性测试、性能对比测试

### 1.3 关键决策点（已确认）

| # | 决策点 | 方案 | 状态 |
|---|---|---|---|
| D1 | MCP 传输方式 | stdio（本地），SSE/HTTP 留待 P2 | ✅ 已确认 |
| D2 | 风险评级规则 | 工具名模式匹配（read_*=LOW, delete_*=CRITICAL） | ✅ 已确认 |
| D3 | 学习结果缓存格式 | JSON 文件（每个工具一个 Manifest + 索引文件） | ✅ 已确认 |
| D4 | 增量学习策略 | P1 全量重学，P2 实现增量 diff | ✅ 已确认 |

### 1.4 验收结果

- T1：MCP Client 可连接 mock MCP Server，成功调用 tools/list 和 tools/call ✅
- T2：MCP 工具可提炼为 CI-144 工具定义（含 CIN7 意图名、CAPABILITY-13 参数 schema、PFP Risk-Level）✅
- T3：生成的 Manifest 符合 Tentacle 格式（name/version/executable/integrity/parameters_schema/security_level）✅
- T4：端到端验证：学习 4 个工具 → 生成 4 个 Manifest + 1 索引 → 验证内容正确 ✅
- T5：效率对比数据 + 确定性测试通过 ✅
- `cargo test --workspace` 全绿：17 个测试（14 单元 + 3 集成）✅

### 1.5 性能数据

| 指标 | 数值 | 说明 |
|---|---|---|
| 学习过程（4 工具） | ~107ms | 一次性成本，含连接+初始化+列表+提炼+生成 |
| MCP 直接调用 | ~239μs | read_file 平均延迟（10 次） |
| Manifest 加载 | ~131μs | 单个工具 Manifest 读取+解析 |
| 风险评级 | ~1.3μs | 单次评级（3000 次平均） |
| CI-144 重封装开销 | <1% | Manifest 加载 vs MCP 调用 |

### 1.6 下一阶段预览：P2 — OS Glove + 多 MCP Server 支持

- macOS Glove 最小版本（文件/进程/AppleScript）
- 多 MCP Server 同时学习
- 增量学习 + 版本管理
- 学习结果持久化（SQLite）
- Tuck 策略规则自动生成
- MCP 代理执行体（mcp_proxy.js）实现，让 Tentacle 可直接执行学习后的工具

---

## 2. 阶段总览（地图，不展开）

| 阶段 | 内容 | 状态 |
|---|---|---|
| **P1** | **MCP-Learner 最小验证（stdio + mock-server + 端到端）** | **✅ 已完成** |
| P2 | OS Glove + 多 MCP Server + 增量学习 + MCP 代理执行体 | ⏳ 预览 |
| P3 | 鸿蒙/安卓手套 + IoT 支持 | ⏳ 远期 |

---

## 3. 活跃决策与契约指针

| 项 | 指针 |
|---|---|
| MCP 协议 | JSON-RPC 2.0 over stdio/SSE（modelcontextprotocol.io） |
| CI-144 协议家族 | CommonIntents/BIND-19 v2.0-alpha（PFP-xCF14 + SAP-xCF14） |
| Tentacle 插件 Manifest | Helix-Tentacle `crates/tentacle-core/src/manifest.rs` |
| PFP 风险评级 | `src/ci144/mod.rs` risk_rating() 函数 |
| ECO-Glove 愿景 | Helix-Mind `docs/vision/helix-eco-glove-vision.md` |

---

## 4. 文档生态 SOP（phyt-DNA v1.0）

PLAN 是导航牌不是历史档案；阶段收尾时完成记录追加 GROWTH.md 并从 PLAN 移除；GROWTH ≤3 条超则归档；PLAN ≤150 行超则触发历史迁移。提交信息必须包含 ADR 关联 `(ADR-NNNN §Tx)`。详见 `docs/DNA.md` 和 `docs/RNA.md`。
