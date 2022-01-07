// $ cargo rb ferris
#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use nrf_embassy as _; // global logger + panicking-behavior + memory layout

use defmt::*;
use embassy::executor::Spawner;
use embassy::time::{Delay, Duration, Timer};
use embassy_nrf::gpio::{Level, NoPin, Output, OutputDrive};
use embassy_nrf::{interrupt, spim, Peripherals};
use embedded_graphics::{
    image::{Image, ImageRaw, ImageRawLE},
    pixelcolor::Rgb565,
    prelude::*,
};
use st7735_embassy::{self, ST7735};

#[embassy::main]
async fn main(_spawner: Spawner, p: Peripherals) {
    info!("running!");
    let mut config = spim::Config::default();
    config.frequency = spim::Frequency::M4;
    config.mode = spim::MODE_0;
    let irq = interrupt::take!(SPIM3);
    // let irq = interrupt::take!(SPIM2_SPIS2_SPI2);


    // D24 .. D26 (aka SPI pins)
    // D24 is P0.15 (SPI MISO)
    // D25 is P0.13 (SPI MOSI)
    // D26 is P0.14 (SPI SCK )
    // D11 is P0.06 select/dc
    // D12 is P0.08 rst
    // D13 is P1.09 chip select
    let spim = spim::Spim::new(p.SPI3, irq, p.P0_14, p.P0_15, p.P0_13, config);

    let _cs_pin = Output::new(p.P0_09, Level::Low, OutputDrive::Standard);
    let rst = Output::new(p.P0_06, Level::Low, OutputDrive::Standard);
    let dc = Output::new(p.P0_08, Level::Low, OutputDrive::Standard);

    let mut display = ST7735::new(spim, dc, rst, Default::default(), 160, 128);

    display.init(&mut Delay).await.unwrap();

    display.clear(Rgb565::BLACK).unwrap();

    let image_raw: ImageRawLE<Rgb565> =
        ImageRaw::new(include_bytes!("../../assets/ferris.raw"), 86);
    let image: Image<_> = Image::new(&image_raw, Point::new(34, 24));
    image.draw(&mut display).unwrap();
    display.flush().await.unwrap();

    let mut backlight = Output::new(p.P1_15, Level::High, OutputDrive::Standard);
    loop {
        backlight.set_high();
        info!("{=str}", "hello world");
        Timer::after(Duration::from_millis(700)).await;
        backlight.set_low();
        Timer::after(Duration::from_millis(300)).await;
    }
}