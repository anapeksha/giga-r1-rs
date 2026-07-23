#![no_std]
#![no_main]

use core::mem::MaybeUninit;

use cortex_m_rt::entry;
#[cfg(feature = "defmt")]
use defmt_rtt as _;
use embassy_stm32::{SharedData, gpio};
use panic_halt as _;

// Embassy uses this fixed D3 SRAM object to publish clock information to the
// M4. Every M7/M4 image uses the same address through its linker script.
#[allow(unsafe_code)]
#[unsafe(link_section = ".shared_data")]
static SHARED_DATA: MaybeUninit<SharedData> = MaybeUninit::uninit();

#[entry]
fn main() -> ! {
    #[cfg(feature = "defmt")]
    defmt::info!("GIGA M7 application entry reached");

    // Start from the internal 64 MHz HSI. This intentionally isolates GPIO,
    // linker, reset, and probe behavior from the board's high-speed clock tree.
    let p = embassy_stm32::init_primary(embassy_stm32::Config::default(), &SHARED_DATA);
    let mut red = gpio::Output::new(p.PI12, gpio::Level::High, gpio::Speed::Low);
    let mut green = gpio::Output::new(p.PJ13, gpio::Level::High, gpio::Speed::Low);
    let mut blue = gpio::Output::new(p.PE3, gpio::Level::High, gpio::Speed::Low);

    #[cfg(feature = "defmt")]
    defmt::info!("GIGA M7 Embassy initialization completed");

    loop {
        red.set_low();
        cortex_m::asm::delay(16_000_000);
        red.set_high();

        green.set_low();
        cortex_m::asm::delay(16_000_000);
        green.set_high();

        blue.set_low();
        cortex_m::asm::delay(16_000_000);
        blue.set_high();
    }
}
