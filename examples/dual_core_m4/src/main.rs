#![no_std]
#![no_main]

use core::mem::MaybeUninit;

use arduino_giga::bridge::{BridgeMailbox, RESPONSE_XOR};
use cortex_m_rt::entry;
#[cfg(feature = "defmt")]
use defmt_rtt as _;
use embassy_stm32::SharedData;
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
    while !BRIDGE.is_initialized() {
        cortex_m::asm::wfe();
    }

    // Make M4 startup visible even if secondary HAL initialization stalls.
    BRIDGE.increment_m4_heartbeat();
    let _peripherals = embassy_stm32::init_secondary(&SHARED_DATA);
    let mut previous_sequence = 0_u32;

    loop {
        BRIDGE.increment_m4_heartbeat();
        if let Some((sequence, command)) = BRIDGE.poll_command(previous_sequence) {
            BRIDGE.publish_response(sequence, command ^ RESPONSE_XOR);
            previous_sequence = sequence;
        }
        cortex_m::asm::delay(1_000_000);
    }
}
