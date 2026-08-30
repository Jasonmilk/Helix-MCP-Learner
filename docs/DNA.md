# Helix-MCP-Learner DNA — 不可变原则

> **版本**：v1.0
> **日期**：2026-08-30
> **性质**：宪法，不可变。修改需人类授权 + ADR 记录。

---

## 六条核心哲学（来自 phyt-DNA 方法论）

### 1. 极致解耦
MCP-Learner 只负责"学习和提炼"，不负责"执行"。Helix 核心（Tentacle）不知道 MCP 的存在，MCP-Learner 也不知道 Tentacle 的内部实现。两者通过 CI-144 工具定义（Manifest）通信。

### 2. 极致复用
不重新发明轮子。MCP 协议（JSON-RPC 2.0）、CI-144 协议家族（PFP-xCF14 + SAP-xCF14）、Tentacle 插件系统都是已验证的基础设施，直接复用。

### 3. 按需加载
只在需要学习新 MCP Server 时才激活 MCP-Learner。学习结果缓存后，运行时直接使用缓存，不反复学习。

### 4. 按需驱动
事件驱动，无轮询。MCP Server 变化时通过文件系统事件或配置变更触发重新学习，不主动轮询。

### 5. 物理事实优先
PFP Risk-Level 基于工具的实际操作风险（删除=CRITICAL，读取=LOW），不基于 AI 语义推理。风险评级规则是确定性的、可审计的。

### 6. 确定性优先
相同 MCP Server + 相同学习逻辑 → 相同 CI-144 工具定义。学习结果是确定性的，可复现，可测试。

---

## 项目定位

MCP-Learner 是 Helix 生态的"翻译官"——将 MCP Server 的工具接口翻译为 CI-144 标准工具定义，让 Tentacle 可以直接执行。

**不做**：
- ❌ 不执行工具（那是 Tentacle 的职责）
- ❌ 不做安全决策（那是 Tuck 的职责）
- ❌ 不做认知/学习（那是 Mind 的职责）
- ❌ 不做 UI 渲染（那是 Cellrix 的职责）

**只做**：
- ✅ 发现 MCP Server
- ✅ 学习工具清单（tools/list）
- ✅ 试调用学习返回格式（tools/call）
- ✅ 提炼为 CI-144 工具定义（CIN7 + CAPABILITY-13 + PFP）
- ✅ 生成 Tentacle 可加载的插件 Manifest
- ✅ 缓存学习结果 + 监控变化

---

## 命名规范

- 项目名：`Helix-MCP-Learner`
- 二进制名：`mcp-learner`
- 输出格式：Tentacle 插件 Manifest（JSON）
- 风险评级：PFP Risk-Level（LOW/MEDIUM/CRITICAL/CATASTROPHIC）

---

*DNA v1.0 定稿。修改需 ADR + 人类授权。*
