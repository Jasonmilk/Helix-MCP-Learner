# ADR-0005: P2 架构决策 — MCP 代理执行体 + 多 MCP Server + macOS Glove + 增量学习

**状态**：已采纳
**日期**：2026-08-30
**阶段**：P2

---

## 背景

P1 完成了 MCP-Learner 的最小验证（学习单个 MCP Server，生成 Tentacle 插件 Manifest）。P2 需要让 Helix 可以真正执行学习后的工具，并支持多 MCP Server 同时学习，同时提供 macOS 本地系统适配。

## 决策

### D1: MCP 代理执行体形式

**决策**：独立 Library Crate，可被多组件复用（非 Tentacle 静态插件）。

**理由**：
- 学习是低频操作（一次性），执行是高频操作，两者应该解耦
- 独立 Library Crate 可以被 Tentacle、Anaphase、Mind 等多组件复用
- 代理执行体只负责工具调用转发，不负责学习和提炼
- 懒连接设计：首次调用时才连接 MCP Server，减少资源占用

**备选方案**：
- 独立守护进程（UDS）：进程间通信开销大，不适合高频调用
- Tentacle 静态插件：耦合度高，无法被其他组件复用
- WASM 插件：复杂度高，当前阶段不需要

### D2: 多 MCP Server 配置格式

**决策**：TOML。

**理由**：
- TOML 是 Rust 生态的标准配置格式（Cargo.toml 就是 TOML）
- 支持注释、数组、嵌套结构，适合描述多个 MCP Server
- 比 YAML 更简单，比 JSON 更易读

**配置结构**：
```toml
[global]
output_dir = "./plugins"
conflict_strategy = "rename"  # error | skip | rename | overwrite

[[servers]]
name = "filesystem"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
```

### D3: macOS Glove 实现方式

**决策**：直接生成 CI-144 工具 + 可作为 MCP Server 被学习。

**理由**：
- macOS 系统 API 是已知且稳定的，不需要通过 MCP Server 学习
- 直接生成 CI-144 工具定义，性能更高，延迟更低
- 同时提供 MCP Server 包装器，方便与其他 MCP 工具统一管理

**工具列表**（6 个）：
1. `macos_read_file` - 读取文件（LOW）
2. `macos_write_file` - 写入文件（MEDIUM）
3. `macos_list_directory` - 列出目录（LOW）
4. `macos_execute_command` - 执行 shell 命令（CRITICAL）
5. `macos_list_processes` - 列出进程（LOW）
6. `macos_run_applescript` - 执行 AppleScript（CRITICAL）

### D4: 增量学习策略

**决策**：工具清单 diff（新增/变更/删除检测）。

**理由**：
- 全量重学对比简单但效率低，不适合工具数量多的场景
- 版本号对比需要 MCP Server 支持版本号，不是所有 MCP Server 都有
- 工具清单 diff 是最通用的方式：对比工具名、描述、参数 schema

**变化检测类型**：
1. **Added** - 新增工具
2. **Modified** - 工具变更（描述或 schema 变化）
3. **Unchanged** - 工具未变更
4. **Removed** - 工具被删除（废弃）

## 后果

### 正面
- MCP 代理执行体可以被多组件复用，符合"极致复用"哲学
- 多 MCP Server 支持让 Helix 可以同时学习多个外部服务
- macOS Glove 提供了本地系统工具，让 Helix 可以操作本地环境
- 增量学习减少了重复学习的开销，符合"按需加载"哲学

### 负面
- 模块数量增加（从 3 个增加到 7 个），维护成本上升
- macOS Glove 是平台特定的，其他平台需要单独实现
- 增量学习的变化检测需要对比 schema，可能有误报

## 验证

- `cargo test --workspace` 全绿：42 passed
- MCP 代理端到端测试：mock MCP Server → 代理 → 工具调用 → 返回结果
- 多 MCP Server 批量学习：TOML 配置 → 批量学习 → 结果合并
- macOS Glove 工具测试：文件读写、命令执行、进程列表
- 增量学习测试：新增/变更/删除检测

## 参考

- ADR-0001: 独立项目架构（而非 Tentacle 静态插件）
- ADR-0002: PFP 风险评级规则
- ADR-0003: 模板化复用架构
- ADR-0004: stdio 传输优先
