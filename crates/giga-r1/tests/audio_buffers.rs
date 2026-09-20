use giga_r1::audio::{
    ANALOG_AUDIO, AudioBuffer, AudioConfig, AudioDevice, AudioError, AudioPeripherals, AudioPins,
    ChannelLayout, SampleFormat,
};
use giga_r1::pins::{PinId, Port};

#[test]
fn reports_expected_board_route() {
    assert_eq!(ANALOG_AUDIO.input, PinId::new(Port::C, 4));
    assert_eq!(ANALOG_AUDIO.output_left, PinId::new(Port::A, 4));
    assert_eq!(ANALOG_AUDIO.output_right, PinId::new(Port::A, 5));
}

#[test]
fn computes_interleaved_buffer_shape() {
    let config = AudioConfig::new(48_000, ChannelLayout::Stereo, SampleFormat::I16, 64);
    assert_eq!(config.channels.channels(), 2);
    assert_eq!(config.samples_per_buffer(), 128);

    let input = [0_i16; 128];
    let mut output = [0_i16; 128];
    let buffer = AudioBuffer::new(&input, &mut output, ChannelLayout::Stereo).unwrap();
    assert_eq!(buffer.frames(), Ok(64));
}

#[test]
fn rejects_bad_buffer_shapes() {
    let input = [0_i16; 3];
    let mut output = [0_i16; 3];
    assert_eq!(
        AudioBuffer::new(&input, &mut output, ChannelLayout::Stereo).err(),
        Some(AudioError::InvalidBufferLength)
    );

    let input = [0_i16; 4];
    let mut output = [0_i16; 2];
    assert_eq!(
        AudioBuffer::new(&input, &mut output, ChannelLayout::Stereo).err(),
        Some(AudioError::BufferLengthMismatch)
    );
}

#[test]
fn process_buffer_applies_application_callback() {
    let config = AudioConfig::new(48_000, ChannelLayout::Mono, SampleFormat::I16, 4);
    let mut device = AudioDevice::new(
        config,
        AudioPeripherals::new("adc", "dac"),
        AudioPins::new("a0", "a12", "a13"),
    );

    let input = [100_i16, -200, 300, -400];
    let mut output = [0_i16; 4];
    let buffer = AudioBuffer::new(&input, &mut output, ChannelLayout::Mono).unwrap();
    device
        .process_buffer(buffer, |input, output| {
            for (source, destination) in input.iter().zip(output.iter_mut()) {
                *destination = source.saturating_mul(2);
            }
        })
        .unwrap();

    assert_eq!(output, [200, -400, 600, -800]);
}

#[test]
fn process_buffer_checks_channel_layout() {
    let config = AudioConfig::new(48_000, ChannelLayout::Stereo, SampleFormat::I16, 4);
    let mut device = AudioDevice::new(
        config,
        AudioPeripherals::new((), ()),
        AudioPins::new((), (), ()),
    );

    let input = [0_i16; 4];
    let mut output = [0_i16; 4];
    let buffer = AudioBuffer::new(&input, &mut output, ChannelLayout::Mono).unwrap();
    assert_eq!(
        device.process_buffer(buffer, |_, _| {}).err(),
        Some(AudioError::ChannelLayoutMismatch {
            expected: ChannelLayout::Stereo,
            actual: ChannelLayout::Mono
        })
    );
}

#[test]
fn process_buffer_checks_configured_frame_count() {
    let config = AudioConfig::new(48_000, ChannelLayout::Mono, SampleFormat::I16, 8);
    let mut device = AudioDevice::new(
        config,
        AudioPeripherals::new((), ()),
        AudioPins::new((), (), ()),
    );

    let input = [0_i16; 4];
    let mut output = [0_i16; 4];
    let buffer = AudioBuffer::new(&input, &mut output, ChannelLayout::Mono).unwrap();
    assert_eq!(
        device.process_buffer(buffer, |_, _| {}).err(),
        Some(AudioError::UnexpectedFrameCount {
            expected: 8,
            actual: 4
        })
    );
}

#[test]
fn tracks_overrun_and_underrun_counters() {
    let mut device = AudioDevice::new(
        AudioConfig::default(),
        AudioPeripherals::new((), ()),
        AudioPins::new((), (), ()),
    );

    device.record_overrun();
    device.record_underrun();
    device.record_underrun();

    assert_eq!(device.overruns(), 1);
    assert_eq!(device.underruns(), 2);
}
