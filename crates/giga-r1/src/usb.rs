//! Board-level USB connector routing.

use crate::pins::{PinId, Port};

/// Pins used by the USB-C connector in full-speed device mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct UsbDevicePins {
    pub dm: PinId,
    pub dp: PinId,
}

/// USB-C device connection through STM32 USB OTG FS.
pub const DEVICE: UsbDevicePins = UsbDevicePins {
    dm: PinId::new(Port::A, 11),
    dp: PinId::new(Port::A, 12),
};

/// Pins used by the USB-A connector's internal full-speed PHY.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct UsbHostPins {
    pub dm: PinId,
    pub dp: PinId,
    pub power_enable: PinId,
}

/// USB-A host connection through STM32 USB OTG HS.
pub const HOST: UsbHostPins = UsbHostPins {
    dm: PinId::new(Port::B, 14),
    dp: PinId::new(Port::B, 15),
    power_enable: PinId::new(Port::A, 15),
};
