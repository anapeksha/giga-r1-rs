# AGENTS.md

Guidance for AI coding agents working in this repository. This file applies to the entire repository unless a more specific `AGENTS.md` exists in a subdirectory.

## Project purpose

`giga-r1-rs` is a Rust-native, `no_std`, HAL-agnostic board support package (BSP) for the Arduino GIGA R1 WiFi and its heterogeneous STM32H747XI:

- Cortex-M7: primary core, board and clock initialization, application orchestration.
- Cortex-M4: secondary worker core, released by M7 after shared state is initialized.
- CYW4343W: independent Wi-Fi (SDIO) and Bluetooth (UART) functions.

The public `giga-r1` crate provides board-level ownership, routing, initialization, and dual-core communication. It must not become an MCU HAL, application runtime, executor, network stack, or BLE host. Embassy is used by examples and the generated template, not imposed on the base BSP.

## Repository map

| Path | Purpose |
|---|---|
| `crates/giga-r1/` | Published HAL- and runtime-neutral BSP crate |
| `crates/giga-r1/src/lib.rs` | Public module and feature surface |
| `crates/giga-r1/src/board.rs` | Board/core metadata |
| `crates/giga-r1/src/pins.rs` | Arduino pin ownership and application-side Embassy adapter |
| `crates/giga-r1/src/led.rs` | Active-low RGB LED ownership API |
| `crates/giga-r1/src/bridge.rs` | Raw lock-free dual-core mailbox and M7 MPU setup |
| `crates/giga-r1/src/ipc.rs` | Typed allocation-free postcard IPC |
| `crates/giga-r1/src/{wifi,ble}.rs` | CYW4343W board initialization and ownership boundaries |
| `crates/giga-r1/src/{can,qspi,usb}.rs` | Board routing metadata and typed resources |
| `crates/giga-r1/src/firmware/` | Licensed bundled radio firmware/data |
| `examples/` | Independently buildable hardware and dual-core firmware packages |
| `examples/dual_core_postcard_ipc/` | Typed M7/M4 IPC reference implementation |
| `template/` | Cargo Generate dual-core starter; excluded from the root workspace |
| `.cargo/config.toml` | Embedded target, linker, and Cargo Embed runner |
| `Embed.toml` | Root probe-rs/Cargo Embed behavior |
| `tools/giga-r1-m7.yaml` | Custom probe-rs target exposing only the usable M7 debug AP |
| `tools/set-giga-option-bytes.S` | Dangerous, advanced option-byte recovery helper |
| `.github/workflows/` | CI and release contract |

## Non-negotiable architecture

### Base BSP boundaries

- Keep `crates/giga-r1` `#![no_std]`.
- Keep the base crate HAL-agnostic and executor/runtime-neutral.
- Public hardware interfaces should use `embedded-hal`, `embedded-hal-async`, or `embedded-io-async` traits where appropriate.
- Do not add `embassy-stm32` to the public BSP merely because an example uses Embassy.
- Constructors should consume resources they own. Return ownership through `release`, `into_parts`, or an owned parts structure.
- Preserve concrete pin/peripheral types and capabilities. Do not erase analog-capable types to generic GPIO for convenience.
- Keep expensive dependencies and firmware assets behind the relevant optional feature.
- Default-feature-free compilation must remain valid.
- Avoid heap allocation in BSP code, bridge code, IPC, and startup paths.

### Feature model

The BSP has no default features. Current feature relationships are defined in `crates/giga-r1/Cargo.toml`:

- `dual-core` enables Cortex-M bridge support.
- `ipc` enables `dual-core`, `postcard`, and `serde`.
- `wifi` enables CYW43 and async HAL support.
- `ble` enables async HAL/I/O support.
- `defmt` adds optional formatting/logging and propagates to CYW43 when present.
- `can`, `qspi`, and `usb` expose board routing modules.
- `audio`, `camera`, `display`, and `sdcard` are reserved feature boundaries even if their modules are not implemented yet.

When adding a feature:

1. Make it opt-in unless there is a compelling compatibility reason otherwise.
2. Gate its module, dependencies, firmware, and docs consistently.
3. Verify both default-feature-free and `--all-features` builds.
4. Avoid forcing unrelated users to compile radio stacks, codecs, runtimes, or firmware blobs.

## Hardware and memory invariants

The memory map is an architectural contract:

