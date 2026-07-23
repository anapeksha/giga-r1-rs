#![no_std]
#![doc = include_str!("../../../README.md")]

mod board;
pub mod pins;

#[cfg(feature = "can")]
pub mod can;
#[cfg(feature = "wifi")]
pub mod wifi;

pub use board::{Board, Core, GigaInfo};
