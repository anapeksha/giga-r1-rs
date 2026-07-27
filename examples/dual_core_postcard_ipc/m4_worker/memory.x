MEMORY
{
  FLASH   (rx) : ORIGIN = 0x08100000, LENGTH = 1M
  RAM (rwx) : ORIGIN = 0x30000000, LENGTH = 256K
  MAILBOX (rwx): ORIGIN = 0x38000400, LENGTH = 1K
}

SECTIONS
{
  .ipc_mailbox (NOLOAD) : ALIGN(32)
  {
    KEEP(*(.ipc_mailbox));
  } > MAILBOX
}
