---
title: USB
description: 如何配置您的键盘以通过 USB 与设备通信。
---

这份文档包含了关于如何使您的键盘通过 USB 与主机设备通信的信息。

# 设置

## 必需的 Cargo 功能

您必须启用以下 `rumcake` 功能：

- `usb`

## 必需的代码

为了设置您的键盘进行 USB 主机通信，您的键盘必须实现 `USBKeyboard` trait，并在您的 `keyboard` 宏调用中添加 `usb`：

```rust ins={5,9-14}
use rumcake::keyboard;

#[keyboard(
    // 在您的键盘宏调用中的某处 ...
    usb
)]
struct MyKeyboard;

// USB 配置
use rumcake::usb::USBKeyboard;
impl USBKeyboard for MyKeyboard {
    const USB_VID: u16 = 0x0000;
    const USB_PID: u16 = 0x0000;
}
```

:::note
如果您使用的是模板，这些配置大部分应该已经为您完成。
如果是这样，请确保更改 `USB_VID` 和 `USB_PID`。
:::