//! Runtime-neutral CYW4343W Bluetooth controller bring-up.
//!
//! Bluetooth uses a dedicated four-wire HCI UART, independently of the Wi-Fi
//! SDIO function. Configure UART7 for 115,200 baud, 8-N-1 with RTS/CTS using
//! the pin identities in [`crate::pins`], then pass the owned UART and control
//! pins to [`BleResources`]. The returned [`BleController`] can be split into
//! application-owned parts for `bt-hci`, `trouble-host`, or another HCI host.

use embedded_hal::{
    delay::DelayNs,
    digital::{InputPin, OutputPin},
};
use embedded_io_async::{Read, Write};

pub const INITIAL_BAUD: u32 = 115_200;
pub const POWER_RESET_US: u32 = 1_000;
pub const POWER_SETTLE_MS: u32 = 500;
pub const HCI_FIRMWARE: &[u8] = include_bytes!("firmware/4343WA1.hcd");

/// Board-owned Bluetooth resources before controller startup.
pub struct BleResources<U, P, H, D> {
    uart: U,
    power: P,
    host_wake: H,
    device_wake: D,
}

impl<U, P, H, D> BleResources<U, P, H, D> {
    pub const fn new(uart: U, power: P, host_wake: H, device_wake: D) -> Self {
        Self {
            uart,
            power,
            host_wake,
            device_wake,
        }
    }
}

/// Initialized Bluetooth HCI controller resources.
pub struct BleController<U, P, H, D> {
    uart: U,
    power: P,
    host_wake: H,
    device_wake: D,
}

/// Owned controller parts returned to the consuming application.
pub struct BleControllerParts<U, P, H, D> {
    pub uart: U,
    pub power: P,
    pub host_wake: H,
    pub device_wake: D,
}

/// Bluetooth control-line failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum BleError<E> {
    Power(E),
    DeviceWake(E),
}

/// Bluetooth HCI firmware or transport initialization failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum HciInitError<E> {
    Io(E),
    UnexpectedEnd,
    UnexpectedPacket(u8),
    InvalidFirmware,
    CommandFailed { opcode: u16, status: u8 },
}

impl<U, P, H, D, E> BleResources<U, P, H, D>
where
    P: OutputPin<Error = E>,
    D: OutputPin<Error = E>,
{
    /// Apply the Cypress reset/settling sequence with a blocking delay.
    pub fn start(
        mut self,
        delay: &mut impl DelayNs,
    ) -> Result<BleController<U, P, H, D>, BleError<E>> {
        self.device_wake.set_high().map_err(BleError::DeviceWake)?;
        self.power.set_low().map_err(BleError::Power)?;
        delay.delay_us(POWER_RESET_US);
        self.power.set_high().map_err(BleError::Power)?;
        delay.delay_ms(POWER_SETTLE_MS);
        self.device_wake.set_low().map_err(BleError::DeviceWake)?;
        Ok(BleController {
            uart: self.uart,
            power: self.power,
            host_wake: self.host_wake,
            device_wake: self.device_wake,
        })
    }

    /// Apply the same reset/settling sequence with any async delay provider.
    pub async fn start_async(
        mut self,
        delay: &mut impl embedded_hal_async::delay::DelayNs,
    ) -> Result<BleController<U, P, H, D>, BleError<E>> {
        self.device_wake.set_high().map_err(BleError::DeviceWake)?;
        self.power.set_low().map_err(BleError::Power)?;
        delay.delay_us(POWER_RESET_US).await;
        self.power.set_high().map_err(BleError::Power)?;
        delay.delay_ms(POWER_SETTLE_MS).await;
        self.device_wake.set_low().map_err(BleError::DeviceWake)?;
        Ok(BleController {
            uart: self.uart,
            power: self.power,
            host_wake: self.host_wake,
            device_wake: self.device_wake,
        })
    }
}

impl<U, P, H, D, E> BleController<U, P, H, D>
where
    H: InputPin<Error = E>,
    D: OutputPin<Error = E>,
{
    /// Returns whether the controller is asserting its active-low wake line.
    pub fn controller_is_awake(&mut self) -> Result<bool, E> {
        self.host_wake.is_low()
    }

    /// Assert or release the active-low device-wake line.
    pub fn set_controller_awake(&mut self, awake: bool) -> Result<(), E> {
        if awake {
            self.device_wake.set_low()
        } else {
            self.device_wake.set_high()
        }
    }
}

