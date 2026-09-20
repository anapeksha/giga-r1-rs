MEMORY
{
  /* Match Arduino's factory boot chain: bootloader below 0x08040000. */
  FLASH  (rx)  : ORIGIN = 0x08040000, LENGTH = 768K
  RAM (rwx): ORIGIN = 0x24000000, LENGTH = 512K
  SHARED (rwx) : ORIGIN = 0x38000000, LENGTH = 1K
}

SECTIONS
{
  .shared_data (NOLOAD) : ALIGN(8)
  {
    KEEP(*(.shared_data));
  } > SHARED
}
