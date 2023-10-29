# Storage

<!--toc:start-->

- [Setup](#setup)
  - [Required Cargo features](#required-cargo-features)
  - [Required code to MCU flash for storage](#required-code-to-mcu-flash-for-storage)
- [To-do List](#to-do-list)
<!--toc:end-->

## Setup

### Required Cargo features

You must enable the following `rumcake` features:

- `storage`

Any features that are capable of storing data to a storage peripheral will automatically start using storage-related features.
This allows you to configure your keyboard to your liking, without losing any of the changes you made between restarts.

Currently, the following features are capable of using storage:

- `backlight` (to store backlight hue, saturation, value, speed, effect, etc.)
- `underglow` (to store underglow hue, saturation, value, speed, effect, etc.)
- `via`/`vial` (to store dynamic keymaps, not yet implemented)

### Storage space considerations

The amount of space you want to allocate for storage highly depends on what features your keyboard uses.

Under the hood, TicKV is used as the file system to store data. For more information, see their
[spec document](https://github.com/tock/tock/blob/master/libraries/tickv/SPEC.md). You may want to
consider allocating multiple pages to improve the longevity of your flash (even if you may not necessarily
need all the space).

### Required code to use MCU flash for storage

Continue with the following instructions **if you want to use the existing flash space on your selected MCU for storage**.

To set up storage, you must add a `CONFIG` section to your `memory.x` file, and specify the
start and end address of the `CONFIG` section, using `__config_start`, and `__config_end`.
This will involve taking some space from the `FLASH` section, so make sure you still have
enough space to flash your compiled firmware binary file.

The following example shows what `memory.x` may look like for an `STM32F303CBx` chip:

```
MEMORY
{
    FLASH : ORIGIN = 0x08000000, LENGTH =  120K /* decreased from the chip's max of 128K */
    CONFIG: ORIGIN = ORIGIN(FLASH) + LENGTH(FLASH), LENGTH = 8K /* add this */
    RAM   : ORIGIN = 0x20000000, LENGTH =   32K
}

__config_start = ORIGIN(CONFIG) - ORIGIN(FLASH); /* add this */
__config_end = __config_start + LENGTH(CONFIG); /* add this */

```

**Requirements for the `CONFIG` section:**

- Size be a multiple of the flash peripheral's "erase size". Sometimes this is also called "page size" or "region size".
  - In the above example, STM32F303CBx has an erase size of 2KiB. So, the size of the `CONFIG` section is 4 pages.
- Start address (`__config_start`) must be aligned to a multiple of the erase size.
- The value of `__config_start` and `__config_end` must be **relative to the start address of the FLASH section**.
  - Note that in the above example, we subtract `ORIGIN(FLASH)` for this reason.

## To-do List

- [ ] QSPI driver
