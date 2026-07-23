#![no_std]
//! Runtime-neutral board support for the dual-core Arduino GIGA R1 WiFi.
//!
//! `giga-r1` supplies typed board metadata, pin mappings, owned RGB LED
//! controls, peripheral configuration helpers, a shared dual-core mailbox,
//! and self-contained CYW4343W initialization. Public hardware interfaces use
//! [`embedded_hal`] and, when enabled, [`embedded_hal_async`] traits; the crate
//! does not require a particular MCU HAL or executor.
//!
//! Hardware families are opt-in through the `audio`, `ble`, `camera`, `can`,
//! `display`, `dual-core`, `ipc`, `qspi`, `sdcard`, `usb`, and `wifi`
//! features. Logging support is independently enabled with `defmt`.
//!
//! See the
//! [repository examples](https://github.com/anapeksha/giga-r1-rs/tree/main/examples)
//! for tested Cortex-M7 and Cortex-M4 applications.

#[cfg(feature = "ble")]
pub mod ble;
mod board;
#[cfg(feature = "dual-core")]
pub mod bridge;
#[cfg(feature = "ipc")]
pub mod ipc;
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
