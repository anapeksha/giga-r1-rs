# arduino-giga

`arduino-giga` is a Rust-native, HAL-agnostic `no_std` board support package
for the Arduino GIGA R1 WiFi and its dual-core STM32H747XI (Cortex-M7 +
Cortex-M4).

The public crate is built on `embedded-hal` traits and does not force an MCU
HAL or runtime on applications. Workspace examples use
[`embassy-stm32`](https://crates.io/crates/embassy-stm32), with each firmware
image kept in its own root package and memory map.

```toml
[dependencies]
arduino-giga = "0.1"
```

Optional hardware and integration features are disabled by default:

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

## Dual-core memory map

| Owner | Flash | RAM |
|---|---:|---:|
| Bootloader | `0x0800_0000`, 256 KiB reserved | — |
| Cortex-M7 application | `0x0804_0000`, 768 KiB | AXI SRAM at `0x2400_0000`, 512 KiB |
| Cortex-M4 | `0x0810_0000`, 1 MiB | D2 SRAM at `0x3000_0000`, 256 KiB |
| Shared | — | D3 SRAM at `0x3800_0000`, first 1 KiB |

The M7 is responsible for clock-tree setup and releasing the M4. Both images
place Embassy's dual-core coordination object at the same D3 SRAM address.

## Source references

The board map is derived from Arduino's official GIGA R1 WiFi schematic,
datasheet, pinout, and the GIGA variant in ArduinoCore-mbed.

## License

MIT
