//! CYW4343W radio control routing.
//!
//! The onboard Murata LBEE5KL1DX radio is connected through SDIO. This module
//! is feature-gated so applications that do not use the radio carry no Wi-Fi
//! dependencies or board policy.

use crate::pins::{PinId, Port};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct WifiControlPins {
    pub host_wake: PinId,
    pub power: PinId,
}

pub const CONTROL: WifiControlPins = WifiControlPins {
    host_wake: PinId::new(Port::I, 8),
    power: PinId::new(Port::B, 10),
};