impl<U, P, H, D> BleController<U, P, H, D> {
    /// Download the bundled Cypress patchram image and reset the controller.
    ///
    /// This performs the board-specific initialization required before an HCI
    /// host such as `trouble-host` takes ownership. The returned controller
    /// retains all UART and control-line ownership.
    pub async fn initialize_hci(
        mut self,
        delay: &mut impl embedded_hal_async::delay::DelayNs,
    ) -> Result<Self, HciInitError<U::Error>>
    where
        U: Read + Write,
    {
        send_command(&mut self.uart, 0x0c03, &[]).await?;
        send_command(&mut self.uart, 0xfc2e, &[]).await?;

        let mut offset = 0;
        while offset < HCI_FIRMWARE.len() {
            if HCI_FIRMWARE.len() - offset < 3 {
                return Err(HciInitError::InvalidFirmware);
            }
            let opcode = u16::from_le_bytes([HCI_FIRMWARE[offset], HCI_FIRMWARE[offset + 1]]);
            let length = HCI_FIRMWARE[offset + 2] as usize;
            offset += 3;
            let end = offset
                .checked_add(length)
                .filter(|end| *end <= HCI_FIRMWARE.len())
                .ok_or(HciInitError::InvalidFirmware)?;
            send_command(&mut self.uart, opcode, &HCI_FIRMWARE[offset..end]).await?;
            offset = end;
        }

        delay.delay_ms(1_000).await;
        send_command(&mut self.uart, 0x0c03, &[]).await?;
        Ok(self)
    }

    pub fn into_parts(self) -> BleControllerParts<U, P, H, D> {
        BleControllerParts {
            uart: self.uart,
            power: self.power,
            host_wake: self.host_wake,
            device_wake: self.device_wake,
        }
    }
}

async fn send_command<U>(
    uart: &mut U,
    opcode: u16,
    parameters: &[u8],
) -> Result<(), HciInitError<U::Error>>
where
    U: Read + Write,
{
    let length = u8::try_from(parameters.len()).map_err(|_| HciInitError::InvalidFirmware)?;
    let opcode_bytes = opcode.to_le_bytes();
    let header = [0x01, opcode_bytes[0], opcode_bytes[1], length];
    uart.write_all(&header).await.map_err(HciInitError::Io)?;
    uart.write_all(parameters).await.map_err(HciInitError::Io)?;
    uart.flush().await.map_err(HciInitError::Io)?;

    loop {
        let mut packet_type = [0_u8; 1];
        read_full(uart, &mut packet_type).await?;
        if packet_type[0] != 0x04 {
            return Err(HciInitError::UnexpectedPacket(packet_type[0]));
        }

        let mut event_header = [0_u8; 2];
        read_full(uart, &mut event_header).await?;
        let mut payload = [0_u8; 255];
        let payload = &mut payload[..event_header[1] as usize];
        read_full(uart, payload).await?;

        match event_header[0] {
            0x0e if payload.len() >= 4 => {
                let completed_opcode = u16::from_le_bytes([payload[1], payload[2]]);
                if completed_opcode == opcode {
                    let status = payload[3];
                    return if status == 0 {
                        Ok(())
                    } else {
                        Err(HciInitError::CommandFailed { opcode, status })
                    };
                }
            }
            0x0f if payload.len() >= 4 => {
                let completed_opcode = u16::from_le_bytes([payload[2], payload[3]]);
                if completed_opcode == opcode && payload[0] != 0 {
                    return Err(HciInitError::CommandFailed {
                        opcode,
                        status: payload[0],
                    });
                }
            }
            _ => {}
        }
    }
}

async fn read_full<U>(uart: &mut U, mut bytes: &mut [u8]) -> Result<(), HciInitError<U::Error>>
where
    U: Read,
{
    while !bytes.is_empty() {
        let read = uart.read(bytes).await.map_err(HciInitError::Io)?;
        if read == 0 {
            return Err(HciInitError::UnexpectedEnd);
        }
        bytes = &mut bytes[read..];
    }
    Ok(())
}

#[cfg(feature = "defmt")]
impl<U, P, H, D> defmt::Format for BleResources<U, P, H, D> {
    fn format(&self, formatter: defmt::Formatter) {
        defmt::write!(formatter, "BleResources")
    }
}

#[cfg(feature = "defmt")]
impl<U, P, H, D> defmt::Format for BleController<U, P, H, D> {
    fn format(&self, formatter: defmt::Formatter) {
        defmt::write!(formatter, "BleController")
    }
}

#[cfg(feature = "defmt")]
impl<U, P, H, D> defmt::Format for BleControllerParts<U, P, H, D> {
    fn format(&self, formatter: defmt::Formatter) {
        defmt::write!(formatter, "BleControllerParts")
    }
}
