MEMORY
{
  FLASH   (rx)  : ORIGIN = 0x08100000, LENGTH = 1M
  RAM     (rwx) : ORIGIN = 0x30000000, LENGTH = 256K
  EMBASSY (rwx) : ORIGIN = 0x38000000, LENGTH = 1K
  MAILBOX (rwx) : ORIGIN = 0x38000400, LENGTH = 1K
  BULK    (rwx) : ORIGIN = 0x38000800, LENGTH = 16K
}

SECTIONS
{
  .bulk_queue (NOLOAD) : ALIGN(32)
  {
    KEEP(*(.bulk_queue));
  } > BULK
}
