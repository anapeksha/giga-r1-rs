#![no_std]
#![no_main]

use core::mem::MaybeUninit;

use cortex_m_rt::entry;
#[cfg(feature = "defmt")]
use defmt_rtt as _;
use embassy_stm32::{
    SharedData,
    gpio::{Input, Level, Output, Pull, Speed},
};
use panic_halt as _;

#[allow(unsafe_code)]
#[unsafe(link_section = ".shared_data")]
static SHARED_DATA: MaybeUninit<SharedData> = MaybeUninit::uninit();

#[entry]
fn main() -> ! {
    let p = embassy_stm32::init_primary(embassy_stm32::Config::default(), &SHARED_DATA);

    // Plain GPIO continuity test: connect Arduino D1/TX0 to D0/RX0.
    let mut d1 = Output::new(p.PA9, Level::Low, Speed::Low);
    let d0 = Input::new(p.PB7, Pull::Down);
    let mut red = Output::new(p.PI12, Level::High, Speed::Low);
    let mut green = Output::new(p.PJ13, Level::High, Speed::Low);
    let mut blue = Output::new(p.PE3, Level::High, Speed::Low);

    loop {
        d1.set_low();
        cortex_m::asm::delay(100_000);
        let low_ok = d0.is_low();

        d1.set_high();
        cortex_m::asm::delay(100_000);
        let high_ok = d0.is_high();

        if low_ok && high_ok {
            red.set_high();
            green.set_low();
            #[cfg(feature = "defmt")]
            defmt::info!("D1 -> D0 GPIO continuity passed");
        } else {
            green.set_high();
            red.set_low();
            #[cfg(feature = "defmt")]
            defmt::error!(
                "D1 -> D0 GPIO continuity failed: low={}, high={}",
                low_ok,
                high_ok
            );
        }
        blue.set_high();
        cortex_m::asm::delay(2_000_000);

        red.set_high();
        green.set_high();
        blue.set_low();
        cortex_m::asm::delay(500_000);
    }
}
