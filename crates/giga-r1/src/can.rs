//! Board routing for the two FDCAN controllers.

use crate::pins::{PinId, Port};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct CanPins {
    pub rx: PinId,
    pub tx: PinId,
}

pub const CAN0: CanPins = CanPins {
    rx: PinId::new(Port::B, 5),
    tx: PinId::new(Port::B, 13),
};

pub const CAN1: CanPins = CanPins {
    rx: PinId::new(Port::B, 8),
    tx: PinId::new(Port::H, 13),
};
