//! Arduino header and onboard-function pin map.

use embedded_hal::digital::{ErrorType, InputPin, OutputPin, StatefulOutputPin};

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

/// An owned pin tagged with its Arduino silkscreen position.
///
/// `P` is deliberately supplied by the consuming HAL. This lets the BSP expose
/// board names without choosing an STM32 HAL or executor. Use [`Self::map`] to
/// configure the inner HAL pin while preserving its Arduino identity.
pub struct ArduinoPin<const POSITION: u8, P> {
    inner: P,
}

impl<const POSITION: u8, P> ArduinoPin<POSITION, P> {
    pub const fn new(inner: P) -> Self {
        Self { inner }
    }

    pub const fn position(&self) -> u8 {
        POSITION
    }

    pub fn map<T>(self, configure: impl FnOnce(P) -> T) -> ArduinoPin<POSITION, T> {
        ArduinoPin::new(configure(self.inner))
    }

    pub fn into_inner(self) -> P {
        self.inner
    }

    pub const fn inner(&self) -> &P {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut P {
        &mut self.inner
    }
}

impl<const POSITION: u8, P: ErrorType> ErrorType for ArduinoPin<POSITION, P> {
    type Error = P::Error;
}

impl<const POSITION: u8, P: InputPin> InputPin for ArduinoPin<POSITION, P> {
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        self.inner.is_high()
    }

    fn is_low(&mut self) -> Result<bool, Self::Error> {
        self.inner.is_low()
    }
}

impl<const POSITION: u8, P: OutputPin> OutputPin for ArduinoPin<POSITION, P> {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.inner.set_low()
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.inner.set_high()
    }
}

impl<const POSITION: u8, P: StatefulOutputPin> StatefulOutputPin for ArduinoPin<POSITION, P> {
    fn is_set_high(&mut self) -> Result<bool, Self::Error> {
        self.inner.is_set_high()
    }

    fn is_set_low(&mut self) -> Result<bool, Self::Error> {
        self.inner.is_set_low()
    }
}

impl<const POSITION: u8, P: core::fmt::Debug> core::fmt::Debug for ArduinoPin<POSITION, P> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("ArduinoPin")
            .field("position", &POSITION)
            .field("inner", &self.inner)
            .finish()
    }
}

#[cfg(feature = "defmt")]
impl<const POSITION: u8, P: defmt::Format> defmt::Format for ArduinoPin<POSITION, P> {
    fn format(&self, formatter: defmt::Formatter) {
        defmt::write!(
            formatter,
            "ArduinoPin(position={}, inner={})",
            POSITION,
            self.inner
        );
    }
}

/// Active-low onboard LED channel tagged independently from digital headers.
pub struct LedPin<P>(P);

impl<P> LedPin<P> {
    pub const fn new(inner: P) -> Self {
        Self(inner)
    }

    pub fn map<T>(self, configure: impl FnOnce(P) -> T) -> LedPin<T> {
        LedPin(configure(self.0))
    }

    pub fn into_inner(self) -> P {
        self.0
    }
}

#[cfg(feature = "defmt")]
impl<P> defmt::Format for LedPin<P> {
    fn format(&self, formatter: defmt::Formatter) {
        defmt::write!(formatter, "LedPin")
    }
}

/// Marker for an analog-only pad not modeled as a GPIO by the selected HAL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AnalogOnlyPin<const POSITION: u8> {
    id: PinId,
}

impl<const POSITION: u8> AnalogOnlyPin<POSITION> {
    pub const fn new(id: PinId) -> Self {
        Self { id }
    }

    pub const fn id(&self) -> PinId {
        self.id
    }
}

impl<P: ErrorType> ErrorType for LedPin<P> {
    type Error = P::Error;
}

impl<P: OutputPin> OutputPin for LedPin<P> {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.0.set_low()
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.0.set_high()
    }
}

impl<P: StatefulOutputPin> StatefulOutputPin for LedPin<P> {
    fn is_set_high(&mut self) -> Result<bool, Self::Error> {
        self.0.is_set_high()
    }

    fn is_set_low(&mut self) -> Result<bool, Self::Error> {
        self.0.is_set_low()
    }
}

macro_rules! define_pin_bank {
    ($name:ident, $count:literal, $($field:ident : $index:literal),+ $(,)?) => {
        pub struct $name<P> {
            $(pub $field: ArduinoPin<$index, P>,)+
        }

        impl<P> $name<P> {
            pub fn from_array(pins: [P; $count]) -> Self {
                let [$($field,)+] = pins;
                Self {
                    $($field: ArduinoPin::new($field),)+
                }
            }
        }

        #[cfg(feature = "defmt")]
        impl<P> defmt::Format for $name<P> {
            fn format(&self, formatter: defmt::Formatter) {
                defmt::write!(formatter, "DigitalPins")
            }
        }
    };
}

