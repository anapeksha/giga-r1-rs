#![no_std]
#![no_main]

use core::mem::MaybeUninit;

use cortex_m_rt::entry;
use embassy_stm32::SharedData;
use giga_r1::bridge::{BridgeMailbox, RESPONSE_XOR};
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
    // The M7 initializes the clocks and shared Embassy state before releasing
    // this core. The CM4 Embassy feature supplies its device interrupt table;
    // bind individual interrupts only when adding interrupt-driven peripherals.
    let _p = embassy_stm32::init_secondary(&SHARED_DATA);

    while !BRIDGE.is_initialized() {
        cortex_m::asm::wfe();
    }

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
