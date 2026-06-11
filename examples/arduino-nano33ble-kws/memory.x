/* Nordic nRF52840 (Arduino Nano 33 BLE Sense): 1 MB flash, 256 KB RAM.
   The stock Arduino bootloader occupies the first 64 KB and jumps to the
   application at 0x10000; flashing via bossac never touches it, so the
   board stays Arduino-compatible. */
MEMORY
{
  FLASH : ORIGIN = 0x00010000, LENGTH = 960K
  RAM   : ORIGIN = 0x20000000, LENGTH = 256K
}
