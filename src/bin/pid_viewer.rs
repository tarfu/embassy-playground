// $ cargo rb frames
#![no_std]
#![no_main]
#![feature(alloc_error_handler)]
#![feature(type_alias_impl_trait)]

use core::fmt::Write;
use core::task::Poll;
use core::mem;
use core::slice;

use embassy::interrupt::InterruptExt;
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::mono_font::ascii::FONT_8X13;
use embedded_graphics::text::Text;
use heapless::String;
use nrf_embassy as _; // global logger + panicking-behavior + memory layout

use defmt::*;

use embassy::channel::signal::Signal;
use embassy::executor::Spawner;
use embassy::time::{Delay, Duration, Timer, Instant};
use embassy::util::Forever;
use embassy_nrf::{
    gpio::{Level, NoPin, Output, OutputDrive},
    interrupt,
    peripherals::{*},
    spim::{self, Spim},
    Peripherals,
};

use nrf_softdevice::ble::central;
use nrf_softdevice::raw;
use nrf_softdevice::Softdevice;

use embedded_graphics::{pixelcolor::Rgb565, prelude::*};
use st7735_embassy::{self, Frame, ST7735IF};

use crate::interrupt::Priority::P2;

const BUF_SIZE: usize = 160 * 128 * 2;
static FRAME_A: Forever<Frame<BUF_SIZE>> = Forever::new();
static FRAME_B: Forever<Frame<BUF_SIZE>> = Forever::new();
static NEXT_FRAME: Forever<Signal<&'static mut Frame<BUF_SIZE>>> = Forever::new();
static READY_FRAME: Forever<Signal<&'static mut Frame<BUF_SIZE>>> = Forever::new();

#[embassy::task]
async fn render(
    spim: Spim<'static, SPI3>,
    dc: Output<'static, P1_02>,
    rst: Output<'static, P1_01>,
    next_frame: &'static Signal<&'static mut Frame<BUF_SIZE>>,
    ready_frame: &'static Signal<&'static mut Frame<BUF_SIZE>>,
) {
    let mut display = ST7735IF::new(spim, dc, rst, Default::default());
    display.init(&mut Delay).await.unwrap();
    let mut frame = ready_frame.wait().await;
    loop {
        next_frame.signal(frame);
        frame = ready_frame.wait().await;
        display.flush_frame(&frame).await.unwrap();
        // info!("flushed frame");
    }
}

#[embassy::task]
async fn blink(mut led :Output<'static, P0_14>){
    loop {
        led.set_high();
        Timer::after(Duration::from_millis(700)).await;
        led.set_low();
        Timer::after(Duration::from_millis(300)).await;
    }
}


#[embassy::task]
async fn blink_con(mut led :Output<'static, P0_15>){
    loop {
        led.set_high();
        Timer::after(Duration::from_millis(500)).await;
        led.set_low();
        Timer::after(Duration::from_millis(300)).await;
    }
}

#[embassy::task]
async fn block(){
    let mut bla: i32 = 0;
    loop {
        bla = bla+1;
        // Timer::at(Instant::from_ticks(0)).await;    
        Timer::at(Instant::MIN).await
    }
}

#[embassy::task]
async fn softdevice_task(sd: &'static Softdevice) {
    sd.run().await;
}

#[embassy::task]
async fn ble_task(sd: &'static Softdevice) {
    let config = {
        let mut config = central::ScanConfig::default();
        config.timeout = 1000;
        config
    };
    let res = central::scan(sd, &config, |params| -> Option<bool> {unsafe {
        info!("AdvReport!");
        info!(
            "type: connectable={} scannable={} directed={} scan_response={} extended_pdu={} status={}",
            params.type_.connectable(),
            params.type_.scannable(),
            params.type_.directed(),
            params.type_.scan_response(),
            params.type_.extended_pdu(),
            params.type_.status()
        );
        info!(
            "addr: resolved={} type={} addr={:x}",
            params.peer_addr.addr_id_peer(),
            params.peer_addr.addr_type(),
            params.peer_addr.addr
        );
        let mut data = slice::from_raw_parts(params.data.p_data, params.data.len as usize);
        while data.len() != 0 {
            let len = data[0] as usize;
            if data.len() < len+1 {
                warn!("Advertisement data truncated?");
                break;
            }
            if len < 1 {
                warn!("Advertisement data malformed?");
                break;
            }
            let key = data[1];
            let value = &data[2..len+1];
            info!("value {}: {:x}", key, value);
            data = &data[len+1..];
        }
        None
        // Some(true)
    }})
    .await;
    // unwrap!(res);
    info!("{}", res);
    info!("Scan returned");
}

