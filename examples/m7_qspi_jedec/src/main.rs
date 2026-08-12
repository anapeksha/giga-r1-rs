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
use giga_r1::qspi::OnboardQspiFlash;
use panic_halt as _;

#[allow(unsafe_code)]
#[unsafe(link_section = ".shared_data")]
static SHARED_DATA: MaybeUninit<SharedData> = MaybeUninit::uninit();

#[embassy_executor::main]
async fn main(_spawner: embassy_executor::Spawner) {
    let p = embassy_stm32::init_primary(embassy_stm32::Config::default(), &SHARED_DATA);

    let mut red = Output::new(p.PI12, Level::High, Speed::Low);
    let mut green = Output::new(p.PJ13, Level::High, Speed::Low);
    let mut blue = Output::new(p.PE3, Level::High, Speed::Low);

    let mut flash = OnboardQspiFlash::new(p.QUADSPI, p.PD11, p.PD12, p.PE2, p.PF6, p.PF10, p.PG6)
        .await
        .unwrap();

    loop {
        let jedec = flash.read_jedec_id().await.unwrap_or([0; 3]);
        let passed = jedec != [0; 3] && jedec != [0xff; 3];

        blue.set_high();
        if passed {
            red.set_high();
            green.set_low();
            #[cfg(feature = "defmt")]
            defmt::info!(
                "QSPI JEDEC passed: manufacturer={=u8:#x}, type={=u8:#x}, capacity={=u8:#x}",
                jedec[0],
                jedec[1],
                jedec[2]
            );
        } else {
            green.set_high();
            red.set_low();
            #[cfg(feature = "defmt")]
            defmt::error!("QSPI JEDEC failed: jedec={=[u8; 3]:#x}", jedec);
        }

        Timer::after(Duration::from_millis(600)).await;
        red.set_high();
        green.set_high();
        blue.set_low();
        Timer::after(Duration::from_millis(300)).await;
        blue.set_high();
    }
}
