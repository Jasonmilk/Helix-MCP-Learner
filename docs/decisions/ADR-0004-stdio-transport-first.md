# ADR-0004: stdio 传输优先（P1 阶段）

- **状态**: Accepted
- **日期**: 2026-08-30
- **决策者**: Jasonmilk
- **关联**: MCP 协议规范 §传输层

## 背景

MCP 协议支持多种传输方式：stdio、SSE（Server-Sent Events）、HTTP、WebSocket。P1 阶段需要选择优先实现的传输方式。

### 候选方案

| 方案 | 适用场景 | 实现复杂度 | 性能 |
|---|---|---|---|
| A. stdio | 本地 MCP Server（同机） | 低 | 高（本地管道） |
| B. SSE | 远程 MCP Server（HTTP 流） | 中 | 中（网络延迟） |
| C. HTTP | 远程 MCP Server（REST 风格） | 中 | 中（网络延迟） |
| D. WebSocket | 远程 MCP Server（双向流） | 高 | 中（网络延迟） |

## 决策

**P1 阶段只实现 stdio 传输。SSE/HTTP/WebSocket 留待 P2 阶段实现。**

## 理由

### 1. 最小验证原则

P1 的目标是验证"MCP → CI-144 → Tentacle"的完整链路，而不是支持所有传输方式。stdio 是最简单的传输方式，可以快速验证核心逻辑。

### 2. 本地场景优先

Helix 生态的主要使用场景是本地：
- 本地 MCP Server（如 mcp-server-filesystem）
- 本地工具执行
- 本地安全决策

stdio 是本地场景的最优选择：
- 无网络依赖
- 低延迟（本地管道）
- 简单可靠

### 3. 极致解耦

传输层与核心逻辑解耦：
- `McpClient` trait 可以支持多种传输
- P1 实现 stdio，P2 可以添加 SSE/HTTP
- 核心提炼逻辑（ci144/、manifest/）不依赖传输方式

### 4. 测试友好

stdio 传输易于测试：
- 可以用 Python 脚本模拟 MCP Server
- 不需要网络服务
- 测试环境简单

## 实现细节

### stdio 协议

- 请求：JSON-RPC 2.0，按行写入 stdin
- 响应：JSON-RPC 2.0，按行读取 stdout
- 通知：无 id，不需要响应
- 初始化：`initialize` → `notifications/initialized`

### 错误处理

- 服务器关闭连接 → 返回 Protocol 错误
- JSON 解析失败 → 跳过该行（可能是服务器日志）
- 请求超时 → 返回 Timeout 错误（P2 实现）

## 后果

### 正面

- ✅ P1 快速验证核心链路
- ✅ 本地场景性能最优
- ✅ 测试简单可靠
- ✅ 传输层与核心逻辑解耦

### 负面

- ⚠️ P1 不支持远程 MCP Server
- ⚠️ 需要用户在本地安装 MCP Server

### 演进路径

P2 阶段添加：
- SSE 传输（远程 MCP Server）
- HTTP 传输（REST 风格 MCP Server）
- 传输方式自动检测（根据配置选择）
- 连接池和重试机制

## 参考

- MCP 协议规范：https://modelcontextprotocol.io
- JSON-RPC 2.0 规范：https://www.jsonrpc.org/specification
- phyt-DNA 方法论 v1.0 §按需加载 §极致解耦
