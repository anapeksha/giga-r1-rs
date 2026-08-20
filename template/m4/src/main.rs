#![no_std]
#![no_main]

use core::mem::MaybeUninit;

use cortex_m_rt::entry;
use embassy_stm32::SharedData;
use giga_r1::ipc::{Channel, IpcMailbox};
use panic_halt as _;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Deserialize)]
struct Ping {
    sequence: u32,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
struct Pong {
    sequence: u32,
    checksum: u32,
}

#[allow(unsafe_code)]
#[unsafe(link_section = ".shared_data")]
static SHARED_DATA: MaybeUninit<SharedData> = MaybeUninit::uninit();

#[allow(unsafe_code)]
#[used]
#[unsafe(link_section = ".ipc_mailbox")]
static IPC: IpcMailbox = IpcMailbox::new();

#[entry]
fn main() -> ! {
    // The M7 initializes the clocks and shared Embassy state before releasing
    // this core. The CM4 Embassy feature supplies its device interrupt table;
    // bind individual interrupts only when adding interrupt-driven peripherals.
    let _p = embassy_stm32::init_secondary(&SHARED_DATA);

    while !IPC.is_initialized() {
        cortex_m::asm::wfe();
    }

    let mut channel = Channel::<Ping, Pong>::new(&IPC);
    loop {
        match channel.try_request() {
            Ok(Some((request_id, ping))) => {
                let response = Pong {
                    sequence: ping.sequence,
                    checksum: checksum(ping.sequence),
                };
                let _ = channel.respond(request_id, &response);
            }
            Ok(None) => cortex_m::asm::delay(10_000),
            Err(_) => cortex_m::asm::delay(10_000),
        }
    }
}

fn checksum(sequence: u32) -> u32 {
    sequence.rotate_left(7) ^ 0x4749_4741
}
