# Helix-MCP-Learner

**MCP Server → CI-144 工具定义自动学习器——Helix 直接挖矿，不淘金。**

[![License](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![Status](https://img.shields.io/badge/Status-P1--完成-brightgreen.svg)]()
[![Tests](https://img.shields.io/badge/Tests-17%20通过-brightgreen.svg)]()

> MCP Server 是"已结构化的金矿"——有人已经帮我们把混乱的外部世界结构化了。
> Helix 不需要用 OCR 去淘金，直接用 MCP-Learner 去挖矿就行了。
> **去掉 MCP 的皮，提炼 CI-144 的骨。**

[English Version](README.md) | 中文版

---

## 定位

MCP-Learner 是 Helix 生态的"翻译官"——将 MCP Server 的工具接口翻译为 CI-144 标准工具定义，让 [Helix-Tentacle](https://github.com/Jasonmilk/Helix-Tentacle) 可以直接执行。

**它不执行工具，只学习和提炼。**

```
外部世界（GitHub/Slack/Notion/文件系统/…）
    ↑ 原生 API（REST/GraphQL/SDK）
MCP Server（已结构化的工具接口）
    ↑ MCP 协议（JSON-RPC 2.0 over stdio/SSE）
MCP-Learner（本项目）
    ├── 发现（扫描 MCP Server）
    ├── 学习（tools/list → 工具清单 + schema）
    ├── 提炼（去掉 MCP 包装，提取 API 调用模式）
    ├── 评级（PFP Risk-Level 自动分配）
    └── 生成（Tentacle 可加载的插件 Manifest）
    ↓ CI-144 工具定义（Manifest）
Helix-Tentacle（执行）
    ↓
Tuck（安全决策）
```

---

## 核心哲学（phyt-DNA v1.0）

| 哲学 | 体现 |
|---|---|
| **极致解耦** | MCP-Learner 不知道 Tentacle 的内部实现，Tentacle 不知道 MCP 的存在，两者通过 Manifest 通信。 |
| **极致复用** | 复用 MCP 协议、CI-144 协议家族、Tentacle 插件系统。 |
| **按需加载** | 只在需要学习新 MCP Server 时才激活，学习结果缓存后直接使用。 |
| **按需驱动** | 事件驱动，MCP Server 变化时触发重新学习，不轮询。 |
| **物理事实优先** | PFP Risk-Level 基于工具实际操作风险（删除=CRITICAL，读取=LOW）。 |
| **确定性优先** | 相同 MCP Server + 相同学习逻辑 → 相同 CI-144 工具定义。 |

---

## 当前状态

**P1 最小验证已完成** ✅（2026-08-30）

| 任务 | 内容 | 状态 |
|---|---|---|
| T1 | MCP Client 基础层（stdio + JSON-RPC 2.0 + tools/list + tools/call） | ✅ 完成 |
| T2 | CI-144 工具提炼层（CIN7 + CAPABILITY-13 + PFP 风险评级） | ✅ 完成 |
| T3 | Tentacle 插件 Manifest 生成器 | ✅ 完成 |
| T4 | 端到端验证：mock-mcp-server → 学习 → 生成 → 验证 | ✅ 完成 |
| T5 | 效率对比 + 确定性验证 | ✅ 完成 |

**测试**：17 个全绿（14 单元 + 3 集成）

### 性能数据

| 指标 | 数值 | 说明 |
|---|---|---|
| 学习过程（4 工具） | ~107ms | 一次性成本 |
| MCP 直接调用 | ~239μs | read_file 平均延迟 |
| Manifest 加载 | ~131μs | 单个工具读取+解析 |
| 风险评级 | ~1.3μs | 单次评级 |
| **CI-144 重封装开销** | **<1%** | Manifest 加载 vs MCP 调用 |

---

## 快速开始

```bash
# 克隆
git clone https://github.com/Jasonmilk/Helix-MCP-Learner.git
cd Helix-MCP-Learner

# 构建
cargo build --release

# 学习 MCP Server，生成 Tentacle 插件 Manifest
./target/release/mcp-learner learn \
  --command python3 \
  --args tests/mock_mcp_server.py \
  --output ./plugins \
  --name mock-filesystem

# 查看已学习的工具
./target/release/mcp-learner list --plugins-dir ./plugins
```

### 输出示例

```
✅ Learning complete!
   Server: mock-filesystem
   Tools: 4
   Output: ./plugins

   Load in Tentacle:
   tentacle --transport stdio --plugins-dir ./plugins
```

---

## PFP 风险评级规则（自动）

| 工具名模式 | Risk-Level | 例子 |
|---|---|---|
| `read_*` / `list_*` / `get_*` / `search_*` | LOW | read_file, list_issues, get_user |
| `create_*` / `update_*` / `write_*` / `send_*` | MEDIUM | create_issue, update_file, send_message |
| `delete_*` / `remove_*` / `execute_*` / `run_*` | CRITICAL | delete_repo, remove_file, execute_command |
| `*_all` / `*_system` / `*_admin` / `*_root` | CATASTROPHIC | delete_all, system_update, admin_override |

**安全约束**：CRITICAL/CATASTROPHIC 级别的工具学习后，需要人工确认才能启用。

---

## 模板化复用（工作原理）

MCP-Learner 生成的是**参数化**的 Manifest，而不是硬编码的工具调用。`parameters_schema` 是标准 JSON Schema：

```json
{
  "type": "object",
  "properties": {
    "name": {"type": "string", "description": "要查询的联系人姓名"}
  },
  "required": ["name"]
}
```

这意味着：
- `name` 是一个**参数槽**，不是硬编码值
- Anaphase（编排层）在运行时填入任何值（Coco、Jim、任何人名）
- Tentacle 只验证参数类型，不关心具体值

**架构分工**：
- **Tentacle（手）**：无状态执行器，泛化能力为 0
- **Anaphase（大脑/编排器）**：意图识别 + 参数填充
- **Mind（记忆）**：高频操作的 L1 策略持久化
- **MCP-Learner（翻译官）**：从 MCP 工具中提炼参数化 Schema

> 手只管抓东西，大脑决定这次抓苹果、下次抓梨。
> MCP-Learner 的作用是把"抓取动作"提炼成标准流程，让大脑能随意填充目标。

---

## 项目结构

```
Helix-MCP-Learner/
├── src/
│   ├── lib.rs              # 库根，重新导出
│   ├── main.rs             # CLI 入口
│   ├── mcp/
│   │   └── mod.rs          # MCP Client（stdio + JSON-RPC 2.0）
│   ├── ci144/
│   │   └── mod.rs          # CI-144 提炼 + PFP 风险评级
│   └── manifest/
│       └── mod.rs          # Tentacle Manifest 生成器
├── tests/
│   ├── integration_test.rs # 端到端测试
│   ├── perf_test.rs        # 性能对比测试
│   └── mock_mcp_server.py  # 测试用模拟 MCP Server
├── docs/
│   ├── DNA.md              # 宪法：6 条不可变原则
│   ├── RNA.md              # 加载协议：AI 如何读取本仓库
│   ├── PLAN.md             # 导航：当前阶段 + 下一阶段预览
│   ├── GROWTH.md           # 生长记录：最近 3 次健康快照
│   └── decisions/          # 架构决策记录（ADR）
├── Cargo.toml
├── README.md               # 英文版
└── README.zh-cn.md         # 中文版（本文件）
```

---

## Helix 生态

| 项目 | 角色 | 状态 |
|---|---|---|
| [Helix-Mind](https://github.com/Jasonmilk/Helix-Mind) | 大脑（记忆/认知） | ✅ 核心完成 |
| [Anaphase-Helix](https://github.com/Jasonmilk/Anaphase-Helix) | 躯干（编排/执行） | ✅ 待裁决 |
| [Helix-Tentacle](https://github.com/Jasonmilk/Helix-Tentacle) | 手（工具执行） | ✅ 完成 |
| [Tuck](https://github.com/Jasonmilk/Tuck) | 免疫系统（安全闸门） | ✅ 完成 |
| [Cellrix](https://github.com/Jasonmilk/Cellrix) | 皮肤（UI/展示） | ✅ 完成 |
| [BIND-19](https://github.com/CommonIntents/BIND-19) | 神经系统（CI-144 协议） | ✅ 完成 |
| **Helix-MCP-Learner** | **翻译官（MCP → CI-144）** | **✅ P1 完成** |

---

## 治理

本项目遵循 **phyt-DNA 方法论 v1.0**。

| 文档 | 用途 |
|---|---|
| [docs/DNA.md](docs/DNA.md) | 宪法：6 条不可变原则 |
| [docs/RNA.md](docs/RNA.md) | 加载协议：AI 如何读取本仓库 |
| [docs/PLAN.md](docs/PLAN.md) | 导航：当前阶段 + 下一阶段预览 |
| [docs/GROWTH.md](docs/GROWTH.md) | 生长记录：最近 3 次健康快照 |
| [docs/decisions/](docs/decisions/) | 架构决策记录（ADR） |

---

## 许可证

Apache 2.0。由 phyt-DNA 方法论 v1.0 管理。

---

*Helix-MCP-Learner。去掉 MCP 的皮，提炼 CI-144 的骨。*
