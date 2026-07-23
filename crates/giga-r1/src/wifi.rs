//! Runtime-independent CYW4343W Wi-Fi bring-up.
//!
//! The onboard Murata LBEE5KL1DX radio is connected through SDIO. There is no
//! standard `embedded-hal` SDIO trait, so the initialized transport implements
//! [`cyw43::SdioBusCyw43`]. The crate owns the GIGA-specific firmware, NVRAM,
//! power sequence, and CYW43 construction; the application receives the
//! network device, control channel, low-level runner, and power pin.

use crate::pins::{PinId, Port};
use core::future::Future;
use embedded_hal::{delay::DelayNs, digital::OutputPin};

pub use cyw43::{Control, NetDriver, Runner, SdioBus, SdioBusCyw43, State};

/// Arduino's CYW4343W firmware image, aligned for SDIO transfers.
static FIRMWARE: &cyw43::Aligned<cyw43::A4, [u8]> = cyw43::aligned_bytes!("firmware/4343WA1.bin");

/// Country Locale Matrix matching [`FIRMWARE`].
static CLM: &[u8] = include_bytes!("firmware/4343WA1.clm_blob");

/// Arduino's GIGA NVRAM configuration for the onboard Murata Type 1DX.
static NVRAM: cyw43::Aligned<cyw43::A4, [u8; 562]> = cyw43::Aligned(
    *b"manfid=0x2d0\0\
prodid=0x0726\0\
vendid=0x14e4\0\
devid=0x43e2\0\
boardtype=0x0726\0\
boardrev=0x1202\0\
boardnum=22\0\
macaddr=02:00:00:00:47:01\0\
sromrev=11\0\
boardflags=0x00404201\0\
boardflags3=0x04000000\0\
xtalfreq=37400\0\
nocrc=1\0\
ag0=0\0\
aa2g=1\0\
ccode=ALL\0\
extpagain2g=0\0\
pa2ga0=-145,6667,-751\0\
AvVmid_c0=0x0,0xc8\0\
cckpwroffset0=2\0\
maxp2ga0=74\0\
cckbw202gpo=0\0\
legofdmbw202gpo=0x88888888\0\
mcsbw202gpo=0xaaaaaaaa\0\
propbw202gpo=0xdd\0\
ofdmdigfilttype=18\0\
ofdmdigfilttypebe=18\0\
papdmode=1\0\
papdvalidtest=1\0\
pacalidx2g=48\0\
papdepsoffset=-22\0\
papdendidx=58\0\
il0macaddr=02:00:00:00:47:01\0\
wl0id=0x431b\0\
muxenab=0x10\0\0\0",
);

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

/// Four-bit SDIO connection between the STM32H747 and Wi-Fi module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct WifiSdioPins {
    pub clock: PinId,
    pub command: PinId,
    pub data: [PinId; 4],
}

pub const SDIO: WifiSdioPins = WifiSdioPins {
    clock: PinId::new(Port::C, 12),
    command: PinId::new(Port::D, 2),
    data: [
        PinId::new(Port::C, 8),
        PinId::new(Port::C, 9),
        PinId::new(Port::C, 10),
        PinId::new(Port::C, 11),
    ],
};

/// Error returned while initializing the Wi-Fi power-control pin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct PowerError<E> {
    /// Error returned by the output pin.
    pub source: E,
}

/// Error returned while powering the radio or constructing its SDIO transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum StartError<POWER, TRANSPORT> {
    /// The Wi-Fi power-control pin rejected an operation.
    Power(POWER),
    /// The caller-provided SDIO transport factory failed.
    Transport(TRANSPORT),
}

/// Board-owned Wi-Fi power controller.
///
/// [`Wifi::new`] immediately drives PB10 low so the radio starts in reset.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Wifi<POWER> {
    power: POWER,
}

/// Fully initialized CYW4343W resources.
///
/// The application owns every returned part. It must keep polling
/// [`runner`](Self::runner) using its chosen executor or blocking future
/// runner. [`device`](Self::device) can then be handed to the application's
/// chosen network stack, while [`control`](Self::control) remains available
/// for scan, join, and power-management operations.
pub struct WifiParts<'a, SDIO, POWER>
where
    SDIO: SdioBusCyw43<64>,
{
    /// Network-device interface for a caller-selected IP stack.
    pub device: NetDriver<'a>,
    /// CYW4343W station control channel.
    pub control: Control<'a>,
    /// Runner retained until the consuming application takes ownership.
    runner: Option<Runner<'a, SdioBus<SDIO>>>,
    /// Owned and asserted Wi-Fi power pin.
    pub power: POWER,
}

