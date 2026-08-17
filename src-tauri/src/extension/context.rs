//! Host-side Cordis runtime for backend extensions.
//!
//! The host owns one root [`Context`] and keeps a fiber per installed backend
//! extension. Services registered here are shared with every backend extension
//! plugin and are disposed through the owning fiber.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::anyhow;
use cordis::{Context, Fiber};

fn cordis_error(context: &str, error: cordis::CordisError) -> anyhow::Error {
    anyhow!("{context}: {error}")
}

/// 类型擦除的 capability 端口存储单元。
///
/// cordis `Context::provide_arc/get/require` 的类型参数受隐式 `Sized` 约束，
/// 无法直接把 `dyn Trait` 作为类型参数传入。此包装持有一个 `Arc<dyn Trait>`
/// （自身是 Sized），使宿主可以"provide 前擦除、读取时还原 `Arc<dyn Trait>`"，
/// 同时保持 capability port 可选注入（未注册时 get 返回 `None` → fail-closed）。
pub struct ErasedCapability<T: ?Sized>(Arc<T>);

impl<T: ?Sized> ErasedCapability<T> {
    /// 包裹一个已擦除的 capability 端口。
    pub fn new(value: Arc<T>) -> Self {
        Self(value)
    }

    /// 借用内部已擦除的端口。
    pub fn inner(&self) -> &Arc<T> {
        &self.0
    }
}

impl<T: ?Sized> Clone for ErasedCapability<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

/// 惰性解析一个可选 capability 端口；缺服务时返回 `None`（fail-closed）。
///
/// 宿主注册（[`register_capability_service`] 存 `Arc<dyn Trait>`，经
/// [`ErasedCapability`] 擦除）与扩展 apply（fiber 内 `ctx.get`）共用同一套
/// 解析逻辑；未注册时 `get` 返回 `None`，扩展在对应 family 上保持 fail-closed。
pub fn resolve_capability<T: ?Sized>(ctx: &Context, name: &str) -> anyhow::Result<Option<Arc<T>>>
where
    T: Send + Sync + 'static,
{
    Ok(ctx
        .get::<ErasedCapability<T>>(name)?
        .map(|capability| capability.inner().clone()))
}

/// Owns the root Cordis context used by backend extensions.
pub struct HostExtensionContext {
    root: Arc<Context>,
    fibers: Mutex<HashMap<String, Fiber>>,
}

/// Runtime alias kept intentionally descriptive for host wiring.
pub type ExtensionCordisRuntime = HostExtensionContext;

impl HostExtensionContext {
    /// Create an empty host runtime with a fresh root context.
    pub fn new() -> Self {
        Self {
            root: Arc::new(Context::new()),
            fibers: Mutex::new(HashMap::new()),
        }
    }

    /// Return a clone of the underlying root context.
    pub fn root_context(&self) -> Arc<Context> {
        self.root.clone()
    }

    /// Borrow the underlying root context.
    pub fn context(&self) -> &Context {
        &self.root
    }

    /// 注册一个可选 capability 端口服务。
    ///
    /// capability port 是可选注入：白板空壳下宿主可能未装配对应子系统（`None`），
    /// 此时调用方不注册该服务，扩展 apply 内 `ctx.get` 返回 `None`，在对应 family
    /// 上保持 fail-closed。提供值在存入前擦除为 `Arc<dyn Trait>`（经
    /// [`ErasedCapability`]），读取时还原为 `Arc<dyn Trait>`；重复注册同名服务
    /// 返回错误（cordis `DuplicateService`）。
    pub fn register_capability_service<T: ?Sized>(
        &self,
        name: &str,
        value: Arc<T>,
    ) -> anyhow::Result<()>
    where
        T: Send + Sync + 'static,
    {
        self.root
            .provide_arc(name, Arc::new(ErasedCapability::new(value)))
            .map(|_| ())
            .map_err(|error| cordis_error("failed to register capability service", error))
    }

    /// 读取一个可选 capability 端口服务；未注册时返回 `None`（fail-closed）。
    pub fn get_capability_service<T: ?Sized>(
        &self,
        name: &str,
    ) -> anyhow::Result<Option<Arc<T>>>
    where
        T: Send + Sync + 'static,
    {
        resolve_capability(&self.root, name)
            .map_err(|error| anyhow!("failed to read capability service: {error}"))
    }

