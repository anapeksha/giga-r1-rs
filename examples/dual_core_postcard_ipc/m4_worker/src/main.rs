#![no_std]
#![no_main]

use cortex_m_rt::entry;
#[cfg(feature = "defmt")]
use defmt_rtt as _;
use dual_core_postcard_protocol::{BIN_COUNT, ComputeRequest, ComputeResponse, FftResult};
use giga_r1::ipc::{Channel, IpcMailbox};
use panic_halt as _;

#[allow(unsafe_code)]
#[used]
#[unsafe(link_section = ".ipc_mailbox")]
static IPC: IpcMailbox = IpcMailbox::new();

#[entry]
fn main() -> ! {
    while !IPC.is_initialized() {
        cortex_m::asm::wfe();
    }
    IPC.set_worker_state(1);
    let mut channel = Channel::<ComputeRequest, ComputeResponse>::new(&IPC);

    loop {
        match channel.try_request() {
            Ok(Some((request_id, ComputeRequest::ComputeFft(samples)))) => {
                IPC.set_worker_state(2);
                let response = ComputeResponse::Fft(compute_fft(samples));
                if channel.respond(request_id, &response).is_ok() {
                    IPC.set_worker_state(3);
                } else {
                    IPC.set_worker_state(4);
                }
            }
            Ok(None) => cortex_m::asm::delay(10_000),
            Err(_) => {
                IPC.set_worker_state(5);
                cortex_m::asm::delay(10_000);
            }
        }
    }
}

fn compute_fft(samples: [i16; 8]) -> FftResult {
    const COS_Q14: [[i16; 8]; BIN_COUNT] = [
        [16384, 16384, 16384, 16384, 16384, 16384, 16384, 16384],
        [16384, 11585, 0, -11585, -16384, -11585, 0, 11585],
        [16384, 0, -16384, 0, 16384, 0, -16384, 0],
        [16384, -11585, 0, 11585, -16384, 11585, 0, -11585],
        [16384, -16384, 16384, -16384, 16384, -16384, 16384, -16384],
    ];
    const SIN_Q14: [[i16; 8]; BIN_COUNT] = [
        [0; 8],
        [0, -11585, -16384, -11585, 0, 11585, 16384, 11585],
        [0, -16384, 0, 16384, 0, -16384, 0, 16384],
        [0, -11585, 16384, -11585, 0, 11585, -16384, 11585],
        [0; 8],
    ];

    let mut power = [0_u64; BIN_COUNT];
    for bin in 0..BIN_COUNT {
        let mut real = 0_i64;
        let mut imaginary = 0_i64;
        for (sample, (cosine, sine)) in samples
            .iter()
            .zip(COS_Q14[bin].iter().zip(SIN_Q14[bin].iter()))
        {
            real += i64::from(*sample) * i64::from(*cosine);
            imaginary += i64::from(*sample) * i64::from(*sine);
        }
        real >>= 14;
        imaginary >>= 14;
        power[bin] = (real * real + imaginary * imaginary) as u64;
    }
    FftResult { power }
}