define_pin_bank!(
    DigitalPins,
    76,
    d0: 0, d1: 1, d2: 2, d3: 3, d4: 4, d5: 5, d6: 6, d7: 7,
    d8: 8, d9: 9, d10: 10, d11: 11, d12: 12, d13: 13, d14: 14, d15: 15,
    d16: 16, d17: 17, d18: 18, d19: 19, d20: 20, d21: 21, d22: 22, d23: 23,
    d24: 24, d25: 25, d26: 26, d27: 27, d28: 28, d29: 29, d30: 30, d31: 31,
    d32: 32, d33: 33, d34: 34, d35: 35, d36: 36, d37: 37, d38: 38, d39: 39,
    d40: 40, d41: 41, d42: 42, d43: 43, d44: 44, d45: 45, d46: 46, d47: 47,
    d48: 48, d49: 49, d50: 50, d51: 51, d52: 52, d53: 53, d54: 54, d55: 55,
    d56: 56, d57: 57, d58: 58, d59: 59, d60: 60, d61: 61, d62: 62, d63: 63,
    d64: 64, d65: 65, d66: 66, d67: 67, d68: 68, d69: 69, d70: 70, d71: 71,
    d72: 72, d73: 73, d74: 74, d75: 75,
);

/// Owned analog positions. Separate type parameters preserve each HAL's ADC
/// channel marker; analog capabilities must not be erased to a generic GPIO.
pub struct AnalogPins<A0, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11, A12, A13> {
    pub a0: ArduinoPin<0, A0>,
    pub a1: ArduinoPin<1, A1>,
    pub a2: ArduinoPin<2, A2>,
    pub a3: ArduinoPin<3, A3>,
    pub a4: ArduinoPin<4, A4>,
    pub a5: ArduinoPin<5, A5>,
    pub a6: ArduinoPin<6, A6>,
    pub a7: ArduinoPin<7, A7>,
    pub a8: ArduinoPin<8, A8>,
    pub a9: ArduinoPin<9, A9>,
    pub a10: ArduinoPin<10, A10>,
    pub a11: ArduinoPin<11, A11>,
    pub a12: ArduinoPin<12, A12>,
    pub a13: ArduinoPin<13, A13>,
}

#[cfg(feature = "defmt")]
impl<A0, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11, A12, A13> defmt::Format
    for AnalogPins<A0, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11, A12, A13>
{
    fn format(&self, formatter: defmt::Formatter) {
        defmt::write!(formatter, "AnalogPins")
    }
}

/// Owned Arduino header and onboard LED pins.
///
/// Digital, analog, and LED pins may use different HAL-erased types. The
/// constructor consumes every supplied pin exactly once; aliases such as
/// `A12`/`DAC0` remain one owned field.
#[allow(clippy::type_complexity)]
pub struct ArduinoGigaPins<D, A0, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11, A12, A13, L> {
    pub digital: DigitalPins<D>,
    pub analog: AnalogPins<A0, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11, A12, A13>,
    pub led_red: LedPin<L>,
    pub led_green: LedPin<L>,
    pub led_blue: LedPin<L>,
}

impl<D, A0, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11, A12, A13, L>
    ArduinoGigaPins<D, A0, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11, A12, A13, L>
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        digital: [D; 76],
        a0: A0,
        a1: A1,
        a2: A2,
        a3: A3,
        a4: A4,
        a5: A5,
        a6: A6,
        a7: A7,
        a8: A8,
        a9: A9,
        a10: A10,
        a11: A11,
        a12: A12,
        a13: A13,
        led_red: L,
        led_green: L,
        led_blue: L,
    ) -> Self {
        Self {
            digital: DigitalPins::from_array(digital),
            analog: AnalogPins {
                a0: ArduinoPin::new(a0),
                a1: ArduinoPin::new(a1),
                a2: ArduinoPin::new(a2),
                a3: ArduinoPin::new(a3),
                a4: ArduinoPin::new(a4),
                a5: ArduinoPin::new(a5),
                a6: ArduinoPin::new(a6),
                a7: ArduinoPin::new(a7),
                a8: ArduinoPin::new(a8),
                a9: ArduinoPin::new(a9),
                a10: ArduinoPin::new(a10),
                a11: ArduinoPin::new(a11),
                a12: ArduinoPin::new(a12),
                a13: ArduinoPin::new(a13),
            },
            led_red: LedPin::new(led_red),
            led_green: LedPin::new(led_green),
            led_blue: LedPin::new(led_blue),
        }
    }
}

