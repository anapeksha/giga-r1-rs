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

    let button = Input::new(p.PC13, Pull::Down);
    let mut red = Output::new(p.PI12, Level::High, Speed::Low);
    let mut green = Output::new(p.PJ13, Level::High, Speed::Low);
    let mut blue = Output::new(p.PE3, Level::High, Speed::Low);

    #[cfg(feature = "defmt")]
    defmt::info!("BOOT0 button test ready");

    loop {
        // The onboard RGB LED is active low. Blue means released and green
        // means pressed. Red is reserved as an unmistakable startup/error hue.
        red.set_high();
        if button.is_high() {
            green.set_low();
            blue.set_high();
        } else {
            green.set_high();
            blue.set_low();
        }

        cortex_m::asm::delay(100_000);
    }
}
