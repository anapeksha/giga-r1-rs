#![no_std]

use serde::{Deserialize, Serialize};

pub const SAMPLE_COUNT: usize = 8;
pub const BIN_COUNT: usize = SAMPLE_COUNT / 2 + 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ComputeRequest {
    ComputeFft([i16; SAMPLE_COUNT]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct FftResult {
    pub power: [u64; BIN_COUNT],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ComputeResponse {
    Fft(FftResult),
}
