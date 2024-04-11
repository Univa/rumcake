---
title: 媒体键 / 消费者控制
description: 如何为您的键盘布局启用媒体键。
---

`rumcake` 可以配置为发送由 [USB 消费者使用页面中的变体](https://docs.rs/usbd-human-interface-device/latest/usbd_human_interface_device/page/enum.Consumer.html) 组成的 HID 报告。
这包括媒体控制、应用程序启动器、应用程序控制等。

# 设置

## 需要的 Cargo 特性

您必须启用以下 `rumcake` 特性：

- `media-keycodes`

## 需要的代码

启用了 `media-keycodes` 特性后，您可以开始在您的 `KeyboardLayout` 实现中使用 `Keycode::Media` 变体：
`Keycode::Media` 变体必须包含一个 `usbd_human_interface_device::page::Consumer` 变体，该变体作为 `rumcake::keyboard::Consumer` 重新导出。

使用示例：

```rust ins={2} ins="{Custom(Media(VolumeIncrement))}"
use keyberon::action::Action::*;
use rumcake::keyboard::{build_layout, Consumer::*, Keycode::Media};

/* ... */

    build_layout! {
        {
            [ Escape {Custom(Media(VolumeIncrement))} A B C]
        }
    }
```