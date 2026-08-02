# {{project-name}}

A minimal dual-core Rust starter for the Arduino GIGA R1 WiFi's STM32H747XI:

- `m7`: initializes the board and clock tree, releases the Cortex-M4, and shows inter-core status on the onboard RGB LED.
- `m4`: runs a small mailbox worker in parallel on the Cortex-M4.
- `Embed.toml`: flashes either image through SWD with Cargo Embed and supports RTT/defmt output from the M7.

The M7 sends an incrementing ping through shared D3 SRAM. The M4 replies. A blinking **green** onboard LED means both cores are running and communicating; **red** means the M4 did not reply.

## Prerequisites

Install Rust (via rustup), Cargo Generate, and the probe-rs tools:

```sh
cargo install cargo-generate
cargo install probe-rs-tools --locked
cargo install flip-link
```

Connect an SWD probe to the GIGA R1's SWD header. Cargo Embed cannot flash through the board's normal USB bootloader connection.

## Create a project

From the `giga-r1-rs` repository:

```sh
cargo generate --path template --name my-giga-app
cd my-giga-app
```

When this template is hosted in a Git repository, it can also be generated with `cargo generate --git <repository-url> --subfolder template`.

## Build

Build each core in a separate Cargo invocation:

```sh
cargo build -p giga-m4 --release
cargo build -p giga-m7 --release
```

Do not use `cargo build --workspace`: M4 and M7 select the mutually exclusive
`stm32h747xi-cm4` and `stm32h747xi-cm7` Embassy device features. Separate
invocations give each image the correct PAC interrupt table, so no handwritten
`device.x` or interrupt vector table is needed. Use `bind_interrupts!` in the
relevant core when adding an interrupt-driven peripheral.

The images use the same `thumbv7em-none-eabihf` Rust target, but distinct linker maps:

| Image | Flash | RAM |
|---|---:|---:|
| Arduino bootloader (reserved) | `0x0800_0000`, 256 KiB | — |
| M7 | `0x0804_0000`, 768 KiB | AXI SRAM, `0x2400_0000`, 512 KiB |
| M4 | `0x0810_0000`, 1 MiB | D2 SRAM, `0x3000_0000`, 256 KiB |
| Shared mailbox | — | D3 SRAM, `0x3800_0400`, 1 KiB |

Do not move the images over the reserved Arduino bootloader region unless you intentionally want to replace it.

## Flash and run

Flash the M4 worker first, then flash and run the M7 controller:

```sh
cargo embed --path target/thumbv7em-none-eabihf/release/giga-m4
cargo embed --release -p giga-m7
```

Press the board's reset button after both images are programmed. The second command also displays M7 defmt logs over RTT. The custom `GIGA_R1_M7` probe-rs target intentionally exposes only the M7 debug access port: Arduino's normal option bytes hold the M4 in reset until the M7 application releases it, while the M7 can still program both flash banks.

For subsequent M7-only changes, `cargo run --release -p giga-m7` uses Cargo Embed through `.cargo/config.toml`. Reflash the M4 image whenever `m4` changes.

## Start building

- Put board initialization and peripheral ownership in `m7/src/main.rs`.
- Put independent compute or real-time work in `m4/src/main.rs`.
- Build the two core packages separately so Cargo does not unify their incompatible Embassy device features.
- Keep shared data in the `.bridge_mailbox` linker section and use the `giga-r1` bridge/IPC APIs; ordinary statics are not automatically shared safely across the cache boundary.
- For typed request/response messages, enable the `giga-r1` crate's `ipc` feature and use `giga_r1::ipc::Channel` with `serde` message types.

See the main [`giga-r1`](https://github.com/anapeksha/giga-r1-rs) repository for Arduino pin mappings and Wi-Fi, BLE, USB, CAN, QSPI, ADC, DAC, PWM, and typed dual-core IPC examples.
