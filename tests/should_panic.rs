#![no_std]
#![no_main]

use core::panic::PanicInfo;
use t_os::{exit_qemu, serial_print, serial_println, QemuExitCode};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    should_fail();
    serial_println!("\x1b[091m[TEST DID NOT PANICKED]\x1b[0");
    exit_qemu(QemuExitCode::Failed);
    loop {}
}

fn should_fail() {
    serial_print!("\x1b[093m {: <50} \x1b[0m\t", "should_panic::should_fail");
    assert_eq!(0, 1);
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    serial_println!("\x1b[092m[OK]\x1b[0m");
    exit_qemu(QemuExitCode::Success);
    loop {}
}
