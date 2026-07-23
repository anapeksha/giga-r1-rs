#![no_std]
#![no_main]

use core::mem::MaybeUninit;

use cortex_m_rt::entry;
#[cfg(feature = "defmt")]
use defmt_rtt as _;
use embassy_stm32::{
    SharedData,
    adc::{Adc, AdcConfig, Resolution, SampleTime},
    gpio::{Level, Output, Speed},
    rcc::mux::{Adcsel, Persel},
};
use panic_halt as _;

#[allow(unsafe_code)]
#[unsafe(link_section = ".shared_data")]
static SHARED_DATA: MaybeUninit<SharedData> = MaybeUninit::uninit();

#[entry]
fn main() -> ! {
    let mut mcu_config = embassy_stm32::Config::default();
    mcu_config.rcc.mux.persel = Persel::HSI;
    mcu_config.rcc.mux.adcsel = Adcsel::PER;
    let p = embassy_stm32::init_primary(mcu_config, &SHARED_DATA);

    // Arduino A0 is PC4 / ADC1 channel 4.
    let mut red = Output::new(p.PI12, Level::Low, Speed::Low);
    let mut green = Output::new(p.PJ13, Level::High, Speed::Low);
    let mut blue = Output::new(p.PE3, Level::High, Speed::Low);
    let mut adc_config = AdcConfig::default();
    adc_config.resolution = Some(Resolution::BITS16);
    let mut adc = Adc::new_with_config(p.ADC1, adc_config);
    let mut a0 = p.PC4;
    red.set_high();

    loop {
        let mut sum = 0_u32;
        for _ in 0..16 {
            sum += u32::from(adc.blocking_read(&mut a0, SampleTime::CYCLES387_5));
        }
        let sample = (sum / 16) as u16;

        red.set_high();
        green.set_high();
        blue.set_high();
        if sample < 4096 {
            blue.set_low();
        } else if sample > 61_440 {
            green.set_low();
        } else {
            red.set_low();
        }

        #[cfg(feature = "defmt")]
        defmt::info!(
            "A0 ADC1 sample: raw={=u16}, millivolts~={=u32}",
            sample,
            u32::from(sample) * 3300 / 65_535
        );
        cortex_m::asm::delay(120_000_000);
    }
}
