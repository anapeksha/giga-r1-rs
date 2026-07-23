#![no_std]
#![no_main]

use core::mem::MaybeUninit;

use cortex_m_rt::entry;
#[cfg(feature = "defmt")]
use defmt_rtt as _;
use embassy_stm32::{
    SharedData,
    gpio::{Input, Level, Output, OutputType, Pull, Speed},
    time::Hertz,
    timer::{
        Ch3,
        low_level::CountingMode,
        simple_pwm::{PwmPin, SimplePwm},
    },
};
use panic_halt as _;

#[allow(unsafe_code)]
#[unsafe(link_section = ".shared_data")]
static SHARED_DATA: MaybeUninit<SharedData> = MaybeUninit::uninit();

fn count_high(input: &Input<'_>) -> u16 {
    let mut high_samples = 0_u16;
    for _ in 0..256 {
        if input.is_high() {
            high_samples += 1;
        }
        // Deliberately not an integer fraction of the PWM period.
        cortex_m::asm::delay(997);
    }
    high_samples
}

#[entry]
fn main() -> ! {
    let p = embassy_stm32::init_primary(embassy_stm32::Config::default(), &SHARED_DATA);

    // Arduino D3 is PA2 / TIM2_CH3. Connect it to D2 / PA3.
    let pwm_pin = PwmPin::<_, Ch3>::new(p.PA2, OutputType::PushPull);
    let pwm = SimplePwm::new(
        p.TIM2,
        None,
        None,
        Some(pwm_pin),
        None,
        Hertz(1_000),
        CountingMode::EdgeAlignedUp,
    );
    let mut pwm_channel = pwm.split().ch3;
    pwm_channel.enable();
    let input = Input::new(p.PA3, Pull::Down);

    let mut red = Output::new(p.PI12, Level::High, Speed::Low);
    let mut green = Output::new(p.PJ13, Level::High, Speed::Low);
    let mut blue = Output::new(p.PE3, Level::High, Speed::Low);

    loop {
        pwm_channel.set_duty_cycle_percent(25);
        cortex_m::asm::delay(100_000);
        let quarter_high = count_high(&input);

        pwm_channel.set_duty_cycle_percent(75);
        cortex_m::asm::delay(100_000);
        let three_quarters_high = count_high(&input);

        let passed =
            (32..=96).contains(&quarter_high) && (160..=224).contains(&three_quarters_high);

        blue.set_high();
        if passed {
            red.set_high();
            green.set_low();
            #[cfg(feature = "defmt")]
            defmt::info!(
                "TIM2 PWM loopback passed: duty25_high={=u16}/256, duty75_high={=u16}/256",
                quarter_high,
                three_quarters_high
            );
        } else {
            green.set_high();
            red.set_low();
            #[cfg(feature = "defmt")]
            defmt::error!(
                "TIM2 PWM loopback failed: duty25_high={=u16}/256, duty75_high={=u16}/256",
                quarter_high,
                three_quarters_high
            );
        }

        cortex_m::asm::delay(120_000_000);
        red.set_high();
        green.set_high();
        blue.set_low();
        cortex_m::asm::delay(60_000_000);
        blue.set_high();
    }
}
