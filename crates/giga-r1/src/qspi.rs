//! Board routing for the onboard QSPI flash.

use crate::pins::{PinId, Port};

/// QSPI bank-1 signal routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct QspiPins {
    pub io0: PinId,
    pub io1: PinId,
    pub io2: PinId,
    pub io3: PinId,
    pub clock: PinId,
    pub chip_select: PinId,
}

/// Onboard QSPI flash connected to the STM32 QUADSPI bank 1.
pub const FLASH: QspiPins = QspiPins {
    io0: PinId::new(Port::D, 11),
    io1: PinId::new(Port::D, 12),
    io2: PinId::new(Port::E, 2),
    io3: PinId::new(Port::F, 6),
    clock: PinId::new(Port::F, 10),
    chip_select: PinId::new(Port::G, 6),
};
