#![no_std]
#![no_main]

use core::mem::MaybeUninit;

use cortex_m_rt::entry;
#[cfg(feature = "defmt")]
use defmt_rtt as _;
use embassy_stm32::{
    SharedData,
    gpio::{Level, Output, Speed},
    usart::{Config, Uart},
};
use panic_halt as _;

#[allow(unsafe_code)]
#[unsafe(link_section = ".shared_data")]
static SHARED_DATA: MaybeUninit<SharedData> = MaybeUninit::uninit();

#[entry]
fn main() -> ! {
    let p = embassy_stm32::init_primary(embassy_stm32::Config::default(), &SHARED_DATA);

    // Arduino Serial1: D1/PA9 is TX and D0/PB7 is RX.
    let mut uart =
        Uart::new_blocking(p.USART1, p.PB7, p.PA9, Config::default()).expect("valid USART1 config");
    let mut red = Output::new(p.PI12, Level::High, Speed::Low);
    let mut green = Output::new(p.PJ13, Level::High, Speed::Low);
    let mut blue = Output::new(p.PE3, Level::High, Speed::Low);
    let mut expected = 0x31_u8;

    loop {
        let write_result = uart
            .blocking_write(&[expected])
            .and_then(|()| uart.blocking_flush());
        let mut received = None;
        let mut receive_error = false;

        // Avoid hiding a missing RX signal in an infinite blocking read.
        for _ in 0..20_000_000 {
            match embedded_hal_nb::serial::Read::read(&mut uart) {
                Ok(byte) => {
                    received = Some(byte);
                    break;
                }
                Err(nb::Error::WouldBlock) => {}
                Err(nb::Error::Other(_error)) => {
                    receive_error = true;
                    #[cfg(feature = "defmt")]
                    defmt::warn!(
                        "USART1 transient receive error: {}",
                        defmt::Debug2Format(&_error)
                    );
                }
            }
        }
        let passed = write_result.is_ok() && received == Some(expected);

        if passed {
            red.set_high();
            green.set_low();
            blue.set_high();
            #[cfg(feature = "defmt")]
            defmt::info!("USART1 loopback passed: {=u8:#04x}", expected);
        } else if receive_error {
            green.set_high();
            red.set_low();
            blue.set_low();
            #[cfg(feature = "defmt")]
            defmt::error!("USART1 loopback receive error");
        } else if let Some(byte) = received {
            green.set_low();
            red.set_low();
            blue.set_high();
            #[cfg(feature = "defmt")]
            defmt::error!(
                "USART1 wrong byte: sent {=u8:#04x}, received {=u8:#04x}",
                expected,
                byte
            );
        } else {
            green.set_high();
            red.set_low();
            blue.set_high();
            #[cfg(feature = "defmt")]
            defmt::error!("USART1 loopback timed out with no received byte");
        }

        cortex_m::asm::delay(240_000_000);
        green.set_high();
        red.set_high();
        blue.set_low();
        cortex_m::asm::delay(120_000_000);
        blue.set_high();

        expected = expected.wrapping_add(1);
    }
}
