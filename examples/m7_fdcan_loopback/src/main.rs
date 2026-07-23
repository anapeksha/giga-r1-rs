#![no_std]
#![no_main]

use core::mem::MaybeUninit;

#[cfg(feature = "defmt")]
use defmt_rtt as _;
use embassy_stm32::{
    SharedData, bind_interrupts,
    can::{
        CanConfigurator, Frame, IT0InterruptHandler, IT1InterruptHandler,
        frame::{FdFrame, Header},
    },
    gpio::{Level, Output, Speed},
    rcc::{Hse, HseMode},
    time::Hertz,
};
use panic_halt as _;

bind_interrupts!(struct Irqs {
    FDCAN2_IT0 => IT0InterruptHandler<embassy_stm32::peripherals::FDCAN2>;
    FDCAN2_IT1 => IT1InterruptHandler<embassy_stm32::peripherals::FDCAN2>;
});

#[allow(unsafe_code)]
#[unsafe(link_section = ".shared_data")]
static SHARED_DATA: MaybeUninit<SharedData> = MaybeUninit::uninit();

#[embassy_executor::main]
async fn main(_spawner: embassy_executor::Spawner) {
    let mut mcu_config = embassy_stm32::Config::default();
    mcu_config.rcc.hse = Some(Hse {
        freq: Hertz::mhz(16),
        mode: HseMode::Oscillator,
    });
    let p = embassy_stm32::init_primary(mcu_config, &SHARED_DATA);

    let mut red = Output::new(p.PI12, Level::High, Speed::Low);
    let mut green = Output::new(p.PJ13, Level::High, Speed::Low);
    let mut blue = Output::new(p.PE3, Level::High, Speed::Low);

    // GIGA CAN0 is FDCAN2 on PB5 (RX) and PB13 (TX). Internal loopback
    // disconnects RX and holds TX recessive, so no transceiver is required.
    let mut configurator = CanConfigurator::new(p.FDCAN2, p.PB5, p.PB13, Irqs);
    configurator.set_bitrate(500_000);
    configurator.set_fd_data_bitrate(1_000_000, true);
    let mut can = configurator.into_internal_loopback_mode();
    let mut sequence = 0_u8;

    loop {
        let payload = [0x47, 0x49, 0x47, 0x41, sequence];
        let frame = Frame::new_standard(0x321, &payload).unwrap();
        let _replaced = can.write(&frame).await;

        let classic_passed = match can.read().await {
            Ok(envelope) => {
                envelope.frame.id() == frame.id() && envelope.frame.data() == payload
            }
            Err(_) => false,
        };

        let fd_payload = [
            0x46, 0x44, 0x43, 0x41, 0x4e, sequence, !sequence, 0x00, 0x08, 0x10, 0x18, 0x20, 0x28,
            0x30, 0x38, 0x40,
        ];
        let fd_header = Header::new_fd(*frame.id(), fd_payload.len() as u8, false, true);
        let fd_frame = FdFrame::new(fd_header, &fd_payload).unwrap();
        let _replaced = can.write_fd(&fd_frame).await;
        let fd_passed = match can.read_fd().await {
            Ok(envelope) => {
                envelope.frame.id() == fd_frame.id() && envelope.frame.data() == fd_payload
            }
            Err(_) => false,
        };
        let passed = classic_passed && fd_passed;

        blue.set_high();
        if passed {
            red.set_high();
            green.set_low();
            #[cfg(feature = "defmt")]
            defmt::info!(
                "FDCAN2 classic+FD loopback passed: sequence={=u8}",
                sequence
            );
        } else {
            green.set_high();
            red.set_low();
            #[cfg(feature = "defmt")]
            defmt::error!(
                "FDCAN2 loopback failed: sequence={=u8}, classic={}, fd={}",
                sequence,
                classic_passed,
                fd_passed
            );
        }

        sequence = sequence.wrapping_add(1);
        cortex_m::asm::delay(120_000_000);
        red.set_high();
        green.set_high();
        blue.set_low();
        cortex_m::asm::delay(60_000_000);
        blue.set_high();
    }
}
