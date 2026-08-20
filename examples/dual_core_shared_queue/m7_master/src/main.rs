#![no_std]
#![no_main]

use core::mem::MaybeUninit;

use cortex_m_rt::entry;
#[cfg(feature = "defmt")]
use defmt_rtt as _;
use dual_core_shared_queue_protocol::{BLOCK_BYTES, BulkMailbox};
use embassy_stm32::{
    SharedData,
    gpio::{Level, Output, Speed},
};
use giga_r1::{
    bridge::configure_m7_shared_sram,
    ipc::QueueError,
    led::{Color, RgbLed},
};
use panic_halt as _;

#[allow(unsafe_code)]
#[unsafe(link_section = ".shared_data")]
static SHARED_DATA: MaybeUninit<SharedData> = MaybeUninit::uninit();

#[allow(unsafe_code)]
#[used]
#[unsafe(link_section = ".bulk_queue")]
static BULK_MAILBOX: BulkMailbox = BulkMailbox::new();

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

    if let Err(error) = BULK_MAILBOX.initialize_primary() {
        report_queue_error("mailbox initialization", error);
        led.set(Color::Red).unwrap();
        loop {
            cortex_m::asm::delay(10_000_000);
        }
    }
    start_m4();

    let mut producer = match BULK_MAILBOX.m7_to_m4.producer() {
        Ok(producer) => producer,
        Err(error) => {
            report_queue_error("M7 producer claim", error);
            led.set(Color::Red).unwrap();
            loop {
                cortex_m::asm::delay(10_000_000);
            }
        }
    };
    let mut consumer = match BULK_MAILBOX.m4_to_m7.consumer() {
        Ok(consumer) => consumer,
        Err(error) => {
            report_queue_error("M7 consumer claim", error);
            led.set(Color::Red).unwrap();
            loop {
                cortex_m::asm::delay(10_000_000);
            }
        }
    };

    let mut sequence = 1_u32;
    let mut outbound = [0_u8; BLOCK_BYTES];
    let mut inbound = [0_u8; BLOCK_BYTES];

    loop {
        fill_block(&mut outbound, sequence);
        led.set(Color::Yellow).unwrap();

        let sent = loop {
            match producer.try_push(&outbound) {
                Err(QueueError::Full) => cortex_m::asm::delay(10_000),
                result => break result,
            }
        };
        let received = if let Err(error) = sent {
            report_queue_error("request publication", error);
            Err(error)
        } else {
            loop {
                match consumer.try_pop(&mut inbound) {
                    Err(QueueError::Empty) => cortex_m::asm::delay(10_000),
                    result => break result,
                }
            }
        };

        let passed = match received {
            Ok(length) => validate_response(&inbound[..length], sequence),
            Err(error) => {
                report_queue_error("response consumption", error);
                false
            }
        };

        if passed {
            led.set(Color::Green).unwrap();
            report_success(sequence);
        } else {
            led.set(Color::Red).unwrap();
            report_validation_failure(sequence);
        }
        cortex_m::asm::delay(120_000_000);
        led.set(Color::Blue).unwrap();
        cortex_m::asm::delay(30_000_000);
        sequence = sequence.wrapping_add(1);
    }
}

fn fill_block(block: &mut [u8; BLOCK_BYTES], sequence: u32) {
    block[..4].copy_from_slice(&sequence.to_le_bytes());
    for (index, byte) in block[4..].iter_mut().enumerate() {
        *byte = sequence.wrapping_mul(17).wrapping_add(index as u32) as u8;
    }
}

fn validate_response(block: &[u8], sequence: u32) -> bool {
    if block.len() != BLOCK_BYTES || block[..4] != sequence.to_le_bytes() {
        return false;
    }
    block[4..].iter().enumerate().all(|(index, byte)| {
        let original = sequence.wrapping_mul(17).wrapping_add(index as u32) as u8;
        *byte == original ^ 0xA5
    })
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

fn report_queue_error(context: &str, error: QueueError) {
    #[cfg(feature = "defmt")]
    defmt::error!("{} failed: {}", context, error);
    #[cfg(not(feature = "defmt"))]
    let _ = (context, error);
}

fn report_success(sequence: u32) {
    #[cfg(feature = "defmt")]
    defmt::info!("validated 1536-byte round trip for sequence {}", sequence);
    #[cfg(not(feature = "defmt"))]
    let _ = sequence;
}

fn report_validation_failure(sequence: u32) {
    #[cfg(feature = "defmt")]
    defmt::error!("block validation failed for sequence {}", sequence);
    #[cfg(not(feature = "defmt"))]
    let _ = sequence;
}
