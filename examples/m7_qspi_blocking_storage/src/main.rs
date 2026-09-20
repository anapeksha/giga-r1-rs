#![no_std]
#![no_main]

use core::mem::MaybeUninit;

#[cfg(feature = "defmt")]
use defmt_rtt as _;
use embassy_stm32::{
    SharedData,
    gpio::{Level, Output, Speed},
};
use embassy_time::{Duration, Timer};
use embedded_storage::nor_flash::NorFlash;
use giga_r1::qspi::{BlockingOnboardQspiFlash, FLASH_SIZE, SECTOR_SIZE};
use panic_halt as _;

const TEST_OFFSET: u32 = (FLASH_SIZE - SECTOR_SIZE) as u32;
const PATTERN: [u8; 32] = *b"giga-r1 blocking qspi storage!!!";

#[allow(unsafe_code)]
#[unsafe(link_section = ".shared_data")]
static SHARED_DATA: MaybeUninit<SharedData> = MaybeUninit::uninit();

#[embassy_executor::main]
async fn main(_spawner: embassy_executor::Spawner) {
    let p = embassy_stm32::init_primary(embassy_stm32::Config::default(), &SHARED_DATA);

    let mut red = Output::new(p.PI12, Level::High, Speed::Low);
    let mut green = Output::new(p.PJ13, Level::High, Speed::Low);
    let mut blue = Output::new(p.PE3, Level::High, Speed::Low);

    let mut flash =
        BlockingOnboardQspiFlash::new(p.QUADSPI, p.PD11, p.PD12, p.PE2, p.PF6, p.PF10, p.PG6)
            .unwrap();

    let passed = exercise_blocking_storage(&mut flash);

    loop {
        blue.set_high();
        if passed {
            red.set_high();
            green.set_low();
            #[cfg(feature = "defmt")]
            defmt::info!(
                "blocking QSPI erase/write/read passed at {=u32:#x}",
                TEST_OFFSET
            );
        } else {
            green.set_high();
            red.set_low();
            #[cfg(feature = "defmt")]
            defmt::error!(
                "blocking QSPI erase/write/read failed at {=u32:#x}",
                TEST_OFFSET
            );
        }

        Timer::after(Duration::from_millis(600)).await;
        red.set_high();
        green.set_high();
        blue.set_low();
        Timer::after(Duration::from_millis(300)).await;
        blue.set_high();
    }
}

fn exercise_blocking_storage(flash: &mut impl NorFlash) -> bool {
    let mut erased = [0_u8; PATTERN.len()];
    let mut readback = [0_u8; PATTERN.len()];

    flash
        .erase(TEST_OFFSET, TEST_OFFSET + SECTOR_SIZE as u32)
        .and_then(|()| flash.read(TEST_OFFSET, &mut erased))
        .and_then(|()| flash.write(TEST_OFFSET, &PATTERN))
        .and_then(|()| flash.read(TEST_OFFSET, &mut readback))
        .is_ok()
        && erased == [0xff; PATTERN.len()]
        && readback == PATTERN
}
