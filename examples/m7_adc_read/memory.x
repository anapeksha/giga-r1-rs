MEMORY
{
  /* MCUboot payload address for Arduino GIGA M7 applications. */
  FLASH  (rx)  : ORIGIN = 0x08040000, LENGTH = 768K
  AXISRAM (rwx): ORIGIN = 0x24000000, LENGTH = 512K
  SHARED (rwx) : ORIGIN = 0x38000000, LENGTH = 1K
}

REGION_ALIAS(RAM, AXISRAM);

SECTIONS
{
  .shared_data (NOLOAD) : ALIGN(8)
  {
    KEEP(*(.shared_data));
  } > SHARED
}
