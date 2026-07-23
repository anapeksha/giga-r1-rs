#![no_std]
#![no_main]

use core::mem::MaybeUninit;

use cortex_m_rt::entry;
#[cfg(feature = "defmt")]
use defmt_rtt as _;
use embassy_stm32::{
    SharedData,
    gpio::{Level, Output, Pull, Speed},
    spi::{Config, Spi},
    time::Hertz,
};
use panic_halt as _;

#[allow(unsafe_code)]
#[unsafe(link_section = ".shared_data")]
static SHARED_DATA: MaybeUninit<SharedData> = MaybeUninit::uninit();

#[entry]
fn main() -> ! {
    let p = embassy_stm32::init_primary(embassy_stm32::Config::default(), &SHARED_DATA);

    // Arduino SPI header: D13/PH6 SCK, D11/PJ10 MOSI, D12/PJ11 MISO.
    // Connect D11 to D12. D10/PK1 is held low as the conventional chip select.
    #[allow(clippy::field_reassign_with_default)]
    let mut config = Config::default();
    config.frequency = Hertz(1_000_000);
    config.miso_pull = Pull::Down;
    let mut spi = Spi::new_blocking(p.SPI5, p.PH6, p.PJ10, p.PJ11, config);
    let _chip_select = Output::new(p.PK1, Level::Low, Speed::Low);

    let mut red = Output::new(p.PI12, Level::High, Speed::Low);
    let mut green = Output::new(p.PJ13, Level::High, Speed::Low);
    let mut blue = Output::new(p.PE3, Level::High, Speed::Low);
    let mut sequence = 0_u8;

    loop {
        let transmitted = [0x55, 0xaa, sequence, !sequence];
        let mut received = [0_u8; 4];
        let passed =
            spi.blocking_transfer(&mut received, &transmitted).is_ok() && received == transmitted;

        blue.set_high();
        if passed {
            red.set_high();
            green.set_low();
            #[cfg(feature = "defmt")]
            defmt::info!(
                "SPI5 loopback passed: sequence={=u8:#04x}, received={=[u8; 4]:#04x}",
                sequence,
                received
            );
        } else {
            green.set_high();
            red.set_low();
            #[cfg(feature = "defmt")]
            defmt::error!(
                "SPI5 loopback failed: sent={=[u8; 4]:#04x}, received={=[u8; 4]:#04x}",
                transmitted,
                received
            );
        }

        cortex_m::asm::delay(240_000_000);
        red.set_high();
        green.set_high();
        blue.set_low();
        cortex_m::asm::delay(120_000_000);
        blue.set_high();

        sequence = sequence.wrapping_add(1);
    }
}
