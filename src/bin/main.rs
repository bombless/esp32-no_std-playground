
#![no_std]
#![no_main]

extern crate alloc;

use alloc::{
    collections::btree_set::BTreeSet,
    string::{String, ToString},
};
use core::cell::RefCell;
use critical_section::Mutex;

use esp_hal::{clock::CpuClock, main, rng::Rng, timer::timg::TimerGroup};
use esp_println::println;
use esp_wifi::{init, wifi};
use ieee80211::{match_frames, mgmt_frame::BeaconFrame};

use core::panic::PanicInfo;

use esp_hal::gpio::{Level, Output};
use esp_hal::delay::Delay;

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

    println!("循环起来");

    loop {
        delay.delay_millis(500);
        led_green.toggle();
        led_blue.toggle();
    }
}
