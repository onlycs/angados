#![no_std]
#![no_main]
#![feature(allocator_api, alloc_error_handler, panic_can_unwind)]

use core::{arch::asm, panic};

use uart::Uart;

pub mod asm;
pub mod page;
pub mod print;
pub mod uart;

#[unsafe(no_mangle)]
extern "C" fn eh_personality() {}

#[panic_handler]
fn panic(info: &panic::PanicInfo) -> ! {
    let msg = info.message();

    if let Some(location) = info.location() {
        println!("\nthread panicked at {location}:\n{msg}");
    } else {
        println!("\nthread panicked:\n{msg}");
    }

    if info.can_unwind() {
        unsafe {
            let mut fp: *const usize;
            asm!("mv {}, s0", out(reg) fp);

            for _ in 0..10 {
                if fp.is_null() {
                    break;
                }

                let ret = *fp.add(1);
                println!("Stack frame: {ret:#x}");

                fp = *fp as *const usize;
            }
        }
    }

    abort();
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
extern "C" fn kmain() {
    Uart::new(0x1000_0000).init();
    page::init();

    loop {
        read!();
    }
}
