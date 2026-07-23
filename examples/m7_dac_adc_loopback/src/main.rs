#![no_std]
#![no_main]

use core::mem::MaybeUninit;

use cortex_m_rt::entry;
#[cfg(feature = "defmt")]
use defmt_rtt as _;
use embassy_stm32::{
    SharedData,
    adc::{Adc, AdcConfig, Resolution, SampleTime},
    dac::{DacChannel, Value},
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

    let mut red = Output::new(p.PI12, Level::High, Speed::Low);
    let mut green = Output::new(p.PJ13, Level::High, Speed::Low);
    let mut blue = Output::new(p.PE3, Level::High, Speed::Low);

    // Arduino A12 is PA4 / DAC1 channel 1; A0 is PC4 / ADC1 channel 4.
    let mut dac = DacChannel::new_blocking(p.DAC1, p.PA4);
    let adc_config = AdcConfig {
        resolution: Some(Resolution::BITS16),
        ..Default::default()
    };
    let mut adc = Adc::new_with_config(p.ADC1, adc_config);
    let mut a0 = p.PC4;

    const TEST_VALUES: [u16; 3] = [0x0400, 0x0800, 0x0c00];
    const TOLERANCE: u16 = 4_000;

    loop {
        let mut passed = true;

        for output in TEST_VALUES {
            dac.set(Value::Bit12Right(output));
            cortex_m::asm::delay(2_000_000);

            let mut sum = 0_u32;
            for _ in 0..16 {
                sum += u32::from(adc.blocking_read(&mut a0, SampleTime::CYCLES387_5));
            }
            let measured = (sum / 16) as u16;
            let expected = output << 4;
            let error = measured.abs_diff(expected);
            passed &= error <= TOLERANCE;

            #[cfg(feature = "defmt")]
            defmt::info!(
                "DAC A12 -> ADC A0: dac12={=u16}, expected16={=u16}, measured16={=u16}, error={=u16}",
                output,
                expected,
                measured,
                error
            );
        }

        blue.set_high();
        if passed {
            red.set_high();
            green.set_low();
        } else {
            green.set_high();
            red.set_low();
        }
        cortex_m::asm::delay(120_000_000);

        red.set_high();
        green.set_high();
        blue.set_low();
        cortex_m::asm::delay(60_000_000);
        blue.set_high();
    }
}
