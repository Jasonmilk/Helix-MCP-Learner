# ADR-0003: 模板化复用架构（参数化 Manifest）

- **状态**: Accepted
- **日期**: 2026-08-30
- **决策者**: Jasonmilk
- **关联**: phyt-DNA 方法论 v1.0 §极致解耦 §极致复用

## 背景

用户提出疑问：插件（工具）是否具备"模板化"复用能力——即抽象出"查询通讯录"这个意图，而非绑定"Coco"这个具体实体？

## 决策

**MCP-Learner 生成参数化的 Manifest（JSON Schema），而非硬编码的工具调用。泛化能力由 Anaphase（编排层）+ Mind（记忆层）提供，Tentacle 只负责无状态执行。**

## 架构设计

### 1. Manifest 是参数化的

生成的 Manifest 中，`parameters_schema` 是标准 JSON Schema：

```json
{
  "type": "object",
  "properties": {
    "name": {"type": "string", "description": "Contact name to query"}
  },
  "required": ["name"]
}
```

- `name` 是一个**参数槽**，不是硬编码值
- Schema 定义结构，运行时填充值

### 2. 三层分工

| 层 | 组件 | 职责 | 泛化能力 |
|---|---|---|---|
| 执行层 | Tentacle | 无状态执行，验证参数类型 | 0（机械执行） |
| 编排层 | Anaphase | 意图识别 + 参数填充 + 工具调度 | 高（动态填充参数） |
| 记忆层 | Mind | L1 策略持久化，高频操作固化 | 高（学习使用模式） |
| 翻译层 | MCP-Learner | 从 MCP 工具提炼参数化 Schema | 中（抽象参数结构） |

### 3. 用户交互流程

```
用户："查 Coco 的电话"
  ↓
Anaphase：解析意图 PHONEBOOK_QUERY，填充 name=Coco
  ↓
Tentacle：执行 phonebook_query(name="Coco")
  ↓
用户："再查 Jim 的"
  ↓
Anaphase：复用同一工具模板，填充 name=Jim
  ↓
Tentacle：执行 phonebook_query(name="Jim")
```

用户不需要手动修改 Manifest，参数由 Anaphase 运行时填充。

## 理由

### 1. 极致解耦（phyt-DNA 哲学）

- 手（Tentacle）只管执行，不管泛化
- 脑（Anaphase/Mind）负责泛化，不管执行细节
- 翻译官（MCP-Learner）负责抽象参数结构

### 2. 极致复用

- 一个 Manifest 模板可以服务无限次不同参数的调用
- 不需要为每个实体（Coco、Jim、...）创建独立工具
- 学习一次，永久使用

### 3. 确定性优先

- Manifest 是静态声明，不包含业务逻辑
- 参数填充是运行时变量，两者分离
- 相同参数 → 相同执行结果

### 4. 与 CI-144 协议对齐

- CAPABILITY-13 本身就是参数化的能力声明
- INTENT-7 定义意图，不绑定具体实体
- PFP 定义风险等级，不绑定具体参数

## 后果

### 正面

- ✅ 一个模板服务无限次调用（极致复用）
- ✅ 用户不需要手动修改 Manifest
- ✅ 符合"手脑分离"架构
- ✅ 与 CI-144 协议设计一致

### 负面

- ⚠️ 需要 Anaphase 实现意图识别和参数填充（P2 生态联调阶段）
- ⚠️ Mind 的 L1 策略需要支持参数化模板（P10 阶段）

### 演进路径

- P2：Anaphase 实现基本的意图识别和参数填充
- P3：Mind 实现 L1 策略的参数化模板持久化
- P4：支持多工具组合的复杂模板（工作流）

## 参考

- phyt-DNA 方法论 v1.0 §极致解耦 §极致复用
- CI-144 v2.0 CAPABILITY-13 §参数化能力声明
- Helix-Mind P9 认知工艺 Phase 3 §L1 执行策略
- Tentacle 插件系统 `crates/tentacle-core/src/manifest.rs`
