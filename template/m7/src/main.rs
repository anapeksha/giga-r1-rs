#![no_std]
#![no_main]

use core::mem::MaybeUninit;

use cortex_m_rt::entry;
#[cfg(feature = "defmt")]
use defmt_rtt as _;
use embassy_stm32::{
    gpio::{Level, Output, Speed},
    SharedData,
};
use giga_r1::{
    bridge::configure_m7_shared_sram,
    ipc::{Channel, IpcError, IpcMailbox},
    led::{Color, RgbLed},
};
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
    let core = cortex_m::Peripherals::take().unwrap();
    configure_m7_shared_sram(core.MPU);

    // The M7 owns board initialization and the clock tree.
    let p = embassy_stm32::init_primary(embassy_stm32::Config::default(), &SHARED_DATA);
    let mut led = RgbLed::new(
        Output::new(p.PI12, Level::High, Speed::Low),
        Output::new(p.PJ13, Level::High, Speed::Low),
        Output::new(p.PE3, Level::High, Speed::Low),
    )
    .unwrap();

    IPC.initialize_primary();
    start_m4();

    let mut channel = Channel::<Ping, Pong>::new(&IPC);
    let mut sequence = 0_u32;
    loop {
        sequence = sequence.wrapping_add(1).max(1);
        let ping = Ping { sequence };
        let result = channel.send(&ping).and_then(|request| {
            for _ in 0..20_000_000 {
                if let Some(response) = channel.try_response(request)? {
                    return Ok(response);
                }
            }
            Err(IpcError::ResponsePending)
        });

        let replied = matches!(
            result,
            Ok(Pong {
                sequence: received,
                checksum: received_checksum,
            }) if received == sequence && received_checksum == checksum(sequence)
        );

        let color = if replied { Color::Green } else { Color::Red };
        led.set(color).unwrap();
        #[cfg(feature = "defmt")]
        if replied {
            defmt::info!("M4 replied to typed IPC ping {}", sequence);
        } else {
            defmt::warn!("M4 did not reply to typed IPC ping {}", sequence);
        }

        cortex_m::asm::delay(120_000_000);
        led.off().unwrap();
        cortex_m::asm::delay(30_000_000);
    }
}

fn checksum(sequence: u32) -> u32 {
    sequence.rotate_left(7) ^ 0x4749_4741
}

fn start_m4() {
    // Match Arduino's bootM4() sequence: select the M4 vector table in flash
    // bank 2, then release CPU2 from the hold imposed by the option bytes.
    embassy_stm32::pac::RCC
        .apb4enr()
        .modify(|register| register.set_syscfgen(true));
    embassy_stm32::pac::RCC
        .c1_apb4enr()
        .modify(|register| register.set_syscfgen(true));
    cortex_m::asm::dsb();

    embassy_stm32::pac::SYSCFG
        .ur3()
        .modify(|register| register.set_boot_add1(0x0810));
    cortex_m::asm::dsb();

    embassy_stm32::pac::RCC
        .gcr()
        .modify(|register| register.set_boot_c2(true));
    cortex_m::asm::dsb();
    cortex_m::asm::sev();
}
