
#![no_std]
#![no_main]

extern crate alloc;

use alloc::{collections::btree_set::BTreeSet, string::{String, ToString}};
use core::cell::RefCell;
use critical_section::Mutex;

use esp_hal::{clock::CpuClock, main, rng::Rng, timer::timg::TimerGroup};
use esp_println::println;
use esp_wifi::{init, wifi};
use ieee80211::{match_frames, mgmt_frame::BeaconFrame};

use core::panic::PanicInfo;

use esp_hal::gpio::{Level, Output};
use esp_hal::delay::Delay;
use esp_hal::i2c::master::I2c;
use esp_hal::time::RateExtU32;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("Panic occurred: {:?}", info);
    loop {}
}


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


    let mut i2c = I2c::new(
        peripherals.I2C1,
        esp_hal::i2c::master::Config::default().with_frequency(400u32.kHz()),
    ).unwrap().with_scl(peripherals.GPIO22).with_sda(peripherals.GPIO21);
    const ADDR : u8 = 0x3c;
    let commands = [
        0xAE, // 显示关闭
        0xD5, 0x80, // 设置显示时钟分频
        0xA8, 0x3F, // 设置多路复用比（64）
        0xD3, 0x00, // 设置显示偏移
        0x40, // 设置显示起始行
        0x8D, 0x14, // 启用电荷泵
        0x20, 0x00, // 设置内存寻址模式（水平）
        0xA1, // 设置段重映射
        0xC8, // 设置 COM 输出扫描方向
        0xDA, 0x12, // 设置 COM 引脚配置
        0x81, 0xCF, // 设置对比度
        0xD9, 0xF1, // 设置预充电周期
        0xDB, 0x40, // 设置 VCOMH 电压
        0xA4, // 显示 GDDRAM 内容
        0xA6, // 设置正常显示（非反转）
        0xAF  // 显示开启
    ];
    delay.delay_millis(500);
    for cmd in commands {
        if let Err(err) = i2c.write(ADDR, &[0, cmd]) {
            panic!("error on command 0x{:02X} : {:?}", cmd, err);
        }
    }

    let settings = [
        0x21, 0, 0x7f, 0x22, 0, 7
    ];

    println!("循环起来");

    loop {
        delay.delay_millis(500);
        led_green.toggle();
        led_blue.toggle();


        for cmd in settings {
            i2c.write(ADDR, &[0, cmd]).unwrap();
        }

        // i2c.write(ADDR, &[0x40, 0, 0x40, 0, 0x40, 255, 0x40, 255, 0x40, 255]).unwrap();

        for x in 2 .. 6 {
            for _ in 0 .. 8 {
                for _ in 0 .. 8 * x {
                    i2c.write(ADDR, &[0x40, 255]).unwrap();
                }
                for _ in 0 .. 8 * x {
                    i2c.write(ADDR, &[0x40, 0]).unwrap();
                }
            }

        }
    }
}
