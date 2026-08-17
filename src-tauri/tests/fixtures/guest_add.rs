//! 硬门验证用最小 WASM 组件 guest（无 import、仅导出 `add(i32, i32) -> i32`）。
//!
//! 用于证明 wasmtime 组件模型在 Windows 可加载并调用；非业务组件。
//! 重新生成：
//!   rustc --target wasm32-unknown-unknown --crate-type cdylib -O guest_add.rs -o guest_add.wasm
//!   wasm-tools component new guest_add.wasm -o component_add.wasm

#![no_std]
#![no_main]

use core::arch::wasm32;
use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    wasm32::unreachable()
}

#[no_mangle]
pub extern "C" fn add(a: i32, b: i32) -> i32 {
    a + b
}
