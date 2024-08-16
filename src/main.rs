//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]

use core::any::Any;

use bsp::entry;
use cortex_m::{delay::Delay, prelude::_embedded_hal_digital_OutputPin};
use defmt::*;
use defmt_rtt as _;
use display_interface_spi::SPIInterface;
use embedded_graphics_core::{pixelcolor::Rgb666, prelude::WebColors};
use embedded_hal::digital::OutputPin;
use mipidsi::{models::ST7789, Builder};
use panic_probe as _;

use rp_pico::hal::fugit::RateExtU32;
use rp_pico::hal::gpio::bank0::Gpio6;
use rp_pico::hal::spi::SpiDevice;
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
    let spi_miso = pins.gpio4.into_function::<hal::gpio::FunctionSpi>();
    let spi_sclk = pins.gpio6.into_function::<hal::gpio::FunctionSpi>();
    let spi = hal::spi::Spi::<_, _, _, 8>::new(peripherals.SPI0, (spi_mosi, spi_miso, spi_sclk));
    let spi = spi.init(
        &mut peripherals.RESETS,
        clocks.peripheral_clock.freq(),
        1_000_000u32.Hz(),
        &embedded_hal::spi::MODE_0,
    );
    info!("Initialized SPI");
    let dc = pins.gpio16;
    let rst = pins.gpio14;
    let di = SPIInterface::new(spi, dc);

    // create the ILI9486 display driver in rgb666 color mode from the display interface and use a HW reset pin during init
    let mut display = Builder::new(ST7789, spi).reset_pin(rst).init(&mut delay)?; // delay provider from your MCU
                                                                                  // clear the display to black
    display.clear(Rgb666::CSS_RED)?;

    let mut led_pin = pins.gpio15.into_push_pull_output();
    loop {
        info!("on!");
        led_pin.set_high().unwrap();
        delay.delay_ms(500);

        info!("off!");
        led_pin.set_low().unwrap();
        delay.delay_ms(500);
    }
}
