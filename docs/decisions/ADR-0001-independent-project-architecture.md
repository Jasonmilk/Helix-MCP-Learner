# ADR-0001: 独立项目架构（而非 Tentacle 静态插件）

- **状态**: Accepted
- **日期**: 2026-08-30
- **决策者**: Jasonmilk
- **关联**: phyt-DNA 方法论 v1.0 §极致解耦

## 背景

在 P1 启动前，出现了一个架构选择：MCP-Learner 应该作为独立项目，还是作为 Helix-Tentacle 的静态插件（编译进同一二进制）？

### 候选方案

| 方案 | 描述 | 性能（学习场景） | 解耦性 | 复用性 |
|---|---|---|---|---|
| A. 独立项目 | 独立 Rust 仓库，CLI 调用 | ~107ms（一次性） | ✅ 优秀 | ✅ 优秀 |
| B. Tentacle 静态插件 | 编译进 Tentacle 二进制 | ~5μs（函数调用） | ❌ 差 | ❌ 差 |
| C. 独立 Library Crate | 核心逻辑为 crate，可被多项目依赖 | ~5μs（进程内可选） | ✅ 优秀 | ✅ 优秀 |

## 决策

**选择方案 A（独立项目）作为 P1 架构，预留方案 C（独立 Library Crate）作为 P2 演进路径。**

## 理由

### 1. 学习是低频操作，性能不是瓶颈

MCP-Learner 的核心工作是"学习"（一次性操作），而不是"高频调用"：
- 学习完成后，生成的 Manifest 被 Tentacle 加载
- 后续的工具执行通过 MCP 代理执行体，不经过 MCP-Learner
- 107ms 学习 4 个工具，完全可接受

用"高频调用"的性能数据来论证"低频学习"的架构选择，是场景错配。

### 2. 极致解耦（phyt-DNA 哲学）

MCP-Learner 作为独立项目：
- 不知道 Tentacle 的存在
- 可以被 Anaphase、Mind 等其他 Helix 组件复用
- 独立版本管理和发布
- 独立测试和调试

如果作为 Tentacle 插件，就绑定到了 Tentacle，其他组件无法直接使用。

### 3. 职责单一

- MCP-Learner：只做"学习和提炼"
- Tentacle：只做"执行"
- 两者通过 CI-144 工具定义（Manifest）通信

这符合"手脑分离"的架构设计。

### 4. 崩溃隔离

独立进程提供了天然的崩溃隔离：
- MCP-Learner 崩溃不会影响 Tentacle
- 虽然 Rust 的 catch_unwind 可以缓解，但独立进程是更强的隔离

## 后果

### 正面

- ✅ 保持极致解耦，符合 phyt-DNA 哲学
- ✅ 支持多项目复用（Tentacle、Anaphase、Mind）
- ✅ 独立版本管理和发布
- ✅ 独立测试和调试
- ✅ 崩溃隔离

### 负面

- ⚠️ 学习操作有进程创建开销（~2-5ms），但对低频操作可接受
- ⚠️ 需要独立维护项目基础设施（CI、文档等）

### 演进路径

P2 阶段可以考虑将核心逻辑重构为独立 Library Crate（`mcp-learner-core`）：
- Tentacle 可以选择依赖 crate 实现进程内调用（如果需要高性能）
- 保留 CLI 包装用于独立测试
- 其他 Helix 组件也可以依赖

## 参考

- [Helix ECO-Glove 愿景](../vision/helix-eco-glove-vision.md)（Helix-Mind 仓库）
- phyt-DNA 方法论 v1.0 §极致解耦
- Tentacle 插件系统：`crates/tentacle-core/src/manifest.rs`
