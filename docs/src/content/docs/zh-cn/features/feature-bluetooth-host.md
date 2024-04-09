---
title: Bluetooth
description: 如何设置键盘通过蓝牙与设备通信。
---

:::caution
此功能仍在开发中。有关仍需要实现的功能列表，请查看[待办事项列表](#待办事项列表)。
:::

这份文档包含了关于如何通过蓝牙（低功耗）与主机设备通信的信息。

# 设置

## 必需的 Cargo 功能

您必须启用以下 `rumcake` 功能：

- `bluetooth`
- 如果您使用基于 nRF 的键盘，则需要启用 `nrf-ble`

:::danger
对于基于 nRF5x 的微控制器（MCU），使用 [`nrf-softdevice` crate](https://github.com/embassy-rs/nrf-softdevice) 来实现蓝牙支持。
由于 `nrf-softdevice` 具有自己的关键段实现，**您必须禁用任何其他关键段实现**。
例如，如果您使用了 rumcake 模板之一，则可能需要从 `cortex-m` 依赖项中删除 `critical-section-single-core`：

```toml del={1} ins={2}
cortex-m = { version = "0.7.6", features = ["critical-section-single-core"] }
cortex-m = { version = "0.7.6" }
```

:::

## 必需的代码

为了使您的键盘支持与蓝牙主机的通信，您必须在 `#[keyboard]` 宏调用中添加 `bluetooth`，并且您的键盘必须实现 `BluetoothKeyboard` 和 `BluetoothDevice` trait：

```rust ins={5,9-21}
use rumcake::keyboard;

#[keyboard(
    // 在您的键盘宏调用中的某处 ...
    bluetooth
)]
struct MyKeyboard;

use rumcake::hw::platform::BluetoothDevice;
impl BluetoothDevice for WingpairLeft {
    // 此地址可以是任何您想要的地址，只要它是有效的“Random Static”蓝牙地址。
    // 请参阅此链接中的“Random Static Address”：https://novelbits.io/bluetooth-address-privacy-ble/
    const BLUETOOTH_ADDRESS: [u8; 6] = [0x41, 0x5A, 0xE3, 0x1E, 0x83, 0xE7]; // TODO: 更改此处
}

// 蓝牙配置
use rumcake::bluetooth::BluetoothKeyboard;
impl BluetoothKeyboard for MyKeyboard {
    const BLE_VID: u16 = 0x0000; // 更改此处
    const BLE_PID: u16 = 0x0000; // 更改此处
}
```

:::tip
您可以在同一个键盘上同时使用蓝牙和 USB 主机通信。

如果您使用的是模板，则 USB 应该已经配置好，但如果您手动设置了 Cargo 工作空间，则请参阅 [USB 主机通信文档](../feature-usb-host/)。

此外，请查看下面的章节以获取更多信息。
:::

# 按键码

在您的 keyberon 布局中，您可以使用 `HardwareCommand` 中定义的任何枚举成员：

```rust
ToggleOutput
OutputUSB
OutputBluetooth
```

更多信息如下。

## USB 主机通信互操作性

默认情况下，您的键盘将使用蓝牙与您的设备通信。
您可以使用 `ToggleOutput`、`OutputUSB` 或 `OutputBluetooth` 按键码来在 USB 和蓝牙之间切换。
这不会断开您的键盘与 USB 或蓝牙主机的连接。它只是确定要发送键盘报告到的设备。

# 待办事项列表

- [ ] Multiple bluetooth profiles
- [ ] LE Secure Connections (I believe this requires `nrf-softdevice` changes)
- [ ] Automatic output selection