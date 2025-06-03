#![no_std]
#![no_main]

use esp_hal::clock::CpuClock;
use esp_hal::main;
use esp_hal::gpio::{Level, Output};
use esp_hal::delay::Delay;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[main]
fn main() -> ! {
    // generator version: 0.3.1

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    
    let mut led_blue = Output::new(peripherals.GPIO26, Level::Low);
    let mut led_green = Output::new(peripherals.GPIO27, Level::High);
    let delay = Delay::new();

    loop {
        delay.delay_millis(500);
        led_green.toggle();
        led_blue.toggle();
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0-beta.0/examples/src/bin
}
