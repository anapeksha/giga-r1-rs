#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_halt as _;

use cortex_m_rt::entry;
use stm32h7xx_hal::{pac, prelude::*};

#[entry]
fn main() -> ! {
    // 1. Take MCU peripherals
    let dp = pac::Peripherals::take().unwrap();

    // 2. Configure Power Supply (VOS0 required for 480 MHz operation)
    let pwr = dp.PWR.constrain();
    let vos = pwr.vos0(&dp.SYSCFG).freeze();

    // 3. Configure Clocks
    let rcc = dp.RCC.constrain();
    let ccdr = rcc.sys_ck(480.MHz()).freeze(vos, &dp.SYSCFG);

    // 4. Print RTT Boot Message (Non-blocking)
    defmt::info!("--- BOOTING CORTEX-M7 ---");

    // 5. Configure GPIO Port E Pin 3 (Onboard Red LED)
    let gpioe = dp.GPIOE.split(ccdr.peripheral.GPIOE);
    let mut red_led = gpioe.pe3.into_push_pull_output();

    let mut count: u32 = 0;

    loop {
        // Toggle the physical LED
        red_led.toggle();

        // Send RTT log message
        defmt::info!("Loop heartbeat: {}", count);
        count = count.wrapping_add(1);

        // Simple hardware delay loop
        for _ in 0..12_000_000 {
            cortex_m::asm::nop();
        }
    }
}
