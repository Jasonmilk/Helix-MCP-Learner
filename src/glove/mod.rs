//! OS Glove 模块 — ⚠️ DEPRECATED
//!
//! # 已迁移至 HelixECO-Glove
//!
//! 本模块（`src/glove/`）中的所有原生 OS 适配代码已迁移至独立项目
//! **HelixECO-Glove**（https://github.com/Jasonmilk/HelixECO-Glove）。
//!
//! ## 迁移原因
//!
//! - **极致解耦**：原生 OS 适配与 MCP 学习是两个独立关注点，不应混在同一仓库
//! - **平台感知**：HelixECO-Glove 提供标准 `EcoGlove` trait + 平台感知加载器
//! - **Tentacle 集成**：HelixECO-Glove 提供 `tentacle-adapter`，可直接注册到 Helix-Tentacle
//! - **Monorepo 结构**：所有平台手套（macOS/Linux/鸿蒙/机器人）在一个 workspace 中，成熟后再拆分
//!
//! ## 迁移状态
//!
//! | 原模块 | 新位置 | 状态 |
//! |---|---|---|
//! | `src/glove/macos/` | `HelixECO-Glove/gloves/macos/` | ✅ 已迁移 |
//! | — | `HelixECO-Glove/adapters/tentacle/` | ✅ 新增（Tentacle 适配器） |
//!
//! ## 弃用计划
//!
//! - **当前版本（v0.2.x）**：保留本模块，标记为 deprecated，编译时输出警告
//! - **v0.3.0**：移除本模块，彻底切换至 HelixECO-Glove
//!
//! ## 如何使用新项目
//!
//! ```rust
//! // 旧方式（已弃用）
//! // use helix_mcp_learner::glove::macos::MacOSGlove;
//!
//! // 新方式
//! use helix_eco_glove_core::*;
//! use helix_eco_glove_macos::MacOSGlove;
//! use helix_eco_glove_tentacle_adapter::register_glove;
//! use tentacle_core::ToolRegistry;
//! use std::sync::Arc;
//!
//! let mut registry = ToolRegistry::new();
//! let glove = Arc::new(MacOSGlove::new()) as Arc<dyn EcoGlove>;
//! register_glove(&glove, &mut registry).unwrap();
//! ```

#![deprecated(
    since = "0.2.0",
    note = "OS Glove 模块已迁移至 HelixECO-Glove (https://github.com/Jasonmilk/HelixECO-Glove)，将在 v0.3.0 移除"
)]

pub mod macos;
