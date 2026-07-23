# Changelog

All notable changes to this project are documented here.

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
