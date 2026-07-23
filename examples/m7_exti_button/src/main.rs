#![no_std]
#![no_main]

use core::mem::MaybeUninit;

#[cfg(feature = "defmt")]
use defmt_rtt as _;
use embassy_stm32::{
    SharedData, bind_interrupts,
    exti::{ExtiInput, InterruptHandler},
    gpio::{Level, Output, Pull, Speed},
    interrupt::typelevel::EXTI15_10,
};
use panic_halt as _;

bind_interrupts!(struct Irqs {
    EXTI15_10 => InterruptHandler<EXTI15_10>;
});

#[allow(unsafe_code)]
#[unsafe(link_section = ".shared_data")]
static SHARED_DATA: MaybeUninit<SharedData> = MaybeUninit::uninit();

#[embassy_executor::main(
    executor = "embassy_stm32::executor::Executor",
    entry = "cortex_m_rt::entry"
)]
async fn main(_spawner: embassy_executor::Spawner) {
    let p = embassy_stm32::init_primary(embassy_stm32::Config::default(), &SHARED_DATA);

    let mut button = ExtiInput::new(p.PC13, p.EXTI13, Pull::Down, Irqs);
    let mut green = Output::new(p.PJ13, Level::High, Speed::Low);
    let mut blue = Output::new(p.PE3, Level::Low, Speed::Low);
    let mut green_selected = false;

    loop {
        button.wait_for_rising_edge().await;
        green_selected = !green_selected;

        if green_selected {
            green.set_low();
            blue.set_high();
        } else {
            green.set_high();
            blue.set_low();
        }

        #[cfg(feature = "defmt")]
        defmt::info!("EXTI press; green selected: {}", green_selected);

        // Waiting for release provides switch debouncing and guarantees that
        // one physical press produces exactly one color transition.
        button.wait_for_falling_edge().await;
    }
}
