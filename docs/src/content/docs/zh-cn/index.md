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
rumcake 仍处于 WIP 中。 预计会出现一些错误和破坏性更改。
:::

`rumcake` 是一个 Rust 库，可让您轻松构建功能强大的键盘固件。
在底层，“rumcake”使用 [`embassy-rs`](https://github.com/embassy-rs/embassy) 作为嵌入式框架。
提供“rumcake”作为库允许您在自己的 Cargo 工作区中构建固件，从而无需将代码推送到中央“rumcake”存储库。

`rumcake` 库：

- 为常见键盘活动提供“使馆执行器”任务，包括矩阵轮询、主机通信、LED 渲染等。
- 提供宏，允许您以易于理解的方式配置键盘固件。 [`keyberon`](https://github.com/TeXitoi/keyberon) 也用于键盘布局配置。
- 目标是与平台无关，并在底层使用不同的 HAL（硬件抽象库），具体取决于您决定构建的芯片。

# 支持的最低 Rust 版本

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
- RP2040

# Features

## Working

The following features are _working_, but may not be stable or have missing components.

- USB host communication
- Bluetooth host communication (only for nRF-based keyboards)
- Backlighting
- Underglow
- Split keyboards
- Displays (e.g. SSD1306)
- Storage
- Via/Vial
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
- [jtroo's `keyberon` fork](https://github.com/jtroo/kanata/tree/main/keyberon)
  - For the implementation of extra layout actions, like one shot and tap dance keys
- [riskable and borisfaure](https://github.com/TeXitoi/keyberon/pull/122)
  - For the implementation of sequences/macros in `keyberon`
- [simmsb's corne firmware](https://github.com/simmsb/keyboard)
  - Very helpful reference for developing a keyboard firmware using [embassy-rs](https://github.com/embassy-rs/embassy)
- [TeXitoi's keyseebee project](https://github.com/TeXitoi/keyseebee)
  - Another helpful reference for a rust-based keyboard firmware
- Any dependency used by rumcake. Building this would be a lot more difficult without them!