| Owner | Region |
|---|---|
| Arduino bootloader | Flash `0x0800_0000`, 256 KiB reserved |
| M7 application | Flash `0x0804_0000`, 768 KiB |
| M7 RAM | AXI SRAM `0x2400_0000`, 512 KiB |
| M4 application | Flash `0x0810_0000`, 1 MiB |
| M4 RAM | D2 SRAM `0x3000_0000`, 256 KiB |
| Embassy `SharedData` | D3 SRAM `0x3800_0000`, first 1 KiB |
| Bridge or IPC mailbox | D3 SRAM `0x3800_0400`, next 1 KiB |

Guardrails:

- Never overlap the Arduino bootloader at `0x0800_0000..0x0803_FFFF` unless explicitly replacing it is the task.
- M7 and M4 flash and RAM regions must remain non-overlapping.
- Both core images must place matching shared objects at identical addresses with identical section names and compatible layouts.
- Shared sections must remain `NOLOAD`, retained with `KEEP`, and correctly aligned.
- `memory.x` is required for each firmware image; do not replace it with guessed defaults.
- Do not hand-write an application `device.x` or interrupt vector table when the correct Embassy/PAC core feature can provide it.
- The template selects `stm32h747xi-cm7` and `stm32h747xi-cm4` separately. Build those packages in separate Cargo invocations; do not use `cargo build --workspace` inside `template/`.
- A change to any `memory.x`, firmware `build.rs`, `.cargo/config.toml`, or probe target is platform-level work. Review all addresses and build both images.

## Dual-core execution model

- M7 owns clock-tree initialization and board-level peripheral setup.
- M7 must configure D3 SRAM as normal, non-cacheable, non-bufferable memory by calling `configure_m7_shared_sram` before shared Embassy data or mailbox access.
- M7 initializes shared Embassy state and the mailbox before releasing M4.
- M7 sets the M4 boot vector to `0x0810_0000` through `SYSCFG.UR3.BOOT_ADD1` (`BCM4_ADD0`) and releases CPU2 through `RCC.GCR.BOOT_C2`.
- M4 uses Embassy `init_secondary` when it is an Embassy application; M7 uses `init_primary`.
- Arduino option bytes normally hold M4 in reset until M7 releases it.
- Flash M4 first, then M7, and reset after both images are programmed.

### Bridge invariants

`BridgeMailbox` is a raw word-oriented, single-command/single-response mailbox.

- Place one instance in each image at the same `.bridge_mailbox` address.
- Keep `#[repr(C, align(32))]` and field order compatible across both images.
- M7 calls `initialize_primary()` before releasing M4.
- Payload/value stores happen before sequence publication.
- Publication uses `Release`; consumption uses `Acquire` before reading payload.
- Callers track previously observed sequence values.
- Shared mutable data must use atomics or another proven cross-core protocol. Do not create ordinary Rust references to bytes concurrently modified by the other core.

### Typed IPC invariants

`ipc::Channel<T, R, N>` is typed, allocation-free request/response IPC over `IpcMailbox`.

- Both images must use the same request and response types in the same order.
- Use a shared `no_std` protocol crate for message types in nontrivial applications.
- The model is single-client/single-worker with one outstanding request.
- Postcard request and response frames are each limited to `IPC_CAPACITY` (256 bytes).
- Sequence zero is reserved; request ID wrap must continue skipping zero.
- Preserve the atomic word storage and release/acquire publication ordering.
- `Polling` is the runtime-neutral default.
- A plain Cortex `SEV` is not a reliable inter-core interrupt. Use STM32 HSEM or another configured peripheral interrupt for a real inter-core doorbell.
- Keep notification and async waiting pluggable through `Notify` and `AsyncWait`; do not couple IPC to a specific executor.

## Interrupts and core-specific Embassy features

For Embassy applications:

- M7 selects `embassy-stm32/stm32h747xi-cm7`.
- M4 selects `embassy-stm32/stm32h747xi-cm4`.
- These device features are mutually exclusive in one Cargo dependency graph.
- Build core packages separately when both use Embassy.
- Let `stm32-metapac` provide the core-specific `device.x` and vector table.
- Use `bind_interrupts!` only when binding actual interrupt-driven peripherals.
- Do not add an empty `device.x`, a generic `__INTERRUPTS` array, or synthetic default vectors to work around feature unification. Fix the build scope or device feature selection instead.

## Wi-Fi ownership boundary

The GIGA Wi-Fi function uses the CYW4343W over four-bit SDIO.

- SDIO: PC12 clock, PD2 command, PC8-PC11 data.
- Control: PB10 power, PI8 host-wake.
- The BSP owns board power/reset sequencing and bundled firmware/NVRAM/CLM data.
- The application supplies the concrete SDIO transport and `cyw43::State` storage.
- Construct/discover the SDIO transport only after the board power sequence; discovery communicates with the radio.
- `take_runner()` is one-shot. The application must continuously run the returned consuming CYW43 runner.
- The BSP must not choose the executor, IP stack, or application network policy.

