//! navis:ext 世界真实 guest 组件（design/37 C1-5 端到端验证轨）。
//!
//! 实现 `navis:ext/guest` 世界（fixtures/wit_ext/guest.wit，即生产
//! `navis:ext/ext` 世界 + 额外 import navis:host/log、navis:host/operation）：
//! - 导出 lifecycle.init / activate / deactivate 与 message.handle；
//! - `activate` 期间调用 host 的 `log.write` 与 `operation.list-operations`，
//!   验证 host 接口在真实组件中的全链路；任一步失败即返回 Err（fail-closed
//!   语义向上传播，宿主 activate 因此失败）。
//!
//! 重新生成：
//! ```bash
//! # 1) 由 fixtures/wit_{host,ext} 生成 guest 绑定（同目录下 .wasm 依赖它）
//! wit-bindgen rust --world navis:ext/guest --out-dir tests/fixtures \
//!   --generate-all tests/fixtures/wit_host tests/fixtures/wit_ext
//! # 2) 编译核心模块
//! rustc --target wasm32-unknown-unknown --edition 2021 -O \
//!   tests/fixtures/guest_navis_ext.rs -o tests/fixtures/guest_navis_ext_core.wasm
//! # 3) 组件化（读取绑定嵌入的 component-type 元数据，无需 --wit）
//! wasm-tools component new tests/fixtures/guest_navis_ext_core.wasm \
//!   -o tests/fixtures/navis_ext.wasm
//! ```
//!
//! 说明：生成的 `navis_ext_bindings.rs` 来自 wit-bindgen 0.57.1。其代码引用
//! `wit_bindgen::rt::...`（外部 crate），而 Rust 表达式路径不回落本地模块；
//! 为使 guest 保持「单一 rustc、无外部依赖」可编译（同 fixtures/guest_add.rs），
//! 绑定文件内已做两处 `crate::wit_bindgen::rt::` 前缀补丁，本文件再以本地
//! `pub mod wit_bindgen` shim 提供运行时支撑。重新生成绑定后需重打补丁。

#![no_std]
#![no_main]

extern crate alloc;

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}

// ============================================================================
// wit-bindgen 运行时 shim：生成代码仅引用 `wit_bindgen::rt` 的
// `run_ctors_once` / `maybe_link_cabi_realloc` 与顶层 `cabi_realloc` 导出。
// 此处以本地模块替代外部 wit-bindgen crate，保持 guest 单一 rustc 可编译
// （参考 fixtures/guest_add.rs 的无依赖模式）。
// ============================================================================

pub mod wit_bindgen {
    pub mod rt {
        /// 简单 guest 无静态构造器，no-op 即可（wasm-ld 无需合成 __wasm_call_ctors）。
        pub fn run_ctors_once() {}

        /// 强制 `cabi_realloc` 符号不被链接器 GC（与 wit-bindgen rt 同语义）。
        pub fn maybe_link_cabi_realloc() {
            #[used]
            static _NAME_DOES_NOT_MATTER: unsafe extern "C" fn(
                *mut u8,
                usize,
                usize,
                usize,
            ) -> *mut u8 = cabi_realloc;
        }

        /// canonical ABI realloc：组件边界所有跨内存分配统一走全局分配器。
        #[no_mangle]
        pub unsafe extern "C" fn cabi_realloc(
            old_ptr: *mut u8,
            old_len: usize,
            align: usize,
            new_len: usize,
        ) -> *mut u8 {
            unsafe {
                let layout =
                    alloc::alloc::Layout::from_size_align_unchecked(new_len.max(1), align.max(1));
                if new_len == 0 {
                    if !old_ptr.is_null() {
                        let old_layout = alloc::alloc::Layout::from_size_align_unchecked(
                            old_len.max(1),
                            align.max(1),
                        );
                        alloc::alloc::dealloc(old_ptr, old_layout);
                    }
                    return core::ptr::null_mut();
                }
                let new_ptr = alloc::alloc::alloc(layout);
                if !old_ptr.is_null() && !new_ptr.is_null() {
                    core::ptr::copy_nonoverlapping(old_ptr, new_ptr, old_len.min(new_len));
                    let old_layout = alloc::alloc::Layout::from_size_align_unchecked(
                        old_len.max(1),
                        align.max(1),
                    );
                    alloc::alloc::dealloc(old_ptr, old_layout);
                }
                new_ptr
            }
        }
    }
}

