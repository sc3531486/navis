//! 项目绑定模块
//!
//! 处理 Session 与 Project 的关联关系。
//!
//! # 关系
//! - 1 Project : N Session
//! - 1 Session : 1 Project
//!
//! # project.switched 事件处理流程
//! 1. 保存当前 Project 活跃 Session 的检查点
//! 2. 查找目标 Project 的最近活跃 Session
//! 3. 存在则 setActive，不存在则 createSession 并绑定
//! 4. 发出 session.switched 事件

