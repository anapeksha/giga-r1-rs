MEMORY
{
  FLASH   (rx) : ORIGIN = 0x08100000, LENGTH = 1M
  RAM (rwx) : ORIGIN = 0x30000000, LENGTH = 256K
  EMBASSY (rwx): ORIGIN = 0x38000000, LENGTH = 1K
  MAILBOX (rwx): ORIGIN = 0x38000400, LENGTH = 1K
}

SECTIONS
{
  .shared_data (NOLOAD) : ALIGN(8)
  {
    KEEP(*(.shared_data));
  } > EMBASSY

  .bridge_mailbox (NOLOAD) : ALIGN(32)
  {
    KEEP(*(.bridge_mailbox));
  } > MAILBOX
}