#[embassy::main(config = "config()")]
async fn main(spawner: Spawner, p: Peripherals) {

    info!("Hello World!");

    let softdevice_config = nrf_softdevice::Config {
        clock: Some(raw::nrf_clock_lf_cfg_t {
            source: raw::NRF_CLOCK_LF_SRC_RC as u8,
            rc_ctiv: 4,
            rc_temp_ctiv: 2,
            accuracy: 7,
        }),
        conn_gap: Some(raw::ble_gap_conn_cfg_t {
            conn_count: 6,
            event_length: 6,
        }),
        conn_gatt: Some(raw::ble_gatt_conn_cfg_t { att_mtu: 128 }),
        gatts_attr_tab_size: Some(raw::ble_gatts_cfg_attr_tab_size_t {
            attr_tab_size: 32768,
        }),
        gap_role_count: Some(raw::ble_gap_cfg_role_count_t {
            adv_set_count: 1,
            periph_role_count: 3,
            central_role_count: 3,
            central_sec_count: 0,
            _bitfield_1: raw::ble_gap_cfg_role_count_t::new_bitfield_1(0),
        }),
        gap_device_name: Some(raw::ble_gap_cfg_device_name_t {
            p_value: b"HelloRust" as *const u8 as _,
            current_len: 9,
            max_len: 9,
            write_perm: unsafe { mem::zeroed() },
            _bitfield_1: raw::ble_gap_cfg_device_name_t::new_bitfield_1(
                raw::BLE_GATTS_VLOC_STACK as u8,
            ),
        }),
        ..Default::default()
    };

    let sd = Softdevice::enable(&softdevice_config);




    let mut config = spim::Config::default();
    config.frequency = spim::Frequency::M32;
    let irq = interrupt::take!(SPIM3);
    irq.set_priority(P2);
    let spim = spim::Spim::new(p.SPI3, irq, p.P1_05, p.P1_06, p.P1_04, config);

    let _cs_pin = Output::new(p.P1_03, Level::Low, OutputDrive::Standard);
    let dc = Output::new(p.P1_02, Level::High, OutputDrive::Standard);
    let rst = Output::new(p.P1_01, Level::High, OutputDrive::Standard);

    let next_frame = NEXT_FRAME.put(Signal::new());
    let frame_a = FRAME_A.put(Default::default());
    next_frame.signal(frame_a);

    let ready_frame = READY_FRAME.put(Signal::new());
    let frame_b = FRAME_B.put(Default::default());
    ready_frame.signal(frame_b);

    let led1 = Output::new(p.P0_14, Level::High, OutputDrive::Standard);
    let led_con = Output::new(p.P0_15, Level::High, OutputDrive::Standard);



    defmt::unwrap!(spawner.spawn(render(spim, dc, rst, next_frame, ready_frame)));
    defmt::unwrap!(spawner.spawn(blink(led1)));
    defmt::unwrap!(spawner.spawn(blink_con(led_con)));
    defmt::unwrap!(spawner.spawn(block()));
    defmt::unwrap!(spawner.spawn(softdevice_task(sd)));
    defmt::unwrap!(spawner.spawn(ble_task(sd)));



    let _backlight = Output::new(p.P0_13, Level::High, OutputDrive::Standard);

    let mut x: u16 = 0;
    let mut y: u16 = 0;
    let mut run: i32 = 0;

    let text_style = MonoTextStyleBuilder::new()
    .font(&FONT_8X13)
    .text_color(Rgb565::BLUE)
    .background_color(Rgb565::WHITE)
    .build();

    let mut buffer: heapless::String<64> = heapless::String::new();


    loop {

        let frame = next_frame.wait().await;

        frame.clear(Rgb565::BLACK).unwrap();

        buffer = String::from(run);
        Text::new(&buffer, Point::new(x as i32, y as i32), text_style).draw(frame).unwrap();
        run = run + 1;


        
        frame.set_pixel(x, y, Rgb565::GREEN);
        ready_frame.signal(frame);
        x = (x + 1) % 160;
        y = (y + 1) % 128;
        Timer::after(Duration::from_millis(0)).await;
    }
}

fn config() -> embassy_nrf::config::Config {
    let mut config = embassy_nrf::config::Config::default();
    config.hfclk_source = embassy_nrf::config::HfclkSource::Internal;
    config.gpiote_interrupt_priority = P2;
    config.time_interrupt_priority = P2;
    config
}

#[inline]
async fn deblock() -> Option<bool> {
    return None;
}