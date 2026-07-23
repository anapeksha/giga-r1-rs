#![no_std]
#![no_main]

use core::mem::MaybeUninit;

use cortex_m_rt::entry;
#[cfg(feature = "defmt")]
use defmt_rtt as _;
use embassy_stm32::{
    SharedData,
    gpio::{Level, Output, Speed},
};
use giga_r1::bridge::{BridgeMailbox, PING_XOR, RESPONSE_XOR, configure_m7_shared_sram};
use panic_halt as _;

#[allow(unsafe_code)]
#[unsafe(link_section = ".shared_data")]
static SHARED_DATA: MaybeUninit<SharedData> = MaybeUninit::uninit();

#[allow(unsafe_code)]
#[used]
#[unsafe(link_section = ".bridge_mailbox")]
static BRIDGE: BridgeMailbox = BridgeMailbox::new();

#[entry]
fn main() -> ! {
    let core = cortex_m::Peripherals::take().unwrap();
    configure_m7_shared_sram(core.MPU);

    let _p = embassy_stm32::init_primary(embassy_stm32::Config::default(), &SHARED_DATA);
    let mut red = Output::new(_p.PI12, Level::High, Speed::Low);
    let mut green = Output::new(_p.PJ13, Level::High, Speed::Low);
    let mut blue = Output::new(_p.PE3, Level::High, Speed::Low);

    BRIDGE.initialize_primary();
    cortex_m::asm::dsb();

    // Match Arduino's factory `bootM4()` sequence. The GIGA option bytes hold
    // CPU2, so the M7 must program its boot vector before releasing it.
    embassy_stm32::pac::RCC
        .apb4enr()
        .modify(|register| register.set_syscfgen(true));
    embassy_stm32::pac::RCC
        .c1_apb4enr()
        .modify(|register| register.set_syscfgen(true));
    cortex_m::asm::dsb();

    // On the dual-core H747, UR3.BOOT_ADD1 is named BCM4_ADD0 by the reference
    // manual. Its value is the upper 16 bits of the M4 vector-table address.
    embassy_stm32::pac::SYSCFG
        .ur3()
        .modify(|register| register.set_boot_add1(0x0810));
    cortex_m::asm::dsb();

    embassy_stm32::pac::RCC
        .gcr()
        .modify(|register| register.set_boot_c2(true));
    cortex_m::asm::dsb();
    cortex_m::asm::sev();

    let mut sequence = 0_u32;
    let mut previous_heartbeat = 0_u32;

    loop {
        sequence = sequence.wrapping_add(1);
        let command = PING_XOR ^ sequence;
        BRIDGE.publish_command(sequence, command);

        let mut valid_response = false;
        for _ in 0..20_000_000 {
            if BRIDGE.response(sequence) == Some(command ^ RESPONSE_XOR) {
                valid_response = true;
                break;
            }
        }

        let snapshot = BRIDGE.snapshot();
        let heartbeat_advanced = snapshot.m4_heartbeat != previous_heartbeat;
        previous_heartbeat = snapshot.m4_heartbeat;
        let passed = valid_response && heartbeat_advanced;

        blue.set_high();
        if passed {
            red.set_high();
            green.set_low();
            #[cfg(feature = "defmt")]
            defmt::info!("M7<->M4 bridge passed: {}", snapshot);
        } else {
            green.set_high();
            red.set_low();
            #[cfg(feature = "defmt")]
            defmt::error!(
                "M7<->M4 bridge failed: valid_response={}, heartbeat_advanced={}, snapshot={}",
                valid_response,
                heartbeat_advanced,
                snapshot
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
