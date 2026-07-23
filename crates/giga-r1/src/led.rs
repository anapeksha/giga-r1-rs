//! Runtime-independent control of the onboard active-low RGB LED.

use embedded_hal::digital::OutputPin;

/// RGB color represented by the onboard LED channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Color {
    /// All channels off.
    Off,
    /// Red channel.
    Red,
    /// Green channel.
    Green,
    /// Blue channel.
    Blue,
    /// Red and green channels.
    Yellow,
    /// Green and blue channels.
    Cyan,
    /// Red and blue channels.
    Magenta,
    /// All channels.
    White,
}

impl Color {
    const fn channels(self) -> (bool, bool, bool) {
        match self {
            Self::Off => (false, false, false),
            Self::Red => (true, false, false),
            Self::Green => (false, true, false),
            Self::Blue => (false, false, true),
            Self::Yellow => (true, true, false),
            Self::Cyan => (false, true, true),
            Self::Magenta => (true, false, true),
            Self::White => (true, true, true),
        }
    }
}

/// Identifies the LED channel that rejected an output operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Channel {
    /// Red channel on PI12.
    Red,
    /// Green channel on PJ13.
    Green,
    /// Blue channel on PE3.
    Blue,
}

/// Error returned by an RGB LED operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Error<E> {
    /// Channel whose pin operation failed.
    pub channel: Channel,
    /// Error returned by the HAL pin.
    pub source: E,
}

/// Owned onboard RGB LED controller.
///
/// Pins must already be configured as push-pull outputs. [`RgbLed::new`]
/// immediately drives all three active-low channels high, leaving the LED off.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct RgbLed<RED, GREEN, BLUE> {
    red: RED,
    green: GREEN,
    blue: BLUE,
}

impl<RED, GREEN, BLUE, E> RgbLed<RED, GREEN, BLUE>
where
    RED: OutputPin<Error = E>,
    GREEN: OutputPin<Error = E>,
    BLUE: OutputPin<Error = E>,
{
    /// Takes ownership of the three output pins and initializes the LED off.
    pub fn new(red: RED, green: GREEN, blue: BLUE) -> Result<Self, Error<E>> {
        let mut led = Self { red, green, blue };
        led.set(Color::Off)?;
        Ok(led)
    }

    /// Displays one of the eight RGB channel combinations.
    pub fn set(&mut self, color: Color) -> Result<(), Error<E>> {
        let (red, green, blue) = color.channels();
        Self::drive(&mut self.red, Channel::Red, red)?;
        Self::drive(&mut self.green, Channel::Green, green)?;
        Self::drive(&mut self.blue, Channel::Blue, blue)
    }

    /// Switches all LED channels off.
    pub fn off(&mut self) -> Result<(), Error<E>> {
        self.set(Color::Off)
    }

    /// Returns ownership of all three pins.
    #[must_use]
    pub fn release(self) -> (RED, GREEN, BLUE) {
        (self.red, self.green, self.blue)
    }

    fn drive<PIN>(pin: &mut PIN, channel: Channel, enabled: bool) -> Result<(), Error<E>>
    where
        PIN: OutputPin<Error = E>,
    {
        if enabled {
            pin.set_low()
        } else {
            pin.set_high()
        }
        .map_err(|source| Error { channel, source })
    }
}
