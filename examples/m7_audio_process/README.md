# M7 audio process

This example demonstrates the BSP's HAL-neutral raw audio buffer model. It owns
the Arduino GIGA R1 raw analog route:

- input: `A0` / `PC4` / ADC1 channel 4
- left/mono output: `A12` / `PA4` / DAC1 channel 1
- right output: `A13` / `PA5` / DAC1 channel 2

The example fills an interleaved stereo input buffer, applies gain plus soft
clipping into the output buffer, and leaves clear insertion points for an
application effect chain such as `giga-fx`.

It does not claim high-quality onboard audio. Real audio hardware should add
biasing, input protection, anti-alias filtering, output reconstruction filtering,
amplification, and a HAL timer/DMA setup appropriate for the desired sample
rate.

Build:

```sh
cargo build -p m7-audio-process --release
```
