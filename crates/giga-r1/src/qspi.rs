//! Board routing and storage wrapper for the onboard QSPI flash.
//!
//! The Arduino GIGA R1 routes a 16 MiB SPI NOR flash to STM32H747 QUADSPI bank
//! 1. [`FLASH`] remains the board routing source of truth.
//!
//! [`OnboardQspiFlash`] is an optional Embassy-backed convenience wrapper
//! implementing [`embedded_storage_async::nor_flash::ReadNorFlash`] and
//! [`embedded_storage_async::nor_flash::NorFlash`] for generic storage crates
//! such as `sequential-storage`.
//!
//! ```ignore
//! use giga_r1::qspi::OnboardQspiFlash;
//!
//! let mut flash = OnboardQspiFlash::new(
//!     p.QUADSPI, p.PD11, p.PD12, p.PE2, p.PF6, p.PF10, p.PG6,
//! )
//! .await?;
//! let app_storage = 0..(256 * 1024);
//! // Pass `&mut flash` and `app_storage` to sequential-storage map APIs.
//! ```
//!
//! Applications are responsible for choosing safe storage ranges and for any
//! higher-level schema, wear-leveling policy, or database format.

use crate::pins::{PinId, Port};
use embassy_stm32::{
    Peri,
    gpio::Speed,
    mode::Blocking,
    qspi::{
        BK1D0Pin, BK1D1Pin, BK1D2Pin, BK1D3Pin, BK1NSSPin, Config, Instance, Qspi, SckPin,
        TransferConfig,
        enums::{
            AddressSize, ChipSelectHighTime, FIFOThresholdLevel, MemorySize, QspiWidth,
            SampleShifting,
        },
    },
};
use embedded_storage_async::nor_flash::{
    ErrorType, NorFlash, NorFlashError, NorFlashErrorKind, ReadNorFlash,
};

/// Total capacity of the onboard QSPI NOR flash: 16 MiB.
pub const FLASH_SIZE: usize = 16 * 1024 * 1024;
/// Smallest page-program chunk accepted by common SPI NOR devices.
pub const PAGE_SIZE: usize = 256;
/// Sector erase granularity used by the wrapper.
pub const SECTOR_SIZE: usize = 4096;

const READ: u8 = 0x03;
const READ_STATUS: u8 = 0x05;
const WRITE_ENABLE: u8 = 0x06;
const PAGE_PROGRAM: u8 = 0x02;
const SECTOR_ERASE: u8 = 0x20;
const JEDEC_ID: u8 = 0x9f;
const STATUS_BUSY: u8 = 0x01;

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

/// Configuration for [`OnboardQspiFlash`].
#[derive(Clone, Copy)]
pub struct QspiFlashConfig {
    /// Delay between status-register polls while waiting for the flash to become
    /// ready after erase/program operations.
    pub ready_poll_interval: embassy_time::Duration,
    /// Maximum number of status-register polls before returning
    /// [`QspiFlashError::BusyTimeout`].
    pub ready_poll_attempts: usize,
    /// Embassy QUADSPI peripheral configuration.
    pub peripheral: Config,
}

impl Default for QspiFlashConfig {
    fn default() -> Self {
        #[allow(clippy::field_reassign_with_default)]
        let mut peripheral = Config::default();
        peripheral.memory_size = MemorySize::_16MiB;
        peripheral.address_size = AddressSize::_24bit;
        peripheral.prescaler = 16;
        peripheral.fifo_threshold = FIFOThresholdLevel::_4Bytes;
        peripheral.cs_high_time = ChipSelectHighTime::_5Cycle;
        peripheral.sample_shifting = SampleShifting::HalfCycle;
        peripheral.gpio_speed = Speed::VeryHigh;
        peripheral.dual_flash = false;

        Self {
            ready_poll_interval: embassy_time::Duration::from_millis(1),
            ready_poll_attempts: 10_000,
            peripheral,
        }
    }
}

