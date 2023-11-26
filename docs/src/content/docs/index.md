---
title: rumcake
description: Get started using rumcake firmware on your keyboard.
template: doc
hero:
  tagline: A rust-based keyboard firmware.
  actions:
    - text: Get started
      link: getting-started/setup-template/
      icon: right-arrow
      variant: primary
    - text: Read the API docs
      link: api/
      icon: external
---

:::caution
rumcake is still a work in progress. Expect some bugs and breaking changes.
:::

`rumcake` is a rust library that lets you build featureful keyboard firmware with ease.

Under the hood, `rumcake` uses [`embassy-rs`](https://github.com/embassy-rs/embassy) as the embedded framework.
Providing `rumcake` as a library allows you to build your firmware in your own Cargo workspace, removing the need to push code to the central `rumcake` repo.

The `rumcake` library:

- Provides `embassy-executor` tasks for common keyboard activities, including matrix polling, host communication, LED rendering, etc.
- Provides macros that allow you to configure your keyboard firmware in an easy-to-understand way. [`keyberon`](https://github.com/TeXitoi/keyberon) is also used under the hood for keyboard layout configuration.
- Aims to be platform-agnostic, and uses different HALs (hardware abstraction libraries) under the hood, depending on the chip you decide to build for.

# Minimum Supported Rust Version

`rumcake` uses some Rust features that are only found on the `nightly` toolchain.
Please use the latest nightly toolchain when compiling your firmware.

# MCUs

Note that building and flashing instructions may change depending on the MCU.
See [the templates](https://github.com/Univa/rumcake-templates) for some build
and flashing instructions for some common setups.

## Tested

- STM32F072CBx
- STM32F303CBx
- nRF52840 (tested with nice!nano v2)

## Planned MCUs for the future

- RP-based chips (I don't have access to an RP-based keyboard at the moment)

# Features

## Working

The following features are _working_, but may not be stable or have missing components.

- USB host communication
- Bluetooth host communication (only for nRF-based keyboards)
- Backlighting
- Underglow
- Split keyboards over BLE
- Displays (e.g. SSD1306)
- Storage
- Via/Vial

## Planned

- Media keys
- Encoders

# Why the name "rumcake"

"**RU**st **M**e**C**h**A**nical **KE**yboard"

# Acknowledgements

This firmware would not be possible without the work done by other community projects.

A huge thanks goes to the following projects:

- [QMK](https://github.com/qmk/qmk_firmware)
  - A lot of backlighting and underglow animations have been adapted from QMK.
  - WS2812 Bitbang driver is also loosely based on their implementation.
- [ZMK](https://github.com/zmkfirmware/zmk/)
  - Their existing bluetooth, and split keyboard implementations have been helpful references for rumcake's implementation
- [TeXitoi's `keyberon` crate](https://github.com/TeXitoi/keyberon)
  - For powering the logic for keyboard matrix and layouts
- [simmsb's corne firmware](https://github.com/simmsb/keyboard)
  - Very helpful reference for developing a keyboard firmware using [embassy-rs](https://github.com/embassy-rs/embassy)
- [TeXitoi's keyseebee project](https://github.com/TeXitoi/keyseebee)
  - Another helpful reference for a rust-based keyboard firmware
- Any dependency used by rumcake. Building this would be a lot more difficult without them!
