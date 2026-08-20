MEMORY
{
  /* Arduino's bootloader occupies the first 256 KiB of bank 1. */
  FLASH (rx)   : ORIGIN = 0x08040000, LENGTH = 768K
  RAM (rwx)    : ORIGIN = 0x24000000, LENGTH = 512K
  EMBASSY (rwx): ORIGIN = 0x38000000, LENGTH = 1K
  MAILBOX (rwx): ORIGIN = 0x38000400, LENGTH = 1K
}

SECTIONS
{
  .shared_data (NOLOAD) : ALIGN(8)
  {
    KEEP(*(.shared_data));
  } > EMBASSY

  .ipc_mailbox (NOLOAD) : ALIGN(32)
  {
    KEEP(*(.ipc_mailbox));
  } > MAILBOX
}