## BLE ownership boundary

The Bluetooth function is separate from Wi-Fi and uses UART7.

- UART: 115,200, 8-N-1, RTS/CTS.
- Pins: PF7 TX, PA8 RX, PF8 RTS, PF9 CTS, PA10 power, PG3 host-wake, PH7 device-wake.
- The BSP owns power/wake sequencing, bundled HCI patchram download, and final HCI reset.
- The application supplies the configured UART and pins and receives them back through owned parts.
- Do not couple the BSP to a particular BLE host. `trouble-host` belongs in examples/applications.
- BLE and Wi-Fi may run independently or together; do not conflate their ownership models.

## Pins and peripheral APIs

- Verify mappings against authoritative Arduino schematics, pinout, STM32H747 documentation, and ArduinoCore-mbed sources.
- Preserve Arduino labels D0-D75 and A0-A13 and active-low RGB semantics.
- `arduino_giga_pins!` deliberately expands Embassy mappings in the consuming application so the BSP does not depend on Embassy.
- New wrappers should own resources, initialize safe board states, forward standard traits, and expose explicit release/parts handoffs.
- Errors should be typed and useful in both normal `Debug` and optional `defmt::Format` contexts.

## Adding a new hardware feature

1. Confirm the board routing and electrical behavior from authoritative hardware sources.
2. Identify the closest existing module and example; follow its ownership and feature pattern.
3. Add only board-specific policy to `giga-r1`. Leave generic peripheral drivers and runtime setup to the application/HAL.
4. Add optional dependencies only when justified and feature-gated.
5. Add or update a focused, independently buildable example.
6. Include `defmt` as an optional local example feature when useful, following nearby examples.
7. Document wiring, active levels, startup order, and hardware prerequisites.
8. Update `README.md` and `CHANGELOG.md` for user-visible or public API changes.
9. Validate default features, the new feature, and all features.
10. Hardware-test the smallest relevant example when equipment is available; never claim hardware validation unless it was performed.

## Fixing bugs

- Reproduce or inspect the failing path before editing.
- Fix the ownership, ordering, mapping, or initialization root cause rather than adding delays or retries blindly.
- For shared-memory bugs, inspect linker placement, cache/MPU attributes, alignment, atomic ordering, startup order, and reset behavior before changing protocol code.
- For interrupt bugs, verify the selected core PAC, `bind_interrupts!` mapping, NVIC/peripheral enable state, and package build isolation. Do not paper over missing device metadata with custom vectors.
- For radio bugs, separate board power sequencing from transport/runner/host-stack behavior and identify which layer owns the failure.
- For linker failures, inspect the exact core feature and linker search path. Remove stale build outputs when a deleted linker file may remain in `OUT_DIR`.
- Add a regression example or compile-time check when feasible. This repository has no conventional host test suite, so focused embedded builds are important.
- Do not fix unrelated warnings or redesign neighboring APIs during a bug fix.

## Example conventions

Examples are workspace packages, not `examples/*.rs` targets inside the library crate.

- Directories generally use underscores; package and binary names use hyphens.
- Set `publish = false`, `test = false`, and `bench = false`.
- Firmware packages normally contain `Cargo.toml`, `build.rs`, `memory.x`, and `src/main.rs`.
- M7 Embassy examples select `stm32h747xi-cm7`.
- Keep M4 release builds isolated when device-feature unification is possible.
- Copy the nearest example's build/link/logging structure rather than inventing a new one.
- Loopback and board examples require real wiring/hardware and are not host tests.

## Template conventions

`template/` is an independent Cargo Generate workspace and is intentionally excluded from the root workspace.

- Keep one generated project containing separate `m7` and `m4` firmware packages.
- Keep separate `memory.x` files because the cores have different flash/RAM regions.
- Keep M7 and M4 source entry points separate; shared protocol/application logic may live in a shared `no_std` crate.
- Build with:

```sh
cargo build -p giga-m4 --release
cargo build -p giga-m7 --release
```

- Do not document or validate `cargo build --workspace` for the template while the two packages select incompatible CM4/CM7 Embassy device features.
- Keep the template's `.cargo/config.toml`, `Embed.toml`, toolchain, chip description, and README self-contained.
- After template changes, instantiate a fresh project with Cargo Generate and build both generated images. A build inside the source template alone is insufficient.

## Unsafe code policy

The workspace denies unsafe code.

