#![no_std]
#![no_main]

use core::mem::MaybeUninit;

use cortex_m_rt::entry;
#[cfg(feature = "defmt")]
use defmt_rtt as _;
use embassy_stm32::{
    SharedData,
    gpio::{Level, Output, Speed},
    qspi::{
        Config, Qspi, TransferConfig,
        enums::{DummyCycles, FIFOThresholdLevel, MemorySize, QspiWidth},
    },
};
use panic_halt as _;

#[allow(unsafe_code)]
#[unsafe(link_section = ".shared_data")]
static SHARED_DATA: MaybeUninit<SharedData> = MaybeUninit::uninit();

#[entry]
fn main() -> ! {
    let p = embassy_stm32::init_primary(embassy_stm32::Config::default(), &SHARED_DATA);

    let mut red = Output::new(p.PI12, Level::High, Speed::Low);
    let mut green = Output::new(p.PJ13, Level::High, Speed::Low);
    let mut blue = Output::new(p.PE3, Level::High, Speed::Low);

    #[allow(clippy::field_reassign_with_default)]
    let mut config = Config::default();
    config.memory_size = MemorySize::_16MiB;
    config.prescaler = 3;
    config.fifo_threshold = FIFOThresholdLevel::_1Bytes;
    let mut qspi = Qspi::new_blocking_bank1(
        p.QUADSPI, p.PD11, p.PD12, p.PE2, p.PF6, p.PF10, p.PG6, config,
    );

    loop {
        let mut jedec = [0_u8; 3];
        qspi.blocking_read(
            &mut jedec,
            TransferConfig {
                iwidth: QspiWidth::SING,
                dwidth: QspiWidth::SING,
                instruction: 0x9f,
                ..Default::default()
            },
        );

        let mut sfdp = [0_u8; 4];
        qspi.blocking_read(
            &mut sfdp,
            TransferConfig {
                iwidth: QspiWidth::SING,
                awidth: QspiWidth::SING,
                dwidth: QspiWidth::SING,
                instruction: 0x5a,
                address: Some(0),
                dummy: DummyCycles::_8,
            },
        );

        let valid_id = jedec != [0; 3] && jedec != [0xff; 3];
        let passed = valid_id && sfdp == *b"SFDP";

        blue.set_high();
        if passed {
            red.set_high();
            green.set_low();
            #[cfg(feature = "defmt")]
            defmt::info!(
                "QSPI JEDEC/SFDP passed: manufacturer={=u8:#x}, type={=u8:#x}, capacity={=u8:#x}",
                jedec[0],
                jedec[1],
                jedec[2]
            );
        } else {
            green.set_high();
            red.set_low();
            #[cfg(feature = "defmt")]
            defmt::error!(
                "QSPI JEDEC/SFDP failed: jedec={=[u8; 3]:#x}, sfdp={=[u8; 4]:#x}",
                jedec,
                sfdp
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
