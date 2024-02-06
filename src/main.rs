#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(t_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;
use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use t_os::{
    allocator,
    memory::{self, BootInfoFrameAllocator},
    println,
    task::{executor::Executor, keyboard, Task},
    vga_buffer::{change_color, reset_color, Color, ColorCode},
};
use x86_64::VirtAddr;

entry_point!(kernel_main);

#[no_mangle]
fn kernel_main(_boot_info: &'static BootInfo) -> ! {
    t_os::init();
    let phys_mem_offset = VirtAddr::new(_boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&_boot_info.memory_map) };
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");

    let mut executor = Executor::new();
    executor.spawn(Task::new(infinite_loop()));
    executor.spawn(Task::new(keyboard::print_keypresses()));

    let mut new_executor = Executor::new();
    new_executor.spawn(Task::new(example_task()));

    change_color(ColorCode::new(Color::Green, Color::Black));
    println!("OS Loaded Successfully !");
    reset_color();
    executor.run();
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    t_os::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    t_os::test_panic_handler(info)
}

async fn example_task() {
    println!("async number: {}", 42);
}

async fn infinite_loop() {
    for _ in 0..10 {
        println!("LOOPING");
    }
}

#[test_case]
fn test_print() {
    println!("TEST");
}
