MEMORY
{
  FLASH   (rx)  : ORIGIN = 0x08040000, LENGTH = 768K
  RAM     (rwx) : ORIGIN = 0x24000000, LENGTH = 512K
  EMBASSY (rwx) : ORIGIN = 0x38000000, LENGTH = 1K
  MAILBOX (rwx) : ORIGIN = 0x38000400, LENGTH = 1K
  BULK    (rwx) : ORIGIN = 0x38000800, LENGTH = 16K
}

SECTIONS
{
  .shared_data (NOLOAD) : ALIGN(8)
  {
    KEEP(*(.shared_data));
  } > EMBASSY

  .bulk_queue (NOLOAD) : ALIGN(32)
  {
    KEEP(*(.bulk_queue));
  } > BULK
}
