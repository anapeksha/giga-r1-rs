//! Board-level raw audio routing and buffer helpers.
//!
//! The Arduino GIGA R1 WiFi does not provide a dedicated onboard audio codec in
//! the BSP-owned path. The lowest-level board route available today is the
//! STM32H747 analog path: ADC-capable Arduino analog inputs for capture and the
//! two DAC outputs on `A12`/`A13` for playback. This module documents that route
//! and provides a HAL-neutral ownership and buffer model that applications can
//! build on with their chosen ADC, DAC, timer, DMA, executor, or external codec
//! driver.
//!
//! The BSP deliberately does not implement an effects engine. Applications such
//! as `giga-fx` should receive an [`AudioBuffer`], process `input` into
//! `output`, and own their DSP chain, scheduling policy, and transport.
//!
//! # Ownership model
//!
//! [`AudioDevice`] owns the application-supplied peripheral and pin tokens. The
//! tokens remain fully typed, so the consuming HAL can retain ADC-channel,
//! DAC-channel, DMA, or alternate-function capabilities. Use [`AudioDevice::release`]
//! to recover those resources and reconfigure them.
//!
//! # Buffer model
//!
//! Buffers are interleaved by frame. For stereo `i16`, the layout is:
//!
//! ```text
//! left0, right0, left1, right1, ...
//! ```
//!
//! [`AudioBuffer`] never allocates and never stores references beyond the call
//! that receives it. The application supplies backing storage and is responsible
//! for keeping DMA/cache rules consistent with the HAL it uses. Future DMA
//! integrations can drive the same explicit buffer-handoff model without
//! changing the application DSP callback shape.

use crate::pins::{PinId, Port};

/// Raw analog audio-capable board route exposed by the GIGA R1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AudioRoute {
    /// Preferred raw analog input for examples: Arduino `A0`, STM32 `PC4`, ADC1 channel 4.
    pub input: PinId,
    /// Left/mono raw DAC output: Arduino `A12`, STM32 `PA4`, DAC1 channel 1.
    pub output_left: PinId,
    /// Right raw DAC output: Arduino `A13`, STM32 `PA5`, DAC1 channel 2.
    pub output_right: PinId,
}

/// Default raw analog route used by the audio examples.
///
/// This route is suitable for low-level signal experiments and externally
/// conditioned audio. It is not a complete line-level audio codec path: the
/// application must provide biasing, anti-alias filtering, output filtering,
/// amplification, protection, and timing/DMA setup appropriate for its hardware.
pub const ANALOG_AUDIO: AudioRoute = AudioRoute {
    input: PinId::new(Port::C, 4),
    output_left: PinId::new(Port::A, 4),
    output_right: PinId::new(Port::A, 5),
};

/// Application-supplied audio peripheral tokens.
///
/// The BSP owns these tokens only to make the board route explicit. It does not
/// erase their concrete HAL types or configure clocks, timers, DMA streams, ADCs,
/// or DACs behind the application's back.
pub struct AudioPeripherals<ADC, DAC> {
    pub adc: ADC,
    pub dac: DAC,
}

impl<ADC, DAC> AudioPeripherals<ADC, DAC> {
    pub const fn new(adc: ADC, dac: DAC) -> Self {
        Self { adc, dac }
    }
}

/// Application-supplied pin tokens for the raw analog audio route.
pub struct AudioPins<In, OutL, OutR> {
    pub input: In,
    pub output_left: OutL,
    pub output_right: OutR,
}

impl<In, OutL, OutR> AudioPins<In, OutL, OutR> {
    pub const fn new(input: In, output_left: OutL, output_right: OutR) -> Self {
        Self {
            input,
            output_left,
            output_right,
        }
    }
}

/// Owned audio resources returned by [`AudioDevice::release`].
pub struct AudioParts<Adc, Dac, In, OutL, OutR> {
    pub peripherals: AudioPeripherals<Adc, Dac>,
    pub pins: AudioPins<In, OutL, OutR>,
}

/// Interleaved channel layout used by buffers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ChannelLayout {
    /// One sample per frame.
    Mono,
    /// Two samples per frame, left then right.
    Stereo,
}

impl ChannelLayout {
    pub const fn channels(self) -> usize {
        match self {
            Self::Mono => 1,
            Self::Stereo => 2,
        }
    }
}

/// Sample representation used by an application pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SampleFormat {
    /// Signed 16-bit PCM.
    I16,
    /// Signed 24-bit PCM stored in the most significant 24 bits of an `i32`.
    I24InI32,
    /// Normalized floating point samples. The BSP does not require a floating
    /// point DSP pipeline; this variant documents buffers supplied by apps that do.
    F32,
}

/// Runtime-neutral audio configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AudioConfig {
    /// Nominal sample rate requested by the application.
    pub sample_rate_hz: u32,
    /// Interleaved channel layout.
    pub channels: ChannelLayout,
    /// Sample representation in input/output buffers.
    pub sample_format: SampleFormat,
    /// Number of frames handed to the application at a time.
    pub frames_per_buffer: usize,
}

impl AudioConfig {
    /// Conservative default for raw analog experiments.
    pub const fn new(
        sample_rate_hz: u32,
        channels: ChannelLayout,
        sample_format: SampleFormat,
        frames_per_buffer: usize,
    ) -> Self {
        Self {
            sample_rate_hz,
            channels,
            sample_format,
            frames_per_buffer,
        }
    }