// ============================================================================
// 全局分配器：bump allocator（静态 1 MiB 缓冲）。
// ============================================================================

struct Bump;

unsafe impl alloc::alloc::GlobalAlloc for Bump {
    unsafe fn alloc(&self, layout: alloc::alloc::Layout) -> *mut u8 {
        const BUF_SIZE: usize = 1 << 20;
        static mut NEXT: usize = 0;
        static mut MEMORY: [u8; BUF_SIZE] = [0u8; BUF_SIZE];
        let base = core::ptr::addr_of_mut!(MEMORY).cast::<u8>();
        let start = (NEXT + layout.align() - 1) & !(layout.align() - 1);
        if start + layout.size() > BUF_SIZE {
            return core::ptr::null_mut();
        }
        NEXT = start + layout.size();
        base.add(start)
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: alloc::alloc::Layout) {}

    unsafe fn realloc(
        &self,
        ptr: *mut u8,
        old: alloc::alloc::Layout,
        new_size: usize,
    ) -> *mut u8 {
        let new_ptr = self.alloc(alloc::alloc::Layout::from_size_align_unchecked(
            new_size,
            old.align(),
        ));
        if !new_ptr.is_null() {
            core::ptr::copy_nonoverlapping(ptr, new_ptr, old.size().min(new_size));
        }
        new_ptr
    }
}

#[global_allocator]
static GLOBAL: Bump = Bump;

// ============================================================================
// 生成的 guest 绑定（navis:ext/guest 世界）。
// ============================================================================

include!("navis_ext_bindings.rs");

// ============================================================================
// Guest 实现：真实组件逻辑（activate 内调用 host log.write / operation.list）。
// ============================================================================

/// Guest trait 实现载体（cabi 导出宏要求具名类型）。
struct GuestImpl;

impl exports::navis::ext::lifecycle::Guest for GuestImpl {
    fn init(_handle: exports::navis::ext::lifecycle::HostHandle) -> Result<(), alloc::string::String> {
        Ok(())
    }

    fn activate() -> Result<(), alloc::string::String> {
        // host operation.list-operations：须返回本扩展已注册的操作（host 侧
        // list_operations 按 capabilities.operation.list + OperationRegistry 过滤）。
        let ops = navis::host::operation::list_operations();
        if ops.is_empty() {
            return Err(alloc::format!(
                "navis_ext.activate: operation.list returned no operations (host gate broken)"
            ));
        }
        // host log.write：受控输出走通即返回 Ok；失败向上传播（宿主 activate 失败）。
        navis::host::log::write(
            navis::host::types::LogLevel::Info,
            &alloc::format!(
                "navis_ext.activate:host-log-ok operations={} first={}",
                ops.len(),
                ops[0].id
            ),
        )
        .map_err(|error| alloc::format!("navis_ext.activate: log.write failed: {error}"))?;
        Ok(())
    }

    fn deactivate() -> Result<(), alloc::string::String> {
        Ok(())
    }
}

impl exports::navis::ext::message::Guest for GuestImpl {
    fn handle(
        payload: exports::navis::ext::message::Value,
    ) -> Result<exports::navis::ext::message::Value, alloc::string::String> {
        // 消息回显（C1-3 message 通路预留）。
        Ok(payload)
    }
}

// 接线 cabi 导出（生成代码的 export! 宏路径与模块结构一致，此处直接使用）。
export!(GuestImpl);
