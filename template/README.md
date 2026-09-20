# {{project-name}}

A GIGA R1 application starter for building dual-core firmware in the style of a
`giga-fx` project:

- `m7`: board owner, clock/tree setup, peripherals, Wi-Fi/network policy, UI/API serving, and M4 release.
- `m4`: independent worker core for deterministic DSP, compute, or real-time tasks.
- `web`: browser dashboard source and built assets for app control/status pages.
- `Embed.toml`: SWD flashing and RTT/defmt logging for the M7 image.

The starter sends typed `Ping`/`Pong` messages through `giga_r1::ipc::Channel` in
shared D3 SRAM. Green onboard LED means the M7 and M4 are communicating; red
means the worker did not reply.

## Prerequisites

Install Rust, Cargo Generate, and probe-rs tools:

```sh
cargo install cargo-generate
cargo install probe-rs-tools --locked
cargo install flip-link
```

Connect an SWD probe to the GIGA R1 SWD header. Cargo Embed cannot flash through
the normal Arduino USB bootloader connection.

## Create an application

From the `giga-r1-rs` repository:

```sh
cargo generate --path template --name my-giga-app
cd my-giga-app
```

When this template is hosted in a Git repository, pass `template` as the
positional subfolder argument:

```sh
cargo generate --git <repository-url> template --name my-giga-app
```

## Build firmware

Build each core in a separate Cargo invocation:

```sh
cargo build -p giga-m4 --release
cargo build -p giga-m7 --release
```

Do not use `cargo build --workspace`: M4 and M7 select mutually exclusive
`stm32h747xi-cm4` and `stm32h747xi-cm7` Embassy device features. Separate builds
preserve the correct PAC interrupt table and avoid handwritten `device.x` files.
Use `bind_interrupts!` only when adding a real interrupt-driven peripheral.

| Image                         |                  Flash |                              RAM |
| ----------------------------- | ---------------------: | -------------------------------: |
| Arduino bootloader (reserved) | `0x0800_0000`, 256 KiB |                                — |
| M7                            | `0x0804_0000`, 768 KiB | AXI SRAM, `0x2400_0000`, 512 KiB |
| M4                            |   `0x0810_0000`, 1 MiB |  D2 SRAM, `0x3000_0000`, 256 KiB |
| Typed IPC mailbox             |                      — |    D3 SRAM, `0x3800_0400`, 1 KiB |

Do not move images over the Arduino bootloader region unless replacing it is an
explicit application goal.

## Flash and run

Flash the M4 worker first, then flash and run the M7 controller:

```sh
cargo embed --path target/thumbv7em-none-eabihf/release/giga-m4
cargo embed --release -p giga-m7
```

Press reset after both images are programmed. The M7 debug port can program both
flash banks; the custom `GIGA_R1_M7` target intentionally exposes only the M7 AP
because Arduino option bytes normally hold M4 until the M7 releases it.

For later M7-only changes, `cargo run --release -p giga-m7` uses Cargo Embed via
`.cargo/config.toml`. Reflash M4 whenever `m4` changes.

## Web dashboard layout

Put frontend source in `web/src/` and production assets in `web/dist/`. The M7
firmware should serve built dashboard assets and expose device controls under
`/api/*` when the application adds Wi-Fi/network and HTTP support.

Recommended firmware contract:

- static dashboard files: `GET /`, `GET /assets/*`, or equivalent;
- status/control API: `/api/*` JSON or binary endpoints chosen by the app;
- M7 owns networking, HTTP routing, and board peripherals;
- M4 owns deterministic worker tasks and communicates with M7 through typed IPC
  or a larger `giga_r1::ipc::SharedQueue` region for bulk data.

This template intentionally does not choose a JavaScript framework, allocator,
network stack, or HTTP server. Add those at the application layer.

## Start building

- Put board initialization, Wi-Fi/network policy, audio/control peripherals, and
  dashboard/API serving in `m7/src/main.rs`.
- Put deterministic compute, DSP, or real-time worker code in `m4/src/main.rs`.
- Keep typed IPC state in `.ipc_mailbox` and use `giga_r1::ipc::IpcMailbox` /
  `Channel`; ordinary statics are not automatically shared safely across the
  cache boundary.
- For bulk binary traffic, use `giga_r1::ipc::SharedQueue` in an explicit larger
  D3 SRAM linker region; do not enlarge the postcard RPC mailbox.

See the main [`giga-r1`](https://github.com/anapeksha/giga-r1-rs) repository for
Arduino pin mappings and Wi-Fi, BLE, USB, CAN, QSPI, raw audio buffers, typed
dual-core IPC, and shared-queue bulk IPC examples.
