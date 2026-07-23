#![no_std]
#![no_main]

use core::mem::MaybeUninit;

use cortex_m_rt::entry;
#[cfg(feature = "defmt")]
use defmt_rtt as _;
use embassy_stm32::{
    SharedData,
    gpio::{Level, Output, Speed},
};
use panic_halt as _;

#[allow(unsafe_code)]
#[unsafe(link_section = ".shared_data")]
static SHARED_DATA: MaybeUninit<SharedData> = MaybeUninit::uninit();

#[entry]
fn main() -> ! {
    let p = embassy_stm32::init_primary(embassy_stm32::Config::default(), &SHARED_DATA);
    let mut green = Output::new(p.PJ13, Level::High, Speed::Low);
    let mut heartbeat = 0_u32;

    loop {
        green.set_low();
        #[cfg(feature = "defmt")]
        defmt::info!("GIGA M7 heartbeat {}", heartbeat);
        cortex_m::asm::delay(8_000_000);

        green.set_high();
        cortex_m::asm::delay(8_000_000);
        heartbeat = heartbeat.wrapping_add(1);
    }
}
