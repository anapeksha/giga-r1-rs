#![no_std]
#![no_main]

use core::mem::MaybeUninit;

use cortex_m_rt::entry;
#[cfg(feature = "defmt")]
use defmt_rtt as _;
use dual_core_postcard_protocol::{ComputeRequest, ComputeResponse};
use embassy_stm32::{
    SharedData,
    gpio::{Level, Output, Speed},
};
use giga_r1::{
    bridge::configure_m7_shared_sram,
    ipc::{Channel, EventDoorbell, IpcMailbox},
    led::{Color, RgbLed},
};
use panic_halt as _;

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
    let p = embassy_stm32::init_primary(embassy_stm32::Config::default(), &SHARED_DATA);
    let mut led = RgbLed::new(
        Output::new(p.PI12, Level::High, Speed::Low),
        Output::new(p.PJ13, Level::High, Speed::Low),
        Output::new(p.PE3, Level::High, Speed::Low),
    )
    .unwrap();

    IPC.initialize_primary();
    start_m4();
    let mut channel =
        Channel::<ComputeRequest, ComputeResponse, _>::with_notifier(&IPC, EventDoorbell);
    let samples = [1_000_i16; 8];

    loop {
        led.set(Color::Yellow).unwrap();
        let response = channel
            .send(&ComputeRequest::ComputeFft(samples))
            .and_then(|request| {
                for _ in 0..40_000_000 {
                    if let Some(response) = channel.try_response(request)? {
                        return Ok(response);
                    }
                }
                Err(giga_r1::ipc::IpcError::ResponsePending)
            });
        let passed = matches!(
            response,
            Ok(ComputeResponse::Fft(result))
                if result.power[0] == 64_000_000
                    && result.power[1..].iter().all(|power| *power <= 1)
        );

        let color = if passed {
            Color::Green
        } else {
            match IPC.worker_state() {
                0 => Color::Red,
                1 => Color::Yellow,
                2 => Color::Cyan,
                3 => Color::Magenta,
                _ => Color::White,
            }
        };
        led.set(color).unwrap();
        #[cfg(feature = "defmt")]
        match response {
            Ok(response) => defmt::info!("typed M7 -> M4 FFT response: {}", response),
            Err(error) => defmt::error!("typed IPC failed: {}", error),
        }
        cortex_m::asm::delay(120_000_000);
        led.set(Color::Blue).unwrap();
        cortex_m::asm::delay(30_000_000);
    }
}

fn start_m4() {
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
