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

## Option-byte recovery

[`tools/set-giga-option-bytes.S`](https://github.com/anapeksha/giga-r1-rs/blob/main/tools/set-giga-option-bytes.S)
is an advanced, one-time recovery helper that restores the Arduino GIGA boot
policy: the Cortex-M7 boots automatically while the M4 remains held until the
M7 supplies its boot vector and releases it. The helper runs from AXI SRAM,
unlocks option-byte programming, writes Arduino's option value, waits for
programming to finish, and stops at a debugger breakpoint; reset or power-cycle
the board afterward to load the new options.

This helper is not part of normal Cargo Embed flashing and is unnecessary once
the option bytes are correct. It replaces the complete option register with a
hard-coded board value, so use it only to recover a GIGA R1 whose boot options
were changed.

## Source references

The board map is derived from Arduino's official GIGA R1 WiFi schematic,
datasheet, pinout, and the GIGA variant in ArduinoCore-mbed.

## License

Original source code is licensed under the [MIT License](LICENSE); the bundled CYW4343W firmware is distributed separately under the Cypress Permissive Binary License 1.0, whose terms are reproduced in the same file.
