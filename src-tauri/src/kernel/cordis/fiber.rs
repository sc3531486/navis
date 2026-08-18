//! Fiber 生命周期管理
//! 参考 DeepSeek Harness Cordis Fiber，通用框架，不绑定业务领域。

use super::context::CordisContext;
use super::service::Service;

/// Fiber 状态
#[derive(Debug, Clone, PartialEq)]
pub enum FiberState { Created, Starting, Running, Stopping, Disposed }

/// Cordis 风格的 Fiber（扩展运行时容器）
pub struct Fiber {
    pub id: String,
    pub extension_id: String,
    pub ctx: CordisContext,
    services: Vec<Box<dyn Service>>,
    disposers: Vec<Box<dyn FnOnce() -> Result<(), String> + Send>>,
    state: FiberState,
}

impl Fiber {
    pub fn new(extension_id: String, parent_ctx: &CordisContext) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let ctx = parent_ctx.isolate(format!("fiber:{extension_id}"));
        Self { id, extension_id, ctx, services: Vec::new(), disposers: Vec::new(), state: FiberState::Created }
    }
    pub fn add_service(&mut self, service: Box<dyn Service>) { self.services.push(service); }
    pub fn effect<F: FnOnce() -> Result<(), String> + Send + 'static>(&mut self, disposer: F) {
        self.disposers.push(Box::new(disposer));
    }
    pub fn start(&mut self) -> Result<(), String> {
        if self.state != FiberState::Created { return Err(format!("Cannot start fiber '{}': state {:?}", self.extension_id, self.state)); }
        self.state = FiberState::Starting;
        for s in &mut self.services { s.start().map_err(|e| format!("Service '{}' start failed: {e}", s.name()))?; }
        self.state = FiberState::Running;
        tracing::info!(fiber_id = %self.id, ext = %self.extension_id, "Fiber started");
        Ok(())
    }
    pub fn stop(&mut self) -> Result<(), String> {
        if self.state != FiberState::Running { return Ok(()); }
        self.state = FiberState::Stopping;
        for s in self.services.iter_mut().rev() { let _ = s.stop(); }
        for d in self.disposers.drain(..).rev() { let _ = d(); }
        self.state = FiberState::Disposed;
        tracing::info!(fiber_id = %self.id, ext = %self.extension_id, "Fiber disposed");
        Ok(())
    }
    pub fn state(&self) -> &FiberState { &self.state }
}

impl Drop for Fiber { fn drop(&mut self) { if self.state == FiberState::Running { let _ = self.stop(); } } }

/// Fiber 管理器
pub struct FiberManager { fibers: std::sync::Mutex<Vec<Fiber>> }
impl FiberManager {
    pub fn new() -> Self { Self { fibers: std::sync::Mutex::new(Vec::new()) } }
    pub fn create(&self, extension_id: impl Into<String>, parent_ctx: &CordisContext) -> Fiber {
        let fiber = Fiber::new(extension_id.into(), parent_ctx);
        self.fibers.lock().unwrap().push(Fiber::new(fiber.extension_id.clone(), parent_ctx));
        fiber
    }
    pub fn dispose(&self, extension_id: &str) -> Result<(), String> {
        let mut fibers = self.fibers.lock().unwrap();
        if let Some(pos) = fibers.iter().position(|f| f.extension_id == extension_id) {
            fibers.remove(pos).stop()?;
        }
        Ok(())
    }
}
impl Default for FiberManager { fn default() -> Self { Self::new() } }
