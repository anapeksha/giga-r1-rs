# Changelog

All notable changes to this project are documented here.

## 0.4.0 - 2026-08-19

- Add `ipc::SharedQueue<WORDS, N>` with claimed `Producer` and `Consumer`
  endpoints for bounded, allocation-free SPSC transfer of generic binary
  blocks between the M7 and M4.
- Preserve runtime-neutral polling and pluggable `Notify`/`AsyncWait` waiting,
  with release/acquire publication and atomic-word payload storage that does not
  expose concurrently mutable shared references.
- Add host tests for full/empty handling, FIFO ordering, slot wraparound,
  endpoint ownership, short buffers, notification, and concurrent traffic.
- Add a dual-core example that round-trips generic 1,536-byte blocks through two
  four-slot queues in an explicitly reserved 16 KiB D3 SRAM region.
- Document queue memory cost, linker placement, non-cacheable D3 requirements,
  and the distinction between typed postcard RPC and bulk block transport.

## 0.3.0 - 2026-08-13

- Add an optional Embassy-backed `qspi::OnboardQspiFlash` wrapper for the
  Arduino GIGA R1 onboard 16 MiB QSPI NOR flash.
- Implement `embedded-storage-async` `ReadNorFlash` and `NorFlash` traits for
  the onboard flash wrapper, including range/alignment validation, page-split
  writes, sector erase, JEDEC ID reads, and configurable ready polling.
- Keep QSPI storage dependencies behind the `qspi` feature and preserve the
  existing `qspi::FLASH` routing metadata.
- Update the QSPI JEDEC example to use the reusable board flash wrapper.

## 0.2.0 - 2026-07-24

- Add owned Arduino D0–D75, A0–A13, and RGB pin wrappers with
  `embedded-hal` trait forwarding and an Embassy mapping adapter that does not
  add an Embassy dependency to the BSP.
- Add runtime-neutral CYW4343W Bluetooth power, wake, UART, and bundled HCI
  patchram initialization behind the `ble` feature.
- Add a functional Eddystone-compatible BLE beacon example using `bt-hci` and
  `trouble-host`.
- Add allocation-free typed M7/M4 IPC with postcard framing, atomic D3 SRAM
  storage, pluggable notification/wait policies, and worker supervision behind
  the `ipc` feature.
- Add a split M7 master/M4 worker example that offloads a fixed-point FFT power
  spectrum and validates the typed response.

## 0.1.0 - 2026-07-23

- Add a runtime-neutral `no_std` board support crate for Arduino GIGA R1 WiFi.
- Add typed board, pin, RGB LED, USB, CAN, QSPI, Wi-Fi, and dual-core APIs.
- Bundle the CYW4343W firmware and country data behind the `wifi` feature.
- Add independently buildable hardware examples for the Cortex-M7 and
  Cortex-M4 cores.
- Add Cargo Embed configurations for flashing and RTT logging.
