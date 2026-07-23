# giga-r1

[![crates.io](https://img.shields.io/crates/v/giga-r1.svg)](https://crates.io/crates/giga-r1)
[![docs.rs](https://docs.rs/giga-r1/badge.svg)](https://docs.rs/giga-r1)
[![CI](https://github.com/anapeksha/giga-r1-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/anapeksha/giga-r1-rs/actions/workflows/ci.yml)
[![license](https://img.shields.io/crates/l/giga-r1.svg)](https://github.com/anapeksha/giga-r1-rs/blob/main/LICENSE)

`giga-r1` is a Rust-native, HAL-agnostic `no_std` board support package
for the Arduino GIGA R1 WiFi and its dual-core STM32H747XI (Cortex-M7 +
Cortex-M4).

The public crate is built on `embedded-hal` and `embedded-hal-async` traits and
does not force an MCU HAL, executor, or application runtime. Board controls own
their resources, perform board-specific initialization, and expose
`release`/owned-part handoffs. Workspace examples use
[`embassy-stm32`](https://crates.io/crates/embassy-stm32) where useful, while
the M4 bridge example demonstrates a runtime-independent application.

```toml
[dependencies]
giga-r1 = "0.1"
```

Optional hardware features are disabled by default:

- `defmt`
- `audio`
- `camera`
- `can`
- `display`
- `qspi`
- `sdcard`
- `usb`
- `wifi`

Build the first M7 board test:

```sh
cargo build -p m7-rgb-blinky --features defmt --release
```

Flash and monitor it with the attached probe:

```sh
cargo embed -p m7-rgb-blinky --features defmt --release
```

The onboard RGB LED is active-low. The test displays red, green, and blue in
sequence.

Wi-Fi initialization owns the GIGA power sequence, CYW4343W firmware, NVRAM,
and country data. Because the CYW43 runner consumes itself and must be polled
continuously, the crate returns that runner to the application for execution by
its chosen runtime; the network device and control channel remain
application-owned.

## Dual-core memory map

| Owner | Flash | RAM |
|---|---:|---:|
| Bootloader | `0x0800_0000`, 256 KiB reserved | — |
| Cortex-M7 application | `0x0804_0000`, 768 KiB | AXI SRAM at `0x2400_0000`, 512 KiB |
| Cortex-M4 | `0x0810_0000`, 1 MiB | D2 SRAM at `0x3000_0000`, 256 KiB |
| Shared | — | D3 SRAM at `0x3800_0000`, first 1 KiB |

The M7 is responsible for clock-tree setup and releasing the M4. Both images
communicate through the crate's shared mailbox in D3 SRAM.

## Source references

The board map is derived from Arduino's official GIGA R1 WiFi schematic,
datasheet, pinout, and the GIGA variant in ArduinoCore-mbed.

## License

MIT

The bundled CYW4343W firmware uses Cypress's separate
[Permissive Binary License 1.0](https://github.com/anapeksha/giga-r1-rs/blob/main/LICENSE),
reproduced after the MIT terms.
