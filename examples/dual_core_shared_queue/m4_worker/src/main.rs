#![no_std]
#![no_main]

use cortex_m_rt::entry;
#[cfg(feature = "defmt")]
use defmt_rtt as _;
use dual_core_shared_queue_protocol::{BLOCK_BYTES, BulkMailbox};
use giga_r1::ipc::QueueError;
use panic_halt as _;

#[allow(unsafe_code)]
#[used]
#[unsafe(link_section = ".bulk_queue")]
static BULK_MAILBOX: BulkMailbox = BulkMailbox::new();

#[entry]
fn main() -> ! {
    while !BULK_MAILBOX.is_initialized() {
        cortex_m::asm::delay(10_000);
    }

    let mut consumer = match BULK_MAILBOX.m7_to_m4.consumer() {
        Ok(consumer) => consumer,
        Err(error) => stop_with_error("M4 consumer claim", error),
    };
    let mut producer = match BULK_MAILBOX.m4_to_m7.producer() {
        Ok(producer) => producer,
        Err(error) => stop_with_error("M4 producer claim", error),
    };
    let mut block = [0_u8; BLOCK_BYTES];

    loop {
        let length = loop {
            match consumer.try_pop(&mut block) {
                Ok(length) => break length,
                Err(QueueError::Empty) => cortex_m::asm::delay(10_000),
                Err(error) => stop_with_error("request consumption", error),
            }
        };

        if length != BLOCK_BYTES {
            stop_with_invalid_length(length);
        }
        for byte in &mut block[4..length] {
            *byte ^= 0xA5;
        }

        loop {
            match producer.try_push(&block[..length]) {
                Ok(()) => break,
                Err(QueueError::Full) => cortex_m::asm::delay(10_000),
                Err(error) => stop_with_error("response publication", error),
            }
        }
        report_transformed(length);
    }
}

fn stop_with_error(context: &str, error: QueueError) -> ! {
    #[cfg(feature = "defmt")]
    defmt::error!("{} failed: {}", context, error);
    #[cfg(not(feature = "defmt"))]
    let _ = (context, error);
    loop {
        cortex_m::asm::delay(10_000_000);
    }
}

fn stop_with_invalid_length(length: usize) -> ! {
    #[cfg(feature = "defmt")]
    defmt::error!("expected {} bytes, received {}", BLOCK_BYTES, length);
    #[cfg(not(feature = "defmt"))]
    let _ = length;
    loop {
        cortex_m::asm::delay(10_000_000);
    }
}

fn report_transformed(length: usize) {
    #[cfg(feature = "defmt")]
    defmt::info!("transformed and returned {} bytes", length);
    #[cfg(not(feature = "defmt"))]
    let _ = length;
}
