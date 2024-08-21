//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]

use core::any::Any;

use bsp::entry;
use cortex_m::prelude::_embedded_hal_blocking_spi_Write;
use cortex_m::singleton;
use cortex_m::{delay::Delay, prelude::_embedded_hal_digital_OutputPin};
use defmt::*;
use defmt_rtt as _;
use embedded_hal::digital::OutputPin;
use panic_probe as _;

use rp2040_project_template::font::Font;
use rp2040_project_template::st7789::{self, ColorMode, Rotation, ST7789Display};
use rp2040_project_template::{font, fonts};
use rp_pico::hal::dma::{single_buffer, DMAExt};
use rp_pico::hal::fugit::RateExtU32;
use rp_pico::hal::gpio::bank0::Gpio6;
use rp_pico::hal::spi::SpiDevice;
use rp_pico::hal::typelevel::OptionTNone;
use rp_pico::hal::Spi;
use rp_pico::pac::ppb::SCR;
use rp_pico::{self as bsp, hal};

use bsp::hal::{
    clocks::{init_clocks_and_plls, Clock},
    pac,
    sio::Sio,
    watchdog::Watchdog,
};

const FRAMEBUFFER_SIZE: usize = 57600 * 2;
const FRAMEBUFFER_N_PIXELS: usize = 57600; // 57600 pixes, 5-6-5 RGB
const XOSC_CRYSTAL_FREQ: u32 = 12_000_000; // Typically found in BSP crates
const SCREEN_SIZE: usize = 240;
#[entry]
fn main() -> ! {
    // Get access to device and core peripherals

    info!("Program start");
    let mut peripherals = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(peripherals.WATCHDOG);
    let sio = Sio::new(peripherals.SIO);

    // External high-speed crystal on the pico board is 12Mhz
    let external_xtal_freq_hz = 12_000_000u32;
    let clocks = init_clocks_and_plls(
        external_xtal_freq_hz,
        peripherals.XOSC,
        peripherals.CLOCKS,
        peripherals.PLL_SYS,
        peripherals.PLL_USB,
        &mut peripherals.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();
    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let pins = bsp::Pins::new(
        peripherals.IO_BANK0,
        peripherals.PADS_BANK0,
        sio.gpio_bank0,
        &mut peripherals.RESETS,
    );

    // These are implicitly used by the spi driver if they are in the correct mode
    info!("Initializing SPI");
    let spi_mosi = pins.gpio7.into_function::<hal::gpio::FunctionSpi>();
    let spi_sclk = pins.gpio6.into_function::<hal::gpio::FunctionSpi>();
    let spi = hal::spi::Spi::<_, _, _, 8>::new(peripherals.SPI0, (spi_mosi, spi_sclk));
    let mut spi = spi.init(
        &mut peripherals.RESETS,
        clocks.peripheral_clock.freq(),
        200.MHz(),
        &embedded_hal::spi::MODE_3,
    );
    info!("Initialized SPI");
    info!("Initializing display");
    let dc = pins.gpio16.into_push_pull_output();
    let rst = pins.gpio15.into_push_pull_output();
    let mut display = ST7789Display::init(
        rst,
        dc,
        OptionTNone,
        OptionTNone,
        &mut spi,
        Rotation::Portrait,
        &mut delay,
    );

    let bmp_framebuffer = singleton!(: [u8; FRAMEBUFFER_SIZE] = [0xff; FRAMEBUFFER_SIZE]).unwrap();
    let dma = peripherals.DMA.split(&mut peripherals.RESETS);
    let mut transfer = single_buffer::Config::new(dma.ch0, bmp_framebuffer, spi);

    // Display data
    let mut x: u8 = 0;
    let mut y: u8 = 0;
    let w: u8 = 10;
    let h: u8 = 10;
    let white = split_into_2(rgb(0xff, 0xff, 0xff));
    let black = split_into_2(rgb(0x0, 0x0, 0x0));
    let mut offset: u8 = 0;

    loop {
        let t = transfer.start();
        let (ch, frambuf, to) = t.wait();

        // Update framebuffer
        for i in 0..FRAMEBUFFER_N_PIXELS {
            let screen_x = (i % SCREEN_SIZE) as u8;
            let screen_y = (i / SCREEN_SIZE) as u8;

            if screen_x >= x && screen_x < x + w - 1 && screen_y > y && screen_y < y + h - 1 {
                let color = split_into_2(rgb(screen_x, screen_y, offset));
                frambuf[i * 2] = color.0;
                frambuf[i * 2 + 1] = color.1;
            } else {
                frambuf[i * 2] = black.0;
                frambuf[i * 2 + 1] = black.1;
            }
        }

        // State update
        offset = offset.checked_add(1).unwrap_or(0);

        transfer = single_buffer::Config::new(ch, frambuf, to);
        x = x.checked_add(1).unwrap_or(0);
        y = y.checked_add(8).unwrap_or(0);
        delay.delay_ms(10);
    }
}

fn split_into_2(i: u16) -> (u8, u8) {
    let msb = (i >> 8) as u8;
    let lsb = (i & 0xff) as u8;

    (msb, lsb)
}

fn rgb(r: u8, g: u8, b: u8) -> u16 {
    let br: u16 = Into::<u16>::into(r) >> 3;
    let bg: u16 = Into::<u16>::into(g) >> 2;
    let bb: u16 = Into::<u16>::into(b) >> 3;
    let result: u16 = (br << 11) + (bg << 5) + bb;
    return result;
}