/// Error returned by the onboard QSPI flash wrapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum QspiFlashError {
    /// The requested range is outside the 16 MiB onboard flash.
    OutOfBounds,
    /// The operation did not meet erase/write alignment requirements.
    Unaligned,
    /// The flash remained busy past the configured polling budget.
    BusyTimeout,
}

impl NorFlashError for QspiFlashError {
    fn kind(&self) -> NorFlashErrorKind {
        match self {
            Self::OutOfBounds => NorFlashErrorKind::OutOfBounds,
            Self::Unaligned => NorFlashErrorKind::NotAligned,
            Self::BusyTimeout => NorFlashErrorKind::Other,
        }
    }
}

/// Embassy-backed wrapper for the Arduino GIGA R1 onboard QSPI NOR flash.
///
/// This type uses standard SPI NOR commands over the STM32H747 QUADSPI bank-1
/// routing described by [`FLASH`]. Reads use command `0x03`, writes are split on
/// 256-byte page boundaries, and erases use 4 KiB sectors. Public async storage
/// trait methods yield between status-register polls; the underlying Embassy
/// QSPI transfers are currently blocking.
///
/// The wrapper intentionally does not reserve ranges or impose a database,
/// filesystem, DHCP, or application storage policy.
pub struct OnboardQspiFlash<'d, T: Instance> {
    qspi: Qspi<'d, T, Blocking>,
    config: QspiFlashConfig,
}

impl<'d, T> OnboardQspiFlash<'d, T>
where
    T: Instance,
{
    /// Construct the onboard bank-1 flash with [`QspiFlashConfig::default`].
    pub async fn new(
        peri: Peri<'d, T>,
        io0: Peri<'d, impl BK1D0Pin<T>>,
        io1: Peri<'d, impl BK1D1Pin<T>>,
        io2: Peri<'d, impl BK1D2Pin<T>>,
        io3: Peri<'d, impl BK1D3Pin<T>>,
        clock: Peri<'d, impl SckPin<T>>,
        chip_select: Peri<'d, impl BK1NSSPin<T>>,
    ) -> Result<Self, QspiFlashError> {
        Self::new_with_config(
            peri,
            io0,
            io1,
            io2,
            io3,
            clock,
            chip_select,
            QspiFlashConfig::default(),
        )
        .await
    }

    /// Construct the onboard bank-1 flash with custom timing/QSPI settings.
    #[allow(clippy::too_many_arguments)]
    pub async fn new_with_config(
        peri: Peri<'d, T>,
        io0: Peri<'d, impl BK1D0Pin<T>>,
        io1: Peri<'d, impl BK1D1Pin<T>>,
        io2: Peri<'d, impl BK1D2Pin<T>>,
        io3: Peri<'d, impl BK1D3Pin<T>>,
        clock: Peri<'d, impl SckPin<T>>,
        chip_select: Peri<'d, impl BK1NSSPin<T>>,
        config: QspiFlashConfig,
    ) -> Result<Self, QspiFlashError> {
        let qspi = Qspi::new_blocking_bank1(
            peri,
            io0,
            io1,
            io2,
            io3,
            clock,
            chip_select,
            config.peripheral,
        );
        let mut flash = Self { qspi, config };
        flash.wait_ready().await?;
        Ok(flash)
    }

    /// Return ownership of the underlying Embassy QSPI peripheral wrapper.
    #[must_use]
    pub fn release(self) -> Qspi<'d, T, Blocking> {
        self.qspi
    }

    /// Read the three-byte JEDEC identifier with command `0x9f`.
    pub async fn read_jedec_id(&mut self) -> Result<[u8; 3], QspiFlashError> {
        self.wait_ready().await?;
        let mut id = [0_u8; 3];
        self.qspi.blocking_read(
            &mut id,
            TransferConfig {
                iwidth: QspiWidth::SING,
                dwidth: QspiWidth::SING,
                instruction: JEDEC_ID,
                ..Default::default()
            },
        );
        Ok(id)
    }

    async fn write_enable(&mut self) -> Result<(), QspiFlashError> {
        self.wait_ready().await?;
        self.qspi.blocking_command(TransferConfig {
            iwidth: QspiWidth::SING,
            instruction: WRITE_ENABLE,
            ..Default::default()
        });
        Ok(())
    }

    async fn wait_ready(&mut self) -> Result<(), QspiFlashError> {
        for _ in 0..self.config.ready_poll_attempts {
            let mut status = [0_u8; 1];
            self.qspi.blocking_read(
                &mut status,
                TransferConfig {
                    iwidth: QspiWidth::SING,
                    dwidth: QspiWidth::SING,
                    instruction: READ_STATUS,
                    ..Default::default()
                },
            );
            if status[0] & STATUS_BUSY == 0 {
                return Ok(());
            }
            embassy_time::Timer::after(self.config.ready_poll_interval).await;
        }
        Err(QspiFlashError::BusyTimeout)
    }

    fn validate_range(offset: u32, length: usize) -> Result<(), QspiFlashError> {
        let offset = offset as usize;
        if length > FLASH_SIZE || offset > FLASH_SIZE - length {
            Err(QspiFlashError::OutOfBounds)
        } else {
            Ok(())
        }
    }
}

