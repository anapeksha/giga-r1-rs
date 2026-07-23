#![no_std]
#![no_main]

use cortex_m_rt::entry;
#[cfg(feature = "defmt")]
use defmt_rtt as _;
use giga_r1::bridge::{BridgeMailbox, RESPONSE_XOR};
use panic_halt as _;

#[allow(unsafe_code)]
#[used]
#[unsafe(link_section = ".bridge_mailbox")]
static BRIDGE: BridgeMailbox = BridgeMailbox::new();

#[entry]
fn main() -> ! {
    while !BRIDGE.is_initialized() {
        cortex_m::asm::wfe();
    }

    // The bridge is deliberately HAL- and runtime-independent. The M7 owns
    // clocks and core release; the M4 only needs its core and shared mailbox.
    BRIDGE.increment_m4_heartbeat();
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
