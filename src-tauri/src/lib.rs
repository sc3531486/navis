// ── navis 框架层（白板 + 扩展能力）──
pub mod kernel;
pub mod extension;
pub mod foundation;
pub mod security;
pub mod app;
pub mod ui;

// ── 业务域（扩展点，通过 domains/ 路由）──
pub mod domains;

pub use app::run;
