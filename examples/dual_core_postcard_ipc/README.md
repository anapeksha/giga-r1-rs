# Typed dual-core postcard IPC

`m7_master` serializes `ComputeRequest::ComputeFft` into the BSP's D3 SRAM
mailbox. `m4_worker` polls the runtime-neutral channel, computes an
allocation-free, fixed-point eight-point power spectrum, and serializes the
typed response. Applications that configure HSEM interrupts can supply their
own `Notify` and `AsyncWait` implementations instead of polling.

Build and flash the M4 image first at `0x0810_0000`, then run the M7 image at
`0x0804_0000`:

```sh
cargo build -p dual-core-postcard-m4-worker --release \
  --target thumbv7em-none-eabihf
cargo embed --path \
  target/thumbv7em-none-eabihf/release/dual-core-postcard-m4-worker
cargo embed -p dual-core-postcard-m7-master --release
```

Press reset after both images are programmed. Green followed by blue means the
typed round trip passed. The diagnostic failure colors are red (M4 did not
start), yellow (worker ready, request not observed), cyan (request accepted),
magenta (response published but rejected), and white (worker protocol error).
