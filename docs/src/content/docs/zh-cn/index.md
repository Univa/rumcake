---
title: rumcake
description: 开始在键盘上使用 rumcake 固件。
template: doc
hero:
  tagline: 一个基于 rust 的键盘固件。
  actions:
    - text: 开始
      link: getting-started/setup-template/
      icon: right-arrow
      variant: primary
    - text: 阅读 API 文档
      link: api/
      icon: external
---

:::caution
rumcake 仍处于 WIP 中。 预计会出现一些错误和破坏性更改。
:::

`rumcake` 是一个 Rust 库，可让您轻松构建功能强大的键盘固件。
在底层， `rumcake` 使用 [`embassy-rs`](https://github.com/embassy-rs/embassy) 作为嵌入式框架。
提供 `rumcake` 作为库允许您在自己的 Cargo 工作区中构建固件，而无需将代码推送到中央 `rumcake` 存储库。

`rumcake` 库：

- 为常见键盘活动提供 `embassy-executor` 任务，包括矩阵轮询、主机通信、LED 渲染等。
- 提供宏，允许您以易于理解的方式配置键盘固件。 [`keyberon`](https://github.com/TeXitoi/keyberon) 也用于键盘布局配置。
- 目标是与平台无关，并在底层使用不同的 HAL（硬件抽象库），具体取决于您决定构建的芯片。

# 支持的最低 Rust 版本

`rumcake` 使用了一些只存在于 `nightly` 工具链的功能.
当你编译你的固件时，请使用最新的 nightly 工具链。

# MCUs

请注意，构建和刷写指令可能会根据 MCU 的不同而变化。
请参阅[模板](https://github.com/Univa/rumcake-templates)了解一些常见设置的构建以及刷写说明。

## 已测试的MCU

- STM32F072CBx
- STM32F303CBx
- nRF52840 (使用 nice!nano v2 测试)
- RP2040

# 特性

## 可用的

以下功能 _可用_ ，但可能不稳定或缺少组件。

- USB通讯
- 蓝牙通讯 (只适用于 nRF-based 键盘)
- 背光灯(Backlighting)
- 轴灯(Underglow)
- 分体键盘
- 显示(e.g. SSD1306)
- 存储(Storage)
- Via/Vial
- 媒体键(Media keys)
- 编码器(Encoders)

# 为什么叫 "rumcake"

"**RU**st **M**e**C**h**A**nical **KE**yboard"

# 致谢

如果没有其他社区项目所做的工作，这个固件是不可能实现的。

非常感谢以下项目：

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
