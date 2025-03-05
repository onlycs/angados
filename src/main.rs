#![no_std]
#![no_main]
#![feature(allocator_api, alloc_error_handler)]

use core::arch::asm;

pub mod asm;
pub mod print;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[unsafe(no_mangle)]
extern "C" fn abort() -> ! {
    loop {
        unsafe {
            asm!("wfi");
        }
    }
}

#[unsafe(no_mangle)]
extern "C" fn kmain() {}
