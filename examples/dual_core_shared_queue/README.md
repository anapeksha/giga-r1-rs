# Dual-core shared fixed-block queues

This example sends generic fixed-size binary blocks from the Cortex-M7 to the
Cortex-M4 and returns transformed blocks in the opposite direction. It uses two
allocation-free `giga_r1::ipc::SharedQueue<384, 4>` SPSC queues. The first four
bytes of each 1,536-byte block contain a little-endian sequence number; the M4
preserves that number and XORs every remaining byte with `0xA5`.

The exact queue footprint is 6,208 bytes with the current public layout, so the
two queues occupy 12,416 bytes. Both linker scripts reserve an explicit 16 KiB
`BULK` region at `0x3800_0800`, after the existing 1 KiB Embassy shared-data
reservation at `0x3800_0000` and 1 KiB IPC mailbox reservation at
`0x3800_0400`. The shared protocol crate has a compile-time assertion that the
bidirectional mailbox fits that 16 KiB region.

The queues use atomic-word storage and release/acquire publication ordering.
Each direction has exactly one producer and one consumer, and blocks are copied
into and out of shared memory rather than exposed through concurrently mutable
references. The M7 configures all D3 SRAM as normal, non-cacheable,
non-bufferable memory, initializes both queues, and only then releases the M4.
Both cores use the exact `BulkMailbox` type from the `no_std` protocol crate.
Queue initialization requires the M4 to be held in reset or otherwise quiescent;
independent single-core debugger resets require an application-level restart
handshake and are not supported by this example. The example polls with short
delays when a queue is empty or full; it does not
configure an inter-core interrupt or use an Embassy executor/time driver.

Build the core packages separately so each image keeps its own linker map:

```sh
cargo build -p dual-core-shared-queue-m4-worker --release
cargo build -p dual-core-shared-queue-m7-master --release
```

Flash the M4 image first at `0x0810_0000`, then flash the M7 image at
`0x0804_0000`:

```sh
cargo embed --path \
  target/thumbv7em-none-eabihf/release/dual-core-shared-queue-m4-worker
cargo embed -p dual-core-shared-queue-m7-master --release
```

Press reset after both images are programmed. The M7 shows yellow while a
round trip is in progress, green for a validated response, and red for a queue
or validation failure; blue separates rounds. Enable each firmware package's
`defmt` feature for sequence and failure details.

This example has not been validated on physical hardware.
