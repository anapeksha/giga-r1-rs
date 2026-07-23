#![no_std]
#![no_main]

use core::mem::MaybeUninit;

use cortex_m_rt::entry;
#[cfg(feature = "defmt")]
use defmt_rtt as _;
use embassy_stm32::{
    SharedData,
    adc::{Adc, AdcConfig, Resolution, SampleTime},
    gpio::{Input, Level, Output, Pull, Speed},
    rcc::mux::{Adcsel, Persel},
};
use embedded_hal::digital::{InputPin, OutputPin};
use giga_r1::led::{Color, RgbLed};
use panic_halt as _;

#[allow(unsafe_code)]
#[unsafe(link_section = ".shared_data")]
static SHARED_DATA: MaybeUninit<SharedData> = MaybeUninit::uninit();

#[entry]
fn main() -> ! {
    let mut config = embassy_stm32::Config::default();
    config.rcc.mux.persel = Persel::HSI;
    config.rcc.mux.adcsel = Adcsel::PER;
    let p = embassy_stm32::init_primary(config, &SHARED_DATA);
    let board = giga_r1::arduino_giga_pins!(p);

    let mut d13 = board
        .digital
        .d13
        .map(|pin| Output::new(pin, Level::Low, Speed::Low));
    let mut d7 = board.digital.d7.map(|pin| Input::new(pin, Pull::Up));
    let mut led = RgbLed::new(
        board
            .led_red
            .map(|pin| Output::new(pin, Level::High, Speed::Low)),
        board
            .led_green
            .map(|pin| Output::new(pin, Level::High, Speed::Low)),
        board
            .led_blue
            .map(|pin| Output::new(pin, Level::High, Speed::Low)),
    )
    .unwrap();
    let mut a0 = board.analog.a0.into_inner();
    let mut adc = Adc::new_with_config(
        p.ADC1,
        AdcConfig {
            resolution: Some(Resolution::BITS16),
            ..Default::default()
        },
    );

    loop {
        let sample = adc.blocking_read(&mut a0, SampleTime::CYCLES387_5);
        let pressed = d7.is_low().unwrap_or(false);

        let a0_low = sample < 8_192;
        let color = match (a0_low, pressed) {
            (true, true) => Color::Yellow,
            (true, false) => Color::Green,
            (false, true) => Color::Red,
            (false, false) => Color::Blue,
        };
        led.set(color).unwrap();
        if a0_low || pressed {
            d13.set_high().ok();
        } else {
            d13.set_low().ok();
        }

        #[cfg(feature = "defmt")]
        defmt::info!(
            "Arduino A0={=u16}, A0 low={}, D7 pressed={}, color={}",
            sample,
            a0_low,
            pressed,
            color,
        );
        cortex_m::asm::delay(8_000_000);
    }
}
