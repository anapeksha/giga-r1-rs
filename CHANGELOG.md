# Changelog

All notable changes to this project are documented here.

## 0.1.0 - 2026-07-23

- Add a runtime-neutral `no_std` board support crate for Arduino GIGA R1 WiFi.
- Add typed board, pin, RGB LED, USB, CAN, QSPI, Wi-Fi, and dual-core APIs.
- Bundle the CYW4343W firmware and country data behind the `wifi` feature.
- Add independently buildable hardware examples for the Cortex-M7 and
  Cortex-M4 cores.
- Add Cargo Embed configurations for flashing and RTT logging.
