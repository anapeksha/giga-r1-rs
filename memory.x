MEMORY
{
  /* Allocate 1MB Flash for M7 */
  FLASH (rx) : ORIGIN = 0x08000000, LENGTH = 1024K
  /* AXI SRAM (512 KB) */
  RAM (rwx)  : ORIGIN = 0x24000000, LENGTH = 512K
}
