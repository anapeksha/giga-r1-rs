#![no_std]
#![no_main]

use core::mem::MaybeUninit;

use cortex_m_rt::entry;
#[cfg(feature = "defmt")]
use defmt_rtt as _;
use embassy_stm32::{
    SharedData,
    gpio::{Level, Output, Speed},
    rcc::mux::{Adcsel, Persel},
};
use giga_r1::{
    audio::{
        AudioBuffer, AudioConfig, AudioDevice, AudioPeripherals, AudioPins, ChannelLayout,
        SampleFormat,
    },
    led::{Color, RgbLed},
};
use panic_halt as _;

#[allow(unsafe_code)]
#[unsafe(link_section = ".shared_data")]
static SHARED_DATA: MaybeUninit<SharedData> = MaybeUninit::uninit();

const FRAMES: usize = 64;
const CHANNELS: usize = 2;
const SAMPLES: usize = FRAMES * CHANNELS;

#[entry]
fn main() -> ! {
    let mut mcu_config = embassy_stm32::Config::default();
    mcu_config.rcc.mux.persel = Persel::HSI;
    mcu_config.rcc.mux.adcsel = Adcsel::PER;
    let p = embassy_stm32::init_primary(mcu_config, &SHARED_DATA);

    let mut led = RgbLed::new(
        Output::new(p.PI12, Level::High, Speed::Low),
        Output::new(p.PJ13, Level::High, Speed::Low),
        Output::new(p.PE3, Level::High, Speed::Low),
    )
    .unwrap();

    let config = AudioConfig::new(48_000, ChannelLayout::Stereo, SampleFormat::I16, FRAMES);
    let mut audio = AudioDevice::new(
        config,
        AudioPeripherals::new(p.ADC1, p.DAC1),
        AudioPins::new(p.PC4, p.PA4, p.PA5),
    );

    let mut phase = 0_i16;
    loop {
        let mut input = [0_i16; SAMPLES];
        let mut output = [0_i16; SAMPLES];

        // In a real application, a HAL timer/DMA completion would hand over an
        // ADC-filled input buffer and an empty DAC output buffer here. This
        // synthetic stereo ramp keeps the example independent of a particular
        // DMA API while exercising the same buffer contract.
        for frame in 0..FRAMES {
            let left = frame * CHANNELS;
            input[left] = phase;
            input[left + 1] = phase.saturating_neg();
            phase = phase.wrapping_add(512);
        }

        let buffer = AudioBuffer::new(&input, &mut output, ChannelLayout::Stereo).unwrap();
        let processed = audio.process_buffer(buffer, |input, output| {
            for (source, destination) in input.iter().zip(output.iter_mut()) {
                *destination = soft_clip(source.saturating_mul(2));
            }
        });

        match processed {
            Ok(()) => {
                led.set(Color::Green).unwrap();
                #[cfg(feature = "defmt")]
                defmt::info!("processed {} stereo audio frames", FRAMES);
            }
            Err(error) => {
                let _ = error;
                audio.record_underrun();
                led.set(Color::Red).unwrap();
                #[cfg(feature = "defmt")]
                defmt::warn!("audio processing failed: {}", error);
            }
        }

        // `output` is where an application would queue DAC/I2S/SAI/USB/network
        // playback or pass samples into its next transport layer.
        cortex_m::asm::delay(30_000_000);
        led.off().unwrap();
        cortex_m::asm::delay(10_000_000);
    }
}

fn soft_clip(sample: i16) -> i16 {
    const KNEE: i16 = 24_000;
    if sample > KNEE {
        KNEE + (sample - KNEE) / 4
    } else if sample < -KNEE {
        -KNEE + (sample + KNEE) / 4
    } else {
        sample
    }
}
