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
        let frame = [0x55, 0xaa, expected, !expected];
        let mut passed = false;
        let mut receive_errors = 0_u32;

        // A live jumper insertion can look like a partial UART byte. Drain old
        // input and search for a framed message so that one noisy edge cannot
        // produce a misleading wrong-byte indication.
        for _attempt in 0..3 {
            for _ in 0..256 {
                match embedded_hal_nb::serial::Read::read(&mut uart) {
                    Ok(_) => {}
                    Err(nb::Error::WouldBlock) => break,
                    Err(nb::Error::Other(_)) => {
                        receive_errors = receive_errors.saturating_add(1);
                    }
                }
            }

            if uart
                .blocking_write(&frame)
                .and_then(|()| uart.blocking_flush())
                .is_err()
            {
                continue;
            }

            let mut sync_index = 0_u8;
            for _ in 0..10_000_000 {
                match embedded_hal_nb::serial::Read::read(&mut uart) {
                    Ok(byte) => {
                        sync_index = match sync_index {
                            0 if byte == frame[0] => 1,
                            1 if byte == frame[1] => 2,
                            2 if byte == frame[2] => 3,
                            3 if byte == frame[3] => {
                                passed = true;
                                break;
                            }
                            _ if byte == frame[0] => 1,
                            _ => 0,
                        };
                    }
                    Err(nb::Error::WouldBlock) => {}
                    Err(nb::Error::Other(_)) => {
                        receive_errors = receive_errors.saturating_add(1);
                        sync_index = 0;
                    }
                }
            }
            if passed {
                break;
            }
        }

        if passed {
            red.set_high();
            green.set_low();
            blue.set_high();
            #[cfg(feature = "defmt")]
            defmt::info!(
                "USART1 loopback passed: sequence={=u8:#04x}, transient_errors={=u32}",
                expected,
                receive_errors
            );
        } else {
            green.set_high();
            red.set_low();
            blue.set_high();
            #[cfg(feature = "defmt")]
            defmt::error!(
                "USART1 loopback disconnected or timed out: sequence={=u8:#04x}, transient_errors={=u32}",
                expected,
                receive_errors
            );
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
