
#![no_std]
#![no_main]

extern crate alloc;

use alloc::{collections::btree_set::BTreeSet, string::{String, ToString}, vec};
use core::cell::RefCell;
use critical_section::Mutex;

use esp_hal::{clock::CpuClock, main, rng::Rng, timer::timg::TimerGroup, Blocking, DriverMode};
use esp_println::println;
use esp_wifi::{init, wifi};
use ieee80211::{match_frames, mgmt_frame::BeaconFrame};

use core::panic::PanicInfo;

use esp_hal::gpio::{Level, Output};
use esp_hal::delay::Delay;

use esp_hal::i2c::master::Config as I2cConfig;
use esp_hal::i2c::master::ConfigError as I2cConfigError;
use esp_hal::i2c::master::I2c;
use esp_hal::peripherals::I2C0;
use esp_hal::time::RateExtU32;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("Panic occurred: {:?}", info);
    loop {}
}

// esp_bootloader_esp_idf::esp_app_desc!();

static KNOWN_SSIDS: Mutex<RefCell<BTreeSet<String>>> = Mutex::new(RefCell::new(BTreeSet::new()));

#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    let mut led_blue = Output::new(peripherals.GPIO26, Level::Low);
    let mut led_green = Output::new(peripherals.GPIO27, Level::High);
    let delay = Delay::new();

    let i2c_config = I2cConfig::default().with_frequency(25_u32.kHz());
    let mut i2c = I2c::new(peripherals.I2C1, i2c_config)
        .unwrap()
        .with_sda(peripherals.GPIO21)
        .with_scl(peripherals.GPIO22);

    esp_alloc::heap_allocator!(72 * 1024);

    let timer_group_0 = TimerGroup::new(peripherals.TIMG0);
    let esp_wifi_ctrl = init(
        timer_group_0.timer0,
        Rng::new(peripherals.RNG),
        peripherals.RADIO_CLK,
    )
    .unwrap();

    // We must initialize some kind of interface and start it.
    let (_controller, interfaces) =
        wifi::new_with_mode(&esp_wifi_ctrl, peripherals.WIFI, wifi::WifiStaDevice).unwrap();


    let mut sniffer = interfaces.take_sniffer().unwrap();
    sniffer.set_promiscuous_mode(true).unwrap();
    sniffer.set_receive_cb(|packet| {
        let _ = match_frames! {
            packet.data,
            beacon = BeaconFrame => {
                let Some(ssid) = beacon.ssid() else {
                    return;
                };
                if critical_section::with(|cs| {
                    KNOWN_SSIDS.borrow_ref_mut(cs).insert(ssid.to_string())
                }) {
                    println!("Found new AP with SSID: {ssid}");
                }
            }
        };
    });

    init_oled(&mut i2c);
    write_oled(&mut i2c, &vec![255; 32]);

    println!("循环起来");


    loop {
        delay.delay_millis(500);
        led_green.toggle();
        led_blue.toggle();
    }
}

fn init_oled(i2c: &mut I2c<Blocking>) {
    let commands = [
        0xAE,        // Display OFF
        0x20, 0x00,  // Set Memory Addressing Mode (Horizontal)
        0xB0,        // Set Page Start Address
        0xC8,        // Set COM Output Scan Direction
        0x00,        // Set low column address
        0x10,        // Set high column address
        0x40,        // Set start line address
        0x81, 0xFF,  // Set contrast control register
        0xA1,        // Set segment re-map 0 to 127
        0xA6,        // Set normal display
        0xA8, 0x3F,  // Set multiplex ratio(1 to 64)
        0xA4,        // Output RAM to Display
        0xD3, 0x00,  // Set display offset
        0xD5, 0xF0,  // Set display clock divide ratio/oscillator frequency
        0xD9, 0x22,  // Set pre-charge period
        0xDA, 0x12,  // Set com pins hardware configuration
        0xDB, 0x20,  // Set vcomh
        0x8D, 0x14,  // Set DC-DC enable
        0xAF         // Display ON
    ];
    for &cmd in commands.iter() {
        i2c.write(0x3cu8, &[0, cmd]).unwrap();
    }
}

fn write_oled(i2c: &mut I2c<Blocking>, data: &[u8]) {
    i2c.write(0x3cu8, &[0, 0, 0, 0xb0, 0, 0, 0, 0x10]).unwrap();
    for &byte in data {
        i2c.write(0x3cu8, &[32, byte]).unwrap();
    }
}

