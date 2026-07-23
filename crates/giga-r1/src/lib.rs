#![no_std]
#![doc = include_str!("../../../README.md")]

mod board;
#[cfg(feature = "dual-core")]
pub mod bridge;
pub mod led;
pub mod pins;

#[cfg(feature = "can")]
pub mod can;
#[cfg(feature = "qspi")]
pub mod qspi;
#[cfg(feature = "usb")]
pub mod usb;
#[cfg(feature = "wifi")]
pub mod wifi;

pub use board::{Board, Core, GigaInfo};