    /// Return the number of samples in one interleaved buffer.
    pub const fn samples_per_buffer(self) -> usize {
        self.frames_per_buffer * self.channels.channels()
    }
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self::new(48_000, ChannelLayout::Stereo, SampleFormat::I16, 64)
    }
}

/// Audio buffer and stream-state failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum AudioError {
    /// A buffer length is not an integer number of interleaved frames.
    InvalidBufferLength,
    /// Input and output buffers have different frame counts.
    BufferLengthMismatch,
    /// A buffer's interleaved layout does not match the configured stream layout.
    ChannelLayoutMismatch {
        expected: ChannelLayout,
        actual: ChannelLayout,
    },
    /// The supplied buffers do not match [`AudioConfig::frames_per_buffer`].
    UnexpectedFrameCount { expected: usize, actual: usize },
    /// The producer overwrote an input buffer before it was processed.
    Overrun,
    /// The consumer needed output before the application filled it.
    Underrun,
    /// The selected route or sample format is not supported by the active HAL setup.
    Unsupported,
}

/// Raw interleaved audio buffers handed to application DSP code.
pub struct AudioBuffer<'a, T> {
    pub input: &'a [T],
    pub output: &'a mut [T],
    pub channels: ChannelLayout,
}

impl<'a, T> AudioBuffer<'a, T> {
    pub fn new(
        input: &'a [T],
        output: &'a mut [T],
        channels: ChannelLayout,
    ) -> Result<Self, AudioError> {
        let buffer = Self {
            input,
            output,
            channels,
        };
        buffer.validate()?;
        Ok(buffer)
    }

    /// Validate that input and output contain matching complete frames.
    pub fn validate(&self) -> Result<(), AudioError> {
        let channels = self.channels.channels();
        if channels == 0 || !self.input.len().is_multiple_of(channels) {
            return Err(AudioError::InvalidBufferLength);
        }
        if !self.output.len().is_multiple_of(channels) {
            return Err(AudioError::InvalidBufferLength);
        }
        if self.input.len() != self.output.len() {
            return Err(AudioError::BufferLengthMismatch);
        }
        Ok(())
    }

    /// Number of interleaved frames in the buffer.
    pub fn frames(&self) -> Result<usize, AudioError> {
        self.validate()?;
        Ok(self.input.len() / self.channels.channels())
    }
}

/// HAL-neutral owner for the configured GIGA audio route.
///
/// `AudioDevice` intentionally does not start hidden interrupts, allocate memory,
/// or pick a DMA strategy. It stores the explicit route/configuration and offers
/// a predictable callback-oriented buffer handoff for application DSP. A HAL
/// integration can wrap this type and use the same [`AudioBuffer`] contract for
/// ping-pong DMA completion callbacks or async tasks.
pub struct AudioDevice<Adc, Dac, In, OutL, OutR> {
    config: AudioConfig,
    route: AudioRoute,
    parts: AudioParts<Adc, Dac, In, OutL, OutR>,
    overruns: u32,
    underruns: u32,
}

impl<Adc, Dac, In, OutL, OutR> AudioDevice<Adc, Dac, In, OutL, OutR> {
    pub const fn new(
        config: AudioConfig,
        peripherals: AudioPeripherals<Adc, Dac>,
        pins: AudioPins<In, OutL, OutR>,
    ) -> Self {
        Self {
            config,
            route: ANALOG_AUDIO,
            parts: AudioParts { peripherals, pins },
            overruns: 0,
            underruns: 0,
        }
    }

    pub const fn config(&self) -> AudioConfig {
        self.config
    }

    pub const fn route(&self) -> AudioRoute {
        self.route
    }

    pub const fn overruns(&self) -> u32 {
        self.overruns
    }

    pub const fn underruns(&self) -> u32 {
        self.underruns
    }

    pub fn record_overrun(&mut self) {
        self.overruns = self.overruns.saturating_add(1);
    }

    pub fn record_underrun(&mut self) {
        self.underruns = self.underruns.saturating_add(1);
    }

    /// Process one complete application-owned buffer.
    ///
    /// The closure is where downstream applications insert gain, filtering,
    /// effects, encoding, transport, or any other policy. The BSP checks the
    /// interleaved shape and configured frame count, then calls the closure with
    /// direct access to the input and output slices.
    pub fn process_buffer<T>(
        &mut self,
        buffer: AudioBuffer<'_, T>,
        process: impl FnOnce(&[T], &mut [T]),
    ) -> Result<(), AudioError> {
        if buffer.channels != self.config.channels {
            return Err(AudioError::ChannelLayoutMismatch {
                expected: self.config.channels,
                actual: buffer.channels,
            });
        }
        let frames = buffer.frames()?;
        if frames != self.config.frames_per_buffer {
            return Err(AudioError::UnexpectedFrameCount {
                expected: self.config.frames_per_buffer,
                actual: frames,
            });
        }
        process(buffer.input, buffer.output);
        Ok(())
    }

    /// Release all owned peripherals and pins back to the application.
    pub fn release(self) -> AudioParts<Adc, Dac, In, OutL, OutR> {
        self.parts
    }
}