    /// 要求一个已注册的 capability 端口服务；未注册时返回错误。
    pub fn require_capability_service<T: ?Sized>(&self, name: &str) -> anyhow::Result<Arc<T>>
    where
        T: Send + Sync + 'static,
    {
        self.root
            .require::<ErasedCapability<T>>(name)
            .map(|capability| capability.inner().clone())
            .map_err(|error| cordis_error("required capability service is unavailable", error))
    }

    /// 启动一个后端扩展插件并返回其生命周期 fiber。
    ///
    /// 由 `ExtensionCordisPlugin::install` 调用；成功启动后 fiber 登记进本宿主的
    /// 强引用 fiber map（Cordis registry 只持 Weak 引用不保活），使
    /// `dispose_extension` / `take_extension_fiber` 可精确撤销。
    pub(crate) fn track_fiber(&self, extension_id: String, fiber: Fiber) -> anyhow::Result<()> {
        let mut fibers = self
            .fibers
            .lock()
            .map_err(|_| anyhow!("extension context mutex poisoned"))?;
        if let Some(previous) = fibers.remove(&extension_id) {
            drop(fibers);
            if let Err(error) = previous.dispose() {
                tracing::warn!(
                    extension_id = %extension_id,
                    error = %error,
                    "Failed to dispose previous Cordis fiber while reinstalling extension"
                );
            }
            fibers = self
                .fibers
                .lock()
                .map_err(|_| anyhow!("extension context mutex poisoned"))?;
        }
        fibers.insert(extension_id, fiber);
        Ok(())
    }

    /// 取出并返回一个已跟踪的扩展 fiber，不触发 dispose。
    ///
    /// 生命周期 disable 用它区分两条清理路径：fiber 存在时走 dispose（其
    /// disposer 负责运行时资源撤销）；fiber 已不存在（残余重试/失败启用）
    /// 时手动消费 runtime_handles 账本清理残余。
    pub(crate) fn take_extension_fiber(
        &self,
        extension_id: &str,
    ) -> anyhow::Result<Option<Fiber>> {
        self.fibers
            .lock()
            .map_err(|_| anyhow!("extension context mutex poisoned"))
            .map(|mut fibers| fibers.remove(extension_id))
    }

    /// Dispose one backend extension fiber by ID.
    pub fn dispose_extension(&self, extension_id: &str) -> anyhow::Result<()> {
        let fiber = self
            .fibers
            .lock()
            .map_err(|_| anyhow!("extension context mutex poisoned"))?
            .remove(extension_id);
        if let Some(fiber) = fiber {
            fiber
                .dispose()
                .map_err(|error| anyhow!("failed to dispose extension `{extension_id}`: {error}"))?;
        }
        Ok(())
    }
}

impl Default for HostExtensionContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试用 capability trait：验证 `Arc<dyn Trait>` 的擦除 provide/get/require 往返。
    trait GreeterPort: Send + Sync {
        fn greeting(&self) -> &'static str;
    }

    struct StaticGreeter;

    impl GreeterPort for StaticGreeter {
        fn greeting(&self) -> &'static str {
            "hello"
        }
    }

    #[test]
    fn capability_service_dyn_trait_round_trip() {
        let host = HostExtensionContext::new();

        // 提供前擦除为 `Arc<dyn Trait>`，存入 Cordis service。
        host.register_capability_service::<dyn GreeterPort>("greet", Arc::new(StaticGreeter))
            .unwrap();

        let provided: Arc<dyn GreeterPort> = host
            .get_capability_service::<dyn GreeterPort>("greet")
            .unwrap()
            .expect("capability service should be visible after registration");
        assert_eq!(provided.greeting(), "hello");

        let required: Arc<dyn GreeterPort> = host
            .require_capability_service::<dyn GreeterPort>("greet")
            .unwrap();
        assert_eq!(required.greeting(), "hello");
    }

    #[test]
    fn duplicate_capability_service_is_rejected() {
        let host = HostExtensionContext::new();
        host.register_capability_service::<dyn GreeterPort>("greet", Arc::new(StaticGreeter))
            .unwrap();

        let result =
            host.register_capability_service::<dyn GreeterPort>("greet", Arc::new(StaticGreeter));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("has been registered"));
    }

    #[test]
    fn missing_capability_service_returns_none() {
        let host = HostExtensionContext::new();
        let service = host.get_capability_service::<dyn GreeterPort>("mcp").unwrap();
        assert!(service.is_none());
    }
}
