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
giga-r1 = "0.2"
```

Optional hardware features are disabled by default:

- `defmt`
- `audio`
- `ble`
- `camera`
- `can`
- `display`
- `dual-core`
- `ipc` (enables `dual-core` and the optional `postcard`/`serde` codec)
- `qspi`
- `sdcard`
- `usb`
- `wifi`

## Arduino pin names

`ArduinoGigaPins` owns D0–D75, A0–A13, and the active-low RGB channels using
the labels printed on the board. Its pin types remain supplied by the
application HAL, and configured pins implement the standard `embedded-hal`
digital traits. The Embassy mapping adapter expands in the consuming
application, so Embassy is not a dependency of `giga-r1`:

```rust,ignore
let board = giga_r1::arduino_giga_pins!(peripherals);
let mut d13 = board
    .digital
    .d13
    .map(|pin| embassy_stm32::gpio::Output::new(
        pin,
        embassy_stm32::gpio::Level::Low,
        embassy_stm32::gpio::Speed::Low,
    ));
let a0 = board.analog.a0.into_inner();
```

The adapter preserves concrete analog channel types rather than erasing them
to GPIO. A8–A11 are identified as the H747's analog-only companion pads,
which Embassy does not currently expose as owned GPIO peripheral tokens.

Build the first M7 board test:

```sh
cargo build -p m7-rgb-blinky --features defmt --release
```

Flash and monitor it with the attached probe:

```sh
cargo embed -p m7-rgb-blinky --features defmt --release
```

`Embed.toml` loads the repository's `GIGA_R1_M7` target description. It exposes
only the M7 debug access port because Arduino's normal option-byte policy holds
the M4 in reset until the application releases it; generic dual-core probe
descriptions otherwise try to halt the unavailable M4 and time out.

The onboard RGB LED is active-low. The test displays red, green, and blue in
sequence.

Wi-Fi initialization owns the GIGA power sequence, CYW4343W firmware, NVRAM,
and country data. Because the CYW43 runner consumes itself and must be polled
continuously, the crate returns that runner to the application for execution by
its chosen runtime; the network device and control channel remain
application-owned.

## Bluetooth Low Energy

The `ble` feature owns the independent CYW4343W Bluetooth power and wake
sequence, downloads the bundled Cypress HCI patchram, resets the controller,
and returns the initialized HCI UART plus all control pins to the application.
Bluetooth uses UART7 at 115,200 baud with hardware flow control: PF7/PA8 are
TX/RX, PF8/PF9 are RTS/CTS, PA10 is power, PG3 is host-wake, and PH7 is
device-wake. This is separate from the Wi-Fi SDIO runner, so BLE may run alone
or alongside Wi-Fi.

The BSP stops at the runtime-neutral, initialized HCI ownership boundary.
[`examples/m7_ble_beacon`](examples/m7_ble_beacon) demonstrates a complete
Eddystone-compatible beacon with `bt-hci` and `trouble-host`; another
application may choose a different async executor or BLE host.

## Dual-core memory map

| Owner | Flash | RAM |
|---|---:|---:|
| Bootloader | `0x0800_0000`, 256 KiB reserved | — |
| Cortex-M7 application | `0x0804_0000`, 768 KiB | AXI SRAM at `0x2400_0000`, 512 KiB |
| Cortex-M4 | `0x0810_0000`, 1 MiB | D2 SRAM at `0x3000_0000`, 256 KiB |
| Shared | — | D3 SRAM at `0x3800_0000`, first 1 KiB |

The M7 is responsible for clock-tree setup and releasing the M4. Both images
communicate through the crate's shared mailbox in D3 SRAM.

With the `ipc` feature, `Channel<T, R>` serializes typed requests and responses
into an allocation-free 256-byte postcard frame. Atomic word storage and
release/acquire publication make the single-client/single-worker mailbox safe
across the cache boundary. Notification is pluggable; the included
`Polling` policy needs no peripheral or runtime, while `Notify` can be
implemented with a configured HSEM interrupt. A plain Cortex `SEV` is only a
local-core event and is not treated as a reliable inter-core doorbell.
Synchronous callers supply their own idle/poll hook, while
`Channel::call` accepts a runtime-provided `AsyncWait` implementation for an
HSEM interrupt, event listener, or timer without making the crate
executor-dependent. [`examples/dual_core_postcard_ipc`](examples/dual_core_postcard_ipc)
offloads a fixed-point eight-point FFT power spectrum to the M4.

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