impl<T: Instance> ErrorType for OnboardQspiFlash<'_, T> {
    type Error = QspiFlashError;
}

impl<T: Instance> ReadNorFlash for OnboardQspiFlash<'_, T> {
    const READ_SIZE: usize = 1;

    async fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        Self::validate_range(offset, bytes.len())?;
        self.wait_ready().await?;
        if bytes.is_empty() {
            return Ok(());
        }
        self.qspi.blocking_read(
            bytes,
            TransferConfig {
                iwidth: QspiWidth::SING,
                awidth: QspiWidth::SING,
                dwidth: QspiWidth::SING,
                instruction: READ,
                address: Some(offset),
                ..Default::default()
            },
        );
        Ok(())
    }

    fn capacity(&self) -> usize {
        FLASH_SIZE
    }
}

impl<T: Instance> NorFlash for OnboardQspiFlash<'_, T> {
    const WRITE_SIZE: usize = 1;
    const ERASE_SIZE: usize = SECTOR_SIZE;

    async fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        if from > to {
            return Err(QspiFlashError::OutOfBounds);
        }
        Self::validate_range(from, (to - from) as usize)?;
        if !(from as usize).is_multiple_of(SECTOR_SIZE)
            || !(to as usize).is_multiple_of(SECTOR_SIZE)
        {
            return Err(QspiFlashError::Unaligned);
        }

        let mut address = from;
        while address < to {
            self.write_enable().await?;
            self.qspi.blocking_command(TransferConfig {
                iwidth: QspiWidth::SING,
                awidth: QspiWidth::SING,
                instruction: SECTOR_ERASE,
                address: Some(address),
                ..Default::default()
            });
            self.wait_ready().await?;
            address += SECTOR_SIZE as u32;
        }
        Ok(())
    }

    async fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        Self::validate_range(offset, bytes.len())?;
        let mut address = offset;
        let mut remaining = bytes;

        while !remaining.is_empty() {
            let page_remaining = PAGE_SIZE - (address as usize % PAGE_SIZE);
            let chunk_len = remaining.len().min(page_remaining);
            let (chunk, rest) = remaining.split_at(chunk_len);

            self.write_enable().await?;
            self.qspi.blocking_write(
                chunk,
                TransferConfig {
                    iwidth: QspiWidth::SING,
                    awidth: QspiWidth::SING,
                    dwidth: QspiWidth::SING,
                    instruction: PAGE_PROGRAM,
                    address: Some(address),
                    ..Default::default()
                },
            );
            self.wait_ready().await?;

            address += chunk_len as u32;
            remaining = rest;
        }
        Ok(())
    }
}
