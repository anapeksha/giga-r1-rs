/// Processor core used by an application image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Core {
    /// Arm Cortex-M7, documented up to 480 MHz.
    M7,
    /// Arm Cortex-M4, documented up to 240 MHz.
    M4,
}

/// Static board identity and the recommended split-image memory layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct GigaInfo {
    /// Core that owns the image.
    pub core: Core,
    /// Maximum documented clock for the selected core.
    pub maximum_core_hz: u32,
    /// Start of the core's flash image.
    pub flash_origin: u32,
    /// Bytes reserved for the core's flash image.
    pub flash_size: u32,
}

impl GigaInfo {
    /// Recommended Cortex-M7 image layout.
    pub const M7: Self = Self {
        core: Core::M7,
        maximum_core_hz: 480_000_000,
        flash_origin: 0x0804_0000,
        flash_size: 768 * 1024,
    };

    /// Recommended Cortex-M4 image layout.
    pub const M4: Self = Self {
        core: Core::M4,
        maximum_core_hz: 240_000_000,
        flash_origin: 0x0810_0000,
        flash_size: 1024 * 1024,
    };
}

/// Marker for the Arduino GIGA R1 WiFi board.
///
/// This type owns no MCU peripherals and does not select a HAL. Applications
/// can use Embassy, an RTIC-oriented HAL, direct PAC access, or another
/// implementation while sharing this crate's board map and generic drivers.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Board;

impl Board {
    /// Create a zero-sized board marker.
    pub const fn new() -> Self {
        Self
    }
}
