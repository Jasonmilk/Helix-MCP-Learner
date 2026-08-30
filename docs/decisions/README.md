# 架构决策记录（ADR）索引

本目录记录 Helix-MCP-Learner 项目的所有重要架构决策。

## 命名规范

- 格式：`ADR-<4位编号>-<短标题>.md`
- 编号：4 位数字，按顺序递增（0001, 0002, ...）
- 引用：使用 `ADR-<编号>`（如 ADR-0001）
- 文件名与引用名必须一致

## 决策列表

| 编号 | 标题 | 状态 | 日期 |
|---|---|---|---|
| [ADR-0001](ADR-0001-independent-project-architecture.md) | 独立项目架构（而非 Tentacle 静态插件） | Accepted | 2026-08-30 |
| [ADR-0002](ADR-0002-pfp-risk-rating-rules.md) | PFP 风险评级规则（工具名模式匹配） | Accepted | 2026-08-30 |
| [ADR-0003](ADR-0003-template-based-reuse-architecture.md) | 模板化复用架构（参数化 Manifest） | Accepted | 2026-08-30 |
| [ADR-0004](ADR-0004-stdio-transport-first.md) | stdio 传输优先（P1 阶段） | Accepted | 2026-08-30 |
| [ADR-0005](ADR-0005-p2-architecture-decisions.md) | P2 架构决策（代理执行体 + 多 Server + macOS Glove + 增量学习） | Accepted | 2026-08-30 |

## 状态定义

- **Proposed**: 提议中，待讨论
- **Accepted**: 已接受，生效中
- **Deprecated**: 已废弃，被新决策替代
- **Superseded**: 已被替代，请查看替代决策

## 创建新 ADR 的流程

1. 复制模板（如有）或创建新文件
2. 编号：使用下一个可用编号
3. 填写：背景、候选方案、决策、理由、后果、参考
4. 更新本索引文件
5. 提交时在 commit message 中关联 ADR 编号
