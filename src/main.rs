//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]

use core::any::Any;

use bsp::entry;
use cortex_m::prelude::_embedded_hal_blocking_spi_Write;
use cortex_m::{delay::Delay, prelude::_embedded_hal_digital_OutputPin};
use defmt::*;
use defmt_rtt as _;
use embedded_hal::digital::OutputPin;
use panic_probe as _;

use rp2040_project_template::font::Font;
use rp2040_project_template::st7789::{self, ColorMode, Rotation, ST7789Display};
use rp2040_project_template::{font, fonts};
use rp_pico::hal::fugit::RateExtU32;
use rp_pico::hal::gpio::bank0::Gpio6;
use rp_pico::hal::spi::SpiDevice;
use rp_pico::hal::typelevel::OptionTNone;
use rp_pico::hal::Spi;
use rp_pico::{self as bsp, hal};

use bsp::hal::{
    clocks::{init_clocks_and_plls, Clock},
    pac,
    sio::Sio,
    watchdog::Watchdog,
};

const XOSC_CRYSTAL_FREQ: u32 = 12_000_000; // Typically found in BSP crates
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
    let spi = spi.init(
        &mut peripherals.RESETS,
        clocks.peripheral_clock.freq(),
        100.MHz(),
        &embedded_hal::spi::MODE_3,
    );
    info!("Initialized SPI");
    info!("Initializing display");
    let dc = pins.gpio16.into_push_pull_output();
    let rst = pins.gpio14.into_push_pull_output();
    let mut display = ST7789Display::new(
        rst,
        dc,
        OptionTNone,
        OptionTNone,
        spi,
        240,
        240,
        Rotation::Portrait,
        &mut delay,
    );

    let mut x = 0;
    let mut y = 0;
    let width = 10;
    let height = 10;
    let mut color = 0x0000;

    loop {
        update_location_and_colour(&mut x, &mut y, &mut color);
        display.draw_solid_rect(x, y, width, height, color);
        delay.delay_ms(40);
    }
}

fn update_location_and_colour(x: &mut u16, y: &mut u16, color: &mut u16) {
    *x = x.checked_add(1).unwrap_or(0) % 230;
    *y = y.checked_add(8).unwrap_or(0) % 230;

    *color = color.checked_add(127).unwrap_or(0);
}
