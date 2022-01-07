MEMORY
{
  /* NOTE 1 K = 1 KiBi = 1024 bytes */
  /* These values correspond to the NRF52840 with Softdevices S140 7.0.1 */
  FLASH : ORIGIN = 0x00000000 + 156k, LENGTH = 1024K - 156k
  RAM : ORIGIN = 0x20000000 + 128k, LENGTH = 256K - 128k
}
