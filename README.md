# rumcake

A rust-based mechanical keyboard firmware that aims to be featureful, and easy to configure.

**⚠ `rumcake` is still a work in progress. Expect some bugs and breaking changes.**

<!--toc:start-->

- [Getting started](#getting-started)
  - [Minimum Supported Rust Version](#minimum-supported-rust-version)
- [MCUs](#mcus)
  - [Tested](#tested)
  - [Planned MCUs for the future](#planned-mcus-for-the-future)
- [Features](#features)
  - [Working](#working)
  - [Not yet working, WIP](#not-yet-working-wip)
  - [Planned](#planned)
- [Why "rumcake"](#why-rumcake)
- [Acknowledgements](#acknowledgements)
<!--toc:end-->

## Getting started

The easiest way to get started with rumcake is with the basic `rumcake` template.

```bash
cargo generate --git https://github.com/Univa/rumcake-templates rumcake-basic-template
```

The template contains a rumcake project that shows you a basic Cargo workspace setup,
along with how to configure your keyboard matrix, and layout. The template also
contains documentation on how to build and flash your firmware.

To learn how to add extra features to your keyboard, see the [templates](https://github.com/Univa/rumcake-templates) or refer to the files in [./docs](./docs)

### Minimum Supported Rust Version

`rumcake` uses some Rust features that are only found on the `nightly` toolchain.
Please use the latest nightly toolchain when compiling your firmware.

## MCUs

Note that building and flashing instructions may change depending on the MCU.
See the templates for some build and flashing instructions for some common setups.

### Tested

- STM32F072CBx
- STM32F303CBx
- nRF52840 (tested with nice!nano v2)

### Planned MCUs for the future

- RP-based chips (I don't have access to an RP-based keyboard at the moment)

## Features

### Working

The following features are _working_, but may not be stable or has missing components.

- USB host communication
- Bluetooth host communication (only for nRF-based keyboards)
- Backlighting
- Underglow
- Split keyboards over BLE

### Not yet working, WIP

- Via/Vial
- EEPROM

### Planned

- Media keys
- Displays (e.g. SSD1306)
- Encoders

## Why "rumcake"

**RU**st **M**e**C**h**A**nical **KE**yboard

## Acknowledgements

This firmware would not be possible without the work done by other community projects.

A huge thanks goes to the following projects:

- [QMK](https://github.com/qmk/qmk_firmware)
  - A lot of backlighting and underglow animations have been adapted from QMK.
  - WS2812 Bitbang driver is also loosely based on their implementation.
- [ZMK](https://github.com/zmkfirmware/zmk/)
  - Their existing bluetooth, and split keyboard implementations have been helpful references for rumcake's implementation
- [simmsb's corne firmware](https://github.com/simmsb/keyboard)
  - Very helpful reference for developing a keyboard firmware using [embassy-rs](https://github.com/embassy-rs/embassy)
- [TeXitoi's keyseebee project](https://github.com/TeXitoi/keyseebee)
  - Another helpful reference for a rust-based keyboard firmware
- Any dependency used by rumcake. Building this would be a lot more difficult without them!
