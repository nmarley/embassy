#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

use core::str;

use cyw43_pio::PioSpi;
use embassy_executor::Spawner;
use embassy_net::Stack;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIN_23, PIN_25, PIO0};
use embassy_rp::pio::Pio;
use embassy_rp::uart;
use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};
use static_cell::make_static;

#[embassy_executor::task]
async fn wifi_task(
    runner: cyw43::Runner<'static, Output<'static, PIN_23>, PioSpi<'static, PIN_25, PIO0, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(stack: &'static Stack<cyw43::NetDriver<'static>>) -> ! {
    stack.run().await
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    rtt_init_print!();
    rprintln!("Hello World!");

    let p = embassy_rp::init(Default::default());

    let fw = include_bytes!("../../../../cyw43-firmware/43439A0.bin");
    let clm = include_bytes!("../../../../cyw43-firmware/43439A0_clm.bin");

    // To make flashing faster for development, you may want to flash the firmwares independently
    // at hardcoded addresses, instead of baking them into the program with `include_bytes!`:
    //     probe-rs-cli download 43439A0.bin --format bin --chip RP2040 --base-address 0x10100000
    //     probe-rs-cli download 43439A0_clm.bin --format bin --chip RP2040 --base-address 0x10140000
    //let fw = unsafe { core::slice::from_raw_parts(0x10100000 as *const u8, 224190) };
    //let clm = unsafe { core::slice::from_raw_parts(0x10140000 as *const u8, 4752) };

    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);
    let mut pio = Pio::new(p.PIO0);
    let spi = PioSpi::new(&mut pio.common, pio.sm0, pio.irq0, cs, p.PIN_24, p.PIN_29, p.DMA_CH0);

    let state = make_static!(cyw43::State::new());
    let (_net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
    spawner.spawn(wifi_task(runner)).unwrap();

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    let config = uart::Config::default();
    let mut uart = uart::Uart::new_with_rtscts_blocking(p.UART0, p.PIN_0, p.PIN_1, p.PIN_3, p.PIN_2, config);
    uart.blocking_write("Initializing WiFi scanning:\r\n".as_bytes())
        .unwrap();

    let mut scanner = control.scan().await;
    while let Some(bss) = scanner.next().await {
        if let Ok(ssid_str) = str::from_utf8(&bss.ssid) {
            let hex_bssid = bssid_to_lowerhex(&bss.bssid).await;
            let s1 = match str::from_utf8(&hex_bssid) {
                Ok(val) => val,
                Err(_e) => "n/a",
            };
            // rprintln!("scanned {} == {:x}", ssid_str, bss.bssid);
            rprintln!("SSID: [{}] (len={}), BSSID: [{}]", ssid_str, bss.ssid_len, s1);

            uart.blocking_write("SSID: [".as_bytes()).unwrap();
            if bss.ssid_len == 0 {
                uart.blocking_write("<hidden>".as_bytes()).unwrap();
            } else {
                uart.blocking_write(ssid_str.as_bytes()).unwrap();
            }
            uart.blocking_write("], BSSID: [".as_bytes()).unwrap();
            uart.blocking_write(s1.as_bytes()).unwrap();
            uart.blocking_write("]\r\n".as_bytes()).unwrap();
        }
    }

    // loop {
    //    uart.blocking_write("hello there!\r\n".as_bytes()).unwrap();
    //    cortex_m::asm::delay(128_000_000);
    // }

    rprintln!("Goodbye, world!");
}

const HEX_DIGITS: [char; 16] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
];

async fn bssid_to_lowerhex(input: &[u8; 6]) -> [u8; 12] {
    let mut result: [u8; 12] = [0; 12];

    let mut count = 0;
    for digit in input {
        result[count] = HEX_DIGITS[(digit / 16) as usize] as u8;
        count += 1;

        result[count] = HEX_DIGITS[(digit % 16) as usize] as u8;
        count += 1;
    }

    // str::from_utf8
    result
}
