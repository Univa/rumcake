---
title: 分体键盘
description: 如何配置分体键盘系统。
---

:::caution
此功能仍在开发中。有关尚需实现的功能列表，请查看[待办事项列表](#待办事项列表)。
:::

分体键盘由一个或多个外围设备组成，这些设备向一个中央设备通信以传输矩阵事件，然后该中央设备创建 HID 键盘报告发送到主机设备。

通常，分体键盘需要编译多个二进制文件，每个设备/分体键盘的一部分一个二进制文件。例如，您将需要一个左半部分的二进制文件，以及一个右半部分的二进制文件。

继续阅读以了解如何使用 `rumcake` 实现“中央”和“外围”设备。

# 示例

以下文档将展示一个左右分体键盘的示例，无需配对设备。

中央设备的代码将放置在 `left.rs` 中，外围设备的代码将放置在 `right.rs` 中。

要查看如何实现分体键盘的完整示例，请查看[模板仓库](https://github.com/Univa/rumcake-templates)。

# 中央设备设置

分体键盘设置中的“中央”设备定义了键盘布局，与主机设备通信，并从其他外围设备接收矩阵事件。应该只有一个中央设备。
如果分体键盘还使用额外功能，如背光或底部发光，则中央设备还负责将其相关命令发送到外围设备。

通常，中央设备可以是一个配对设备（有助于节省电池寿命），或者是键盘的一半。

## 中央设备所需的 Cargo 特性

您必须使用以下 `rumcake` 特性编译一个二进制文件：

- `split-central`
- 您希望使用的[可用驱动程序](#可用驱动程序)的特性标志

## 中央设备所需的代码

要设置中央设备，您必须在 `#[keyboard]` 宏调用中添加 `split_central(driver_setup_fn = <setup_fn>)`，
并且您的键盘必须实现 `CentralDevice` 特性。您的 `CentralDevice` 实现应包括 `type Layout = Self;`。
这将告诉 rumcake 将矩阵事件（从其他外围设备接收）重定向到布局，以处理为按键码。

`driver_setup_fn` 必须是一个没有参数并返回实现 [`CentralDeviceDriver`](/rumcake/api/nrf52840/rumcake/split/drivers/trait.CentralDeviceDriver.html) 特性的类型的异步函数。


```rust ins={6-8,17-24}
// left.rs
use rumcake::keyboard;

#[keyboard(
    // 在您的键盘宏调用的某处
    split_central(
        driver_setup_fn = my_central_setup
    )
)]
struct MyKeyboardLeftHalf;

// KeyboardLayout 必须已经被实现
use rumcake::keyboard::KeyboardLayout;
impl KeyboardLayout for MyKeyboardLeftHalf { /* ... */ }

// 主机设置
use rumcake::split::central::{CentralDevice, CentralDeviceDriver};
async fn my_central_setup() -> impl CentralDeviceDriver {
    // TODO: 我们将很快补充...
    todo!()
}
impl CentralDevice for MyKeyboardLeftHalf {
    type Layout = Self;
}
```
:::caution
确保您的中央设备通过 [USB](../feature-usb-host/) 或 [蓝牙](../feature-bluetooth-host/) 与主机设备通信。请按照这些文档实现您需要的功能。

尽管可以编译一个没有它们的中央设备，但您的键盘将无法与您想要使用它的主机设备通信。
:::

最后，您必须设置驱动程序。为此，您需要完成您的 `driver_setup_fn`，通过构建驱动程序来完成。您可以[查看所选驱动程序的 API 参考](/rumcake/api/nrf52840/rumcake/drivers/index.html)以获得设置函数或宏以使此过程更轻松的信息。

根据驱动程序，您可能还需要在 `#[keyboard]` 宏中实现对应于所选驱动程序的适当特性。
查看[可用驱动程序](#可用驱动程序)以获取此信息。

例如，使用 `SerialSplitDriver` 结构，您可以像这样构造它：

```rust del={11-12} ins={13-23}
// KeyboardLayout should already be implemented
use rumcake::keyboard::KeyboardLayout;
impl KeyboardLayout for MyKeyboardLeftHalf { /* ... */ }

// 分割中央设置
use rumcake::split::central::{CentralDevice, CentralDeviceDriver};
use rumcake::drivers::SerialSplitDriver;
use rumcake::hw::platform::setup_buffered_uarte;
async fn my_central_setup() -> impl CentralDeviceDriver {
    // TODO: 我们很快会填充这部分内容！
    todo!()
    SerialSplitDriver {
        serial: setup_buffered_uarte! { // 注意：这假设 nRF5x，其他 MCU 有自己的宏和参数。
            interrupt: UARTE0_UART0,
            uarte: UARTE0,
            timer: TIMER1,
            ppi_ch0: PPI_CH0,
            ppi_ch1: PPI_CH1,
            ppi_group: PPI_GROUP0,
            rx_pin: P0_29,
            tx_pin: P0_31,
        },
    }
}
impl CentralDevice for MyKeyboardLeftHalf {
    type Layout = Self;
}
```

:::note
如果您想要使用 nRF BLE 作为分体键盘通信的驱动程序，请查看[nRF-BLE](#nrf-ble-驱动程序) 部分以获取更多指导。
:::

# 外围设备设置

分体键盘设置中的“外围”设备具有开关矩阵，并将矩阵事件发送到中央设备。一个分体键盘设置可以有多个外围设备。
如果分体键盘还使用了额外功能，则所有外围设备都应从中央设备接收相关命令。

## 外围设备的 Cargo 特性

您必须使用以下 `rumcake` 特性编译一个二进制文件：

- `split-peripheral`
- 您希望使用的[可用驱动程序](#可用驱动程序)的特性标志

## 外围设备的必需代码

要设置外围设备，您必须在 `#[keyboard]` 宏调用中添加 `split_peripheral(driver_setup_fn = <setup_fn>)`，
并且您的键盘必须实现 `PeripheralDevice` 特性。您的 `KeyboardMatrix` 实现（应该已经实现）应包括 `type PeripheralDeviceType = Self`。
这将告诉 rumcake 将矩阵事件重定向到外围设备驱动程序，以发送到中央设备。

`driver_setup_fn` 必须是一个没有参数并返回实现 [`PeripheralDeviceDriver`](/rumcake/api/nrf52840/rumcake/split/drivers/trait.PeripheralDeviceDriver.html) 特性的类型的异步函数。

```rust ins={6-8,12-24}
// right.rs
use rumcake::keyboard;

#[keyboard(
    // 在键盘宏调用中的某处 ...
    split_peripheral(
        driver_setup_fn = my_peripheral_setup
    )
)]
struct MyKeyboardRightHalf;

// KeyboardMatrix 应该已经实现
use rumcake::keyboard::KeyboardMatrix;
impl KeyboardMatrix for MyKeyboardRightHalf {
    type PeripheralDeviceType = Self;
}

// 分割外围设置
use rumcake::split::peripheral::{PeripheralDevice, PeripheralDeviceDriver};
async fn my_peripheral_setup() -> impl PeripheralDeviceDriver {
    // TODO: 我们很快会填充这部分内容！
    todo!()
}
impl PeripheralDevice for MyKeyboardRightHalf {}
```

:::note
对于外围设备，您不必实现 `KeyboardLayout`。只需要实现 `KeyboardMatrix`。
:::

最后，您必须设置驱动程序。为此，您需要完成您的 `driver_setup_fn`，通过构建驱动程序来完成。您可以[查看所选驱动程序的 API 参考](/rumcake/api/nrf52840/rumcake/drivers/index.html)以获得设置函数或宏以使此过程更轻松的信息。

根据驱动程序，您可能还需要在 `#[keyboard]` 宏中实现对应于所选驱动程序的适当特性。
查看[可用驱动程序](#可用驱动程序)以获取此信息。

例如，使用 `SerialSplitDriver` 结构，您可以像这样构造它：

```rust del={10-11} ins={12-23}
// KeyboardLayout should already be implemented
use rumcake::keyboard::KeyboardLayout;
impl KeyboardLayout for MyKeyboardLeftHalf { /* ... */ }

// 分割外围设置
use rumcake::drivers::SerialSplitDriver;
use rumcake::hw::platform::setup_buffered_uarte;
use rumcake::split::peripheral::{PeripheralDevice, PeripheralDeviceDriver};
async fn my_peripheral_setup() -> impl PeripheralDeviceDriver {
    // TODO: 我们很快会填充这部分内容！
    todo!()
    SerialSplitDriver {
        serial: setup_buffered_uarte! { // 注意：这假设 nRF5x，其他 MCU 有自己的宏和参数。
            interrupt: UARTE0_UART0,
            uarte: UARTE0,
            timer: TIMER1,
            ppi_ch0: PPI_CH0,
            ppi_ch1: PPI_CH1,
            ppi_group: PPI_GROUP0,
            rx_pin: P0_31,
            tx_pin: P0_29,
        },
    }
}
impl PeripheralDevice for MyKeyboardRightHalf {}
```

:::note
如果您想要使用 nRF BLE 作为分体键盘通信的驱动程序，请查看[nRF-BLE](#nrf-ble-驱动程序) 部分以获取更多指导。
:::

# 无矩阵的中央设备（Dongle）

没有矩阵的中央设备的一个示例是一个 dongle。如果您想要实现这样的设备，您可以在您的 `#[keyboard]` 宏调用中添加 `no_matrix`。

这样做将消除实现 `KeyboardMatrix` 的需要，因此您只需实现 `KeyboardLayout`。

```rust ins={6}
// dongle.rs
use rumcake::keyboard;

#[keyboard(
    // 在键盘宏调用中的某处 ...
    no_matrix,
    split_central(
        driver = "ble" // TODO: 更改为您所需的分割驱动程序，并实现适当的特性
    )
)]
struct MyKeyboardDongle;

// 其余的配置 ...
```

# nRF-BLE 驱动程序

如果您使用的是 nRF5x MCU，并希望使用 BLE 进行分体键盘通信，则需要进行额外的更改才能使其正常工作。

对于中央设备和外围设备，都必须实现 [`BluetoothDevice`](/rumcake/api/nrf52840/rumcake/hw/platform/trait.BluetoothDevice.html) 特性：

`BLUETOOTH_ADDRESS` 可以是任何您想要的值，只要它是有效的“随机静态”蓝牙地址。
参见这里的“Random Static Address”：https://novelbits.io/bluetooth-address-privacy-ble/

```rust ins={2-5}
// 中央文件
use rumcake::hw::platform::BluetoothDevice;
impl BluetoothDevice for MyKeyboardLeftHalf {
    const BLUETOOTH_ADDRESS: [u8; 6] = [0x41, 0x5A, 0xE3, 0x1E, 0x83, 0xE7]; // TODO: 更改为其他值
}
```

```rust ins={2-5}
// 外围文件
use rumcake::hw::platform::BluetoothDevice;
impl BluetoothDevice for MyKeyboardRightHalf {
    const BLUETOOTH_ADDRESS: [u8; 6] = [0x92, 0x32, 0x98, 0xC7, 0xF6, 0xF8]; // TODO: 更改为其他值
}
```

:::note
如果您使用的是 `ble` 驱动程序，并且您的键盘还使用蓝牙与主机设备进行通信（基本上如果您遵循了[蓝牙文档](../feature-bluetooth-host/)或选择了带有蓝牙的模板），
那么 `BluetoothDevice` 特性应该已经为您实现。
:::

您还需要更改 `#[keyboard]` 宏调用以添加 `driver_type = "nrf-ble"`。
这将更改 `driver_setup_fn` 的签名要求。

```rust ins={6}
// 中央文件
#[keyboard(
    // 在键盘宏调用中的某处 ...
    split_central(
        driver_setup_fn = my_central_setup,
        driver_type = "nrf-ble"
    )
)]
struct MyKeyboardLeftHalf;
```

```rust ins={6}
// 外围文件
#[keyboard(
    // 在键盘宏调用中的某处 ...
    split_peripheral(
        driver_setup_fn = my_peripheral_setup,
        driver_type = "nrf-ble"
    )
)]
struct MyKeyboardRightHalf;
```

现在，您的 `driver_setup_fn` 将需要更改其签名。

对于中央设备，它需要返回：

- `CentralDeviceDriver` 实现者
- 包含要连接的外围设备地址的切片

对于外围设备，它需要返回：

- `PeripheralDeviceDriver` 实现者
- 要连接的中央设备的地址

`setup_nrf_ble_split_central!` 和 `setup_nrf_ble_split_peripheral!` 驱动程序可以用来实现您的 `driver_setup_fn`。

```rust del={3} ins={4-9}
// 中央文件
use rumcake::drivers::nrf_ble::central::setup_nrf_ble_split_central;
async fn my_central_setup() -> (impl CentralDeviceDriver, &'static [[u8; 6]]) {
    setup_nrf_ble_split_central! {
        peripheral_addresses: [
            [0x92, 0x32, 0x98, 0xC7, 0xF6, 0xF8] // 我们在外围设备文件中指定的外围设备地址
        ]
    }
}
```

```rust del={3} ins={4-7}
// 外围文件
use rumcake::drivers::nrf_ble::peripheral::setup_nrf_ble_split_peripheral;
async fn my_peripheral_setup() -> impl PeripheralDeviceDriver {
async fn my_peripheral_setup() -> (impl PeripheralDeviceDriver, [u8; 6]) {
    setup_nrf_ble_split_peripheral! {
        central_address: [0x41, 0x5A, 0xE3, 0x1E, 0x83, 0xE7] // 我们在中央设备文件中指定的中央设备地址
    }
}
```

# 待办事项列表

- [ ] 在分割键盘设置中从中央设备同步背光和底部灯光命令的方法
- [ ] 单个设备可以同时充当外围设备和中央设备
- [ ] 串行（半双工）驱动程序
- [ ] I2C 驱动程序

# 可用驱动程序

| 名称             | 特性标志       | 必需特性                                                                             |
| ---------------- | -------------- | ------------------------------------------------------------------------------------ |
| Serial[^1]       | 无（默认可用） | 无                                                                                   |
| nRF Bluetooth LE | `nrf-ble`      | [`BluetoothDevice`](/rumcake/api/nrf52840/rumcake/hw/mcu/trait.BluetoothDevice.html) |

[^1]:
    兼容任何同时实现 `embedded_io_async::Read` 和 `embedded_io_async::Write` 的类型。
    这包括 `embassy_nrf::buffered_uarte::BufferedUarte`（nRF UARTE）和 `embassy_stm32::usart::BufferedUart`（STM32 UART）。