impl<POWER, E> Wifi<POWER>
where
    POWER: OutputPin<Error = E>,
{
    /// Takes ownership of PB10 and holds the onboard radio in reset.
    pub fn new(mut power: POWER) -> Result<Self, PowerError<E>> {
        power.set_low().map_err(|source| PowerError { source })?;
        Ok(Self { power })
    }

    /// Runs the GIGA power sequence and initializes the CYW4343W over SDIO.
    ///
    /// The delay is based only on [`embedded_hal::delay::DelayNs`]. The
    /// returned future can be driven by any executor or by a blocking future
    /// runner selected by the application.
    pub async fn start_with<'a, SDIO, DELAY, FACTORY, TRANSPORT>(
        mut self,
        state: &'a mut State,
        delay: &mut DELAY,
        make_sdio: FACTORY,
    ) -> Result<WifiParts<'a, SDIO, POWER>, StartError<PowerError<E>, TRANSPORT>>
    where
        SDIO: SdioBusCyw43<64>,
        DELAY: DelayNs,
        FACTORY: FnOnce() -> Result<SDIO, TRANSPORT>,
    {
        self.power
            .set_low()
            .map_err(|source| StartError::Power(PowerError { source }))?;
        delay.delay_ms(250);
        self.power
            .set_high()
            .map_err(|source| StartError::Power(PowerError { source }))?;
        delay.delay_ms(500);

        // SDIO discovery communicates with the radio, so the transport must
        // only be constructed after the board-specific power sequence.
        let sdio = make_sdio().map_err(StartError::Transport)?;
        let (device, control, runner) = cyw43::new_sdio(state, sdio, FIRMWARE, &NVRAM).await;

        Ok(WifiParts {
            device,
            control,
            runner: Some(runner),
            power: self.power,
        })
    }

    /// Runs the power sequence with an asynchronous `embedded-hal` delay.
    pub async fn start_async_with<'a, SDIO, DELAY, FACTORY, FUTURE, TRANSPORT>(
        mut self,
        state: &'a mut State,
        delay: &mut DELAY,
        make_sdio: FACTORY,
    ) -> Result<WifiParts<'a, SDIO, POWER>, StartError<PowerError<E>, TRANSPORT>>
    where
        SDIO: SdioBusCyw43<64>,
        DELAY: embedded_hal_async::delay::DelayNs,
        FACTORY: FnOnce() -> FUTURE,
        FUTURE: Future<Output = Result<SDIO, TRANSPORT>>,
    {
        self.power
            .set_low()
            .map_err(|source| StartError::Power(PowerError { source }))?;
        delay.delay_ms(250).await;
        self.power
            .set_high()
            .map_err(|source| StartError::Power(PowerError { source }))?;
        delay.delay_ms(500).await;

        // SDIO discovery communicates with the radio, so the transport must
        // only be constructed after the board-specific power sequence.
        let sdio = make_sdio().await.map_err(StartError::Transport)?;
        let (device, control, runner) = cyw43::new_sdio(state, sdio, FIRMWARE, &NVRAM).await;

        Ok(WifiParts {
            device,
            control,
            runner: Some(runner),
            power: self.power,
        })
    }

    /// Returns ownership of the power pin without starting the radio.
    #[must_use]
    pub fn release(self) -> POWER {
        self.power
    }
}

impl<'a, SDIO, POWER> WifiParts<'a, SDIO, POWER>
where
    SDIO: SdioBusCyw43<64>,
{
    /// Takes the low-level runner exactly once.
    ///
    /// Start `runner.run()` using any executor or blocking future runner before
    /// awaiting [`Self::initialize`].
    pub fn take_runner(&mut self) -> Option<Runner<'a, SdioBus<SDIO>>> {
        self.runner.take()
    }

    /// Completes board-owned initialization using the bundled country data.
    ///
    /// The low-level runner must be polled concurrently.
    pub async fn initialize(&mut self) {
        self.control.init(CLM).await;
    }
}
