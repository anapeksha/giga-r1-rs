//! Arduino header and onboard-function pin map.

/// STM32 GPIO port.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Port {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
}

/// Physical STM32 GPIO identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct PinId {
    pub port: Port,
    pub pin: u8,
}

impl PinId {
    pub const fn new(port: Port, pin: u8) -> Self {
        Self { port, pin }
    }
}

/// Arduino D0 through D75.
pub const DIGITAL: [PinId; 76] = [
    p(Port::B, 7),
    p(Port::A, 9),
    p(Port::A, 3),
    p(Port::A, 2),
    p(Port::J, 8),
    p(Port::A, 7),
    p(Port::D, 13),
    p(Port::B, 4),
    p(Port::B, 8),
    p(Port::B, 9),
    p(Port::K, 1),
    p(Port::J, 10),
    p(Port::J, 11),
    p(Port::H, 6),
    p(Port::G, 14),
    p(Port::C, 7),
    p(Port::H, 13),
    p(Port::I, 9),
    p(Port::D, 5),
    p(Port::D, 6),
    p(Port::B, 11),
    p(Port::H, 4),
    p(Port::J, 12),
    p(Port::G, 13),
    p(Port::G, 12),
    p(Port::J, 0),
    p(Port::J, 14),
    p(Port::J, 1),
    p(Port::J, 15),
    p(Port::J, 2),
    p(Port::K, 3),
    p(Port::J, 3),
    p(Port::K, 4),
    p(Port::J, 4),
    p(Port::K, 5),
    p(Port::J, 5),
    p(Port::K, 6),
    p(Port::J, 6),
    p(Port::J, 7),
    p(Port::I, 14),
    p(Port::E, 6),
    p(Port::K, 7),
    p(Port::I, 15),
    p(Port::I, 10),
    p(Port::G, 10),
    p(Port::I, 13),
    p(Port::H, 15),
    p(Port::B, 2),
    p(Port::K, 0),
    p(Port::E, 4),
    p(Port::I, 11),
    p(Port::E, 5),
    p(Port::K, 2),
    p(Port::G, 7),
    p(Port::I, 5),
    p(Port::H, 8),
    p(Port::A, 6),
    p(Port::J, 9),
    p(Port::I, 7),
    p(Port::I, 6),
    p(Port::I, 4),
    p(Port::H, 14),
    p(Port::G, 11),
    p(Port::H, 11),
    p(Port::H, 10),
    p(Port::H, 9),
    p(Port::A, 1),
    p(Port::D, 4),
    p(Port::C, 6),
    p(Port::I, 0),
    p(Port::I, 1),
    p(Port::I, 2),
    p(Port::I, 3),
    p(Port::C, 1),
    p(Port::B, 12),
    p(Port::D, 3),
];

pub const ANALOG: [PinId; 14] = [
    p(Port::C, 4),
    p(Port::C, 5),
    p(Port::B, 0),
    p(Port::B, 1),
    p(Port::C, 3),
    p(Port::C, 2),
    p(Port::C, 0),
    p(Port::A, 0),
    p(Port::C, 2),
    p(Port::C, 3),
    p(Port::A, 1),
    p(Port::A, 0),
    p(Port::A, 4),
    p(Port::A, 5),
];

pub const LED_RED: PinId = p(Port::I, 12);
pub const LED_GREEN: PinId = p(Port::J, 13);
pub const LED_BLUE: PinId = p(Port::E, 3);
pub const USB_HOST_ENABLE: PinId = p(Port::A, 15);
/// BOOT0 push button. The signal is active high while the button is held.
pub const BOOT0_BUTTON: PinId = p(Port::C, 13);

const fn p(port: Port, pin: u8) -> PinId {
    PinId::new(port, pin)
}
