# GIGA R1 WiFi firmware

`4343WA1.bin` is the CYW4343W firmware image distributed with the Arduino
GIGA board core. It is kept outside the Rust crates and examples so firmware
assets have one explicit, versioned root.

- `4343WA1.bin`: 421,098 bytes, SHA-256
  `e47f0ee335354c830ad3bc7836144167eccc80b544f2046939abd7f721202464`
- `4343WA1.clm_blob`: 7,222 bytes, SHA-256
  `33339b0101b7130e81a1e8be9056ee6ef1756190a7e7aa1838b233319fac436f`
- Firmware: `7.45.98.95 (r724303 CY)`, FWID `01-5afc8c1e`
- License: Cypress [Permissive Binary License 1.0](LICENSE-permissive-binary-license-1.0.txt)
  (`LicenseRef-PBL`); redistribution metadata is recorded in [`DEPENDENCIES`](DEPENDENCIES).

The scan example copies this image into AXI SRAM before passing it to the SDIO
transport. It does not depend on the mutable firmware copy in external QSPI.
