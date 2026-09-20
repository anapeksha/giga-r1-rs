# M7 QSPI blocking storage example

Demonstrates `giga_r1::qspi::BlockingOnboardQspiFlash` through the blocking
`embedded-storage` NOR-flash traits.

The example erases the final 4 KiB sector of the onboard 16 MiB QSPI NOR flash,
writes a 32-byte pattern, and reads it back. It is intentionally generic storage
validation, not a bootloader or OTA example.

> Warning: this example is destructive for the selected sector. Only run it when
> the final onboard QSPI sector does not contain data you need.

Build:

```sh
cargo build -p m7-qspi-blocking-storage --release
```

Flash with an attached SWD probe:

```sh
cargo embed -p m7-qspi-blocking-storage --release
```

Green LED means erase/write/read verification passed. Red LED means it failed.
