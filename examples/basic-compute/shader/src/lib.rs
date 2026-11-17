#![no_std]

use core::panic::PanicInfo;

#[panic_handler]
fn panic_handler(_: &PanicInfo) -> ! {
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn bla() -> i32 {
    let i = 42;
    let bla = (i - 2) * 5 + 2;
    bla
}
