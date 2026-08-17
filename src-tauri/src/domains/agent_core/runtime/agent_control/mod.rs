//! Agent 控制用例的应用运行时合同。
//!
//! 该模块只表达运行时需要的事实，不表达具体实现：
//! - Todo port 负责一次完整的 Todo 更新；
//! - Sidechain port 负责子任务的启动、查询、停止和停止请求检查；
//! - AgentToolEvent port 负责把归一化的工具事实交给运行时投影。
//!
//! 这些合同刻意不依赖 `TaskManager`、`Session`、`AgentToolEvent` 或 UI DTO。
//! 当前 UI runtime 可以通过适配器接入，后续其他 application use-case 也可以
//! 复用同一组能力，而无需复制任务事实或把 UI 投影下沉到工具域。