- New unsafe code must be exceptional, narrowly scoped with `#[allow(unsafe_code)]`, and accompanied by a comment stating the exact invariant.
- Hardware register writes, fixed-address shared memory, and linker-section declarations do not justify broad module-level allowances.
- Never use unsafe merely to bypass ownership or type errors.
- Preserve the existing MPU setup barriers and exclusive `MPU` ownership rationale.

## Sensitive files

Do not modify these casually:

- `tools/set-giga-option-bytes.S`: replaces the complete option register with a hard-coded board value. It is recovery-only, not normal flashing.
- `tools/giga-r1-m7.yaml` and `template/tools/giga-r1-m7.yaml`: probe access and flash algorithm contract.
- `Embed.toml` and `template/Embed.toml`: flashing/reset/RTT behavior.
- Any `memory.x` or firmware `build.rs`: bootloader safety and core/shared-memory partition.
- `crates/giga-r1/src/firmware/` and `LICENSE`: firmware provenance and Cypress licensing.
- `pins.rs`, `board.rs`, `bridge.rs`, and `ipc.rs`: public board and cross-core ABI contracts.
- `Cargo.lock` and `template/Cargo.lock`: change only as a consequence of intentional dependency changes.
- CI/release workflows and crate versions: preserve publication and tag/version safeguards.

## Validation

Use the most focused checks first, then broaden. Install `flip-link` if needed:

```sh
cargo install flip-link --locked
```

Root CI-equivalent validation:

```sh
cargo metadata --no-deps --format-version 1
cargo fmt --check --all
cargo clippy --workspace --target thumbv7em-none-eabihf --all-targets -- -D warnings
cargo check --workspace --target thumbv7em-none-eabihf
cargo check --workspace --target thumbv7em-none-eabihf --all-features
cargo build --workspace --exclude dual-core-m4 --exclude dual-core-postcard-m4-worker --target thumbv7em-none-eabihf --release
cargo build -p dual-core-m4 -p dual-core-postcard-m4-worker --target thumbv7em-none-eabihf --release
RUSTDOCFLAGS="-D warnings" cargo doc -p giga-r1 --target thumbv7em-none-eabihf --all-features --no-deps
cargo publish -p giga-r1 --dry-run --allow-dirty --target thumbv7em-none-eabihf
```

Template validation:

```sh
cargo fmt --manifest-path template/Cargo.toml --all -- --check
cargo build --manifest-path template/Cargo.toml -p giga-m4 --release
cargo build --manifest-path template/Cargo.toml -p giga-m7 --release
cargo generate --path template --name giga-template-smoke --destination /tmp
cargo build --manifest-path /tmp/giga-template-smoke/Cargo.toml -p giga-m4 --release
cargo build --manifest-path /tmp/giga-template-smoke/Cargo.toml -p giga-m7 --release
```

Also run `git diff --check` after edits. If a command cannot be run, state that explicitly. Never claim flashing, RTT output, radio operation, or physical loopback passed unless tested on hardware.

## Cargo Embed and hardware validation

- Cargo Embed requires an external SWD probe; it does not use the normal Arduino USB bootloader connection.
- The custom target intentionally exposes only the M7 debug access port because M4 is normally held until M7 releases it.
- The M7 debug port can still program both flash banks.
- Normal dual-core flow:

```sh
cargo embed --path target/thumbv7em-none-eabihf/release/<m4-binary>
cargo embed -p <m7-package> --release
```

- Reflash M4 whenever its image changes.
- Press reset after both images are programmed.
- Record required wiring and board state for hardware tests.

## Documentation and release expectations

- Keep public Rust APIs documented.
- Keep `README.md`, example READMEs, and code behavior synchronized.
- Add user-visible changes to `CHANGELOG.md` in the appropriate release section.
- Use Conventional Commit-style titles, for example `feat(template): ...` or `fix(ipc): ...`.
- Do not bump versions, create tags, publish, commit, or change branches unless explicitly requested.
- Release tags must match `v<crate-version>`; the release workflow enforces this.

## Final checklist for agents

Before finishing a change, confirm:

- The base BSP remains `no_std`, HAL-neutral, and runtime-neutral.
- Resource ownership and release paths remain explicit.
- Features and dependencies are correctly gated.
- Bootloader, M7, M4, Embassy shared-data, and mailbox regions do not overlap.
- CM4 and CM7 Embassy features were not unified into one build.
- Shared-memory startup, alignment, cache attributes, and atomic ordering remain correct.
- No synthetic `device.x` or manual interrupt vectors were introduced as a workaround.
- New unsafe code is minimal and justified.
- Relevant examples/docs/changelog were updated.
- Focused builds and appropriate broader validation were actually run.
- Hardware validation claims match what was physically tested.