#[cfg(feature = "defmt")]
impl<D, A0, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11, A12, A13, L> defmt::Format
    for ArduinoGigaPins<D, A0, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11, A12, A13, L>
{
    fn format(&self, formatter: defmt::Formatter) {
        defmt::write!(formatter, "ArduinoGigaPins")
    }
}

/// Consume an Embassy STM32 peripheral bundle into the runtime-neutral pin
/// wrapper without adding Embassy as a `giga-r1` dependency.
///
/// The macro is merely a board mapping adapter expanded in the application.
/// Import `embassy_stm32::gpio::Pin as _` before invoking it.
#[macro_export]
macro_rules! arduino_giga_pins {
    ($p:ident) => {
        $crate::pins::ArduinoGigaPins::new(
            [
                $p.PB7.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PA9.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PA3.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PA2.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PJ8.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PA7.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PD13.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PB4.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PB8.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PB9.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PK1.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PJ10.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PJ11.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PH6.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PG14.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PC7.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PH13.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PI9.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PD5.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PD6.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PB11.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PH4.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PJ12.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PG13.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PG12.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PJ0.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PJ14.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PJ1.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PJ15.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PJ2.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PK3.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PJ3.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PK4.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PJ4.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PK5.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PJ5.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PK6.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PJ6.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PJ7.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PI14.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PE6.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PK7.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PI15.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PI10.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PG10.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PI13.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PH15.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PB2.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PK0.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PE4.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PI11.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PE5.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PK2.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PG7.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PI5.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PH8.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PA6.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PJ9.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PI7.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PI6.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PI4.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PH14.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PG11.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PH11.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PH10.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PH9.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PA1.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PD4.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PC6.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PI0.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PI1.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PI2.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PI3.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PC1.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PB12.into::<embassy_stm32::gpio::AnyPin>(),
                $p.PD3.into::<embassy_stm32::gpio::AnyPin>(),
            ],
            $p.PC4,
            $p.PC5,
            $p.PB0,
            $p.PB1,
            $p.PC3,
            $p.PC2,
            $p.PC0,
            $p.PA0,
            $crate::pins::AnalogOnlyPin::<8>::new($crate::pins::PinId::new(
                $crate::pins::Port::C,
                2,
            )),
            $crate::pins::AnalogOnlyPin::<9>::new($crate::pins::PinId::new(
                $crate::pins::Port::C,
                3,
            )),
            $crate::pins::AnalogOnlyPin::<10>::new($crate::pins::PinId::new(
                $crate::pins::Port::A,
                1,
            )),
            $crate::pins::AnalogOnlyPin::<11>::new($crate::pins::PinId::new(
                $crate::pins::Port::A,
                0,
            )),
            $p.PA4,
            $p.PA5,
            $p.PI12.into::<embassy_stm32::gpio::AnyPin>(),
            $p.PJ13.into::<embassy_stm32::gpio::AnyPin>(),
            $p.PE3.into::<embassy_stm32::gpio::AnyPin>(),
        )
    };
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

/// CYW4343W Bluetooth HCI UART7 RX (STM32 receives on PA8).
pub const BLE_UART_RX: PinId = p(Port::A, 8);
/// CYW4343W Bluetooth HCI UART7 TX (STM32 transmits on PF7).
pub const BLE_UART_TX: PinId = p(Port::F, 7);
pub const BLE_UART_RTS: PinId = p(Port::F, 8);
pub const BLE_UART_CTS: PinId = p(Port::F, 9);
pub const BLE_POWER: PinId = p(Port::A, 10);
pub const BLE_HOST_WAKE: PinId = p(Port::G, 3);
pub const BLE_DEVICE_WAKE: PinId = p(Port::H, 7);

pub const SERIAL1_RX: PinId = p(Port::B, 7);
pub const SERIAL1_TX: PinId = p(Port::A, 9);
pub const SERIAL2_TX: PinId = p(Port::D, 5);
pub const SERIAL2_RX: PinId = p(Port::D, 6);
pub const SERIAL3_TX: PinId = p(Port::H, 13);
pub const SERIAL3_RX: PinId = p(Port::I, 9);
pub const SERIAL4_TX: PinId = p(Port::G, 14);
pub const SERIAL4_RX: PinId = p(Port::C, 7);

pub const SPI_CS: PinId = p(Port::K, 1);
pub const SPI_MOSI: PinId = p(Port::J, 10);
pub const SPI_MISO: PinId = p(Port::J, 11);
pub const SPI_SCK: PinId = p(Port::H, 6);

/// A12 / DAC1 channel 1.
pub const DAC0: PinId = p(Port::A, 4);
/// A13 / DAC1 channel 2.
pub const DAC1: PinId = p(Port::A, 5);

const fn p(port: Port, pin: u8) -> PinId {
    PinId::new(port, pin)
}
