---
title: 背光
description: 如何给你的键盘添加背光
---

:::caution
该功能仍在开发中。要查看尚需实现的功能列表，请查看 [待办事项列表](#待办事项列表)。
:::

背光可以用于为键帽下方提供照明，通常通过位于或位于各个开关内部的 LED 实现。`rumcake` 支持根据 LED 类型以及 LED 是否可单独寻址来支持不同类型的背光。

# 设置

## 必需的 Cargo 功能

您必须启用以下 `rumcake` 功能：

- 恰好其中一个：
  - `simple-backlight`（单色背光，所有 LED 具有相同的亮度）
  - `simple-backlight-matrix`（单色背光，矩阵中的每个 LED 都可以单独寻址）
  - `rgb-backlight-matrix`（RGB 背光，矩阵中的每个 LED 都可以单独寻址）
- 对于您想要使用的[可用背光驱动程序](#可用驱动程序)的特性标志
- `storage`（可选，如果您想保存背光设置）

某些驱动程序可能无法支持所有背光类型。

## 必需的代码

要设置背光，您必须创建一个新类型以实现 traits。然后，您可以在您的 `#[keyboard]` 宏调用中添加 `<backlight_type>(id = <type>, driver_setup_fn = <setup_fn>)`。您的新类型必须根据您使用的灯光类型实现适当的 traits：

- `simple_backlight`：[`SimpleBacklightDevice`](/rumcake/api/nrf52840/rumcake/lighting/simple_backlight/trait.SimpleBacklightDevice.html)
- `simple_backlight_matrix`：[`SimpleBacklightMatrixDevice`](/rumcake/api/nrf52840/rumcake/lighting/simple_backlight_matrix/trait.SimpleBacklightMatrixDevice.html)
- `rgb_backlight_matrix`：[`RGBBacklightMatrixDevice`](/rumcake/api/nrf52840/rumcake/lighting/rgb_backlight_matrix/trait.RGBBacklightMatrixDevice.html)

`driver_setup_fn` 必须是一个没有参数的异步函数，并返回一个实现适当驱动程序 trait 的类型：

- `simple_backlight`：[`SimpleBacklightDriver<T>`](/rumcake/api/nrf52840/rumcake/lighting/simple_backlight/trait.SimpleBacklightDriver.html)
- `simple_backlight_matrix`：[`SimpleBacklightMatrixDriver<T>`](/rumcake/api/nrf52840/rumcake/lighting/simple_backlight_matrix/trait.SimpleBacklightMatrixDriver.html)
- `rgb_backlight_matrix`：[`RGBBacklightMatrixDriver<T>`](/rumcake/api/nrf52840/rumcake/lighting/rgb_backlight_matrix/trait.RGBBacklightMatrixDriver.html)

```rust ins={5-8,13-22}
use rumcake::keyboard;

#[keyboard(
    // 在您的键盘宏调用中的某处 ...
    simple_backlight_matrix( // TODO: 如果您需要，请将此更改为 `rgb_backlight_matrix` 或 `simple_backlight`。
        id = MyKeyboardLighting,
        driver_setup_fn = my_backlight_setup,
    )
)]
struct MyKeyboard;

// 背光配置
use rumcake::lighting::simple_backlight_matrix::{SimpleBacklightMatrixDevice, SimpleBacklightMatrixDriver};
struct MyKeyboardLighting; // 用于实现灯光 traits 的新类型
async fn my_backlight_setup() -> impl SimpleBacklightMatrixDriver<MyKeyboardLighting> {
    // TODO: 我们很快会填写这个！
    todo!()
}
impl SimpleBacklightMatrixDevice for MyKeyboardLighting {
    // 可选，设置 FPS
    const FPS: usize = 20;
}
```

:::caution
默认情况下，在键盘打开时（例如更改亮度、色调、饱和度、效果等）所做的背光设置将**不会**被保存。

可选择的，您可以添加 `use_storage` 和一个 `storage` 驱动程序来保存背光配置数据。

```rust ins={8,10}
use rumcake::keyboard;

#[keyboard(
    // 在您的键盘宏调用中的某处 ...
    simple_backlight_matrix( // TODO: 如果您需要，请将此更改为 `rgb_backlight_matrix` 或 `simple_backlight`。
        id = MyKeyboardLighting,
        driver_setup_fn = my_backlight_setup,
        use_storage // 可选，如果您想保存背光配置
    ),
    storage(driver = "internal") // 如果启用了 `use_storage`，则需要指定存储驱动程序。有关更多信息，请参见 feature-storage.md。
)]
struct MyKeyboard;
```

您还需要为所选的存储驱动程序进行额外的设置。有关更多信息，请参见 [存储功能](../feature-storage/) 文档。
:::


如果您正在实现背光矩阵（`simple-backlight-matrix` 或 `rgb-backlight-matrix`），您的键盘还必须实现 `BacklightMatrixDevice` 特性：

```rust ins={14,25-42}
use rumcake::keyboard;

#[keyboard(
    // 在键盘宏调用中的某个位置...
    simple_backlight_matrix( // 如果需要，请将此更改为 `rgb_backlight_matrix` 或 `simple_backlight`。
        id = MyKeyboardLighting,
        driver_setup_fn = my_backlight_setup,
    )
)]
struct MyKeyboard;

// 背光配置
use rumcake::lighting::simple_backlight_matrix::{SimpleBacklightMatrixDevice, SimpleBacklightMatrixDriver};
use rumcake::lighting::{BacklightMatrixDevice, setup_backlight_matrix};
struct MyKeyboardLighting;
async fn my_backlight_setup() -> impl SimpleBacklightMatrixDriver<MyKeyboardLighting> {
    // TODO: 待填充！
    todo!()
}
impl SimpleBacklightMatrixDevice for MyKeyboardLighting {
    // 可选，设置 FPS
    const FPS: usize = 20;
}

impl BacklightMatrixDevice for MyKeyboardLighting {
    setup_backlight_matrix! {
        led_layout: {
            [ (0,0)   (17,0)  (34,0)  (51,0)   (68,0)   (85,0)   (102,0)  (119,0)  (136,0)  (153,0)  (170,0)  (187,0)  (204,0)  (221,0)  (238,0)  (255,0) ]
            [ (4,17)  (26,17) (43,17) (60,17)  (77,17)  (94,17)  (111,17) (128,17) (145,17) (162,17) (178,17) (196,17) (213,17) (234,17) (255,17) ]
            [ (6,34)  (30,34) (47,34) (64,34)  (81,34)  (98,34)  (115,34) (132,34) (149,34) (166,34) (183,34) (200,34) (227,34) (227,34) (255,34) ]
            [ (11,51) (0,0)   (38,51) (55,51)  (72,51)  (89,51)  (106,51) (123,51) (140,51) (157,51) (174,51) (191,51) (208,51) (231,51) (255,51) ]
            [ (28,68) (49,68) (79,68) (121,68) (155,68) (176,68) (196,68) (213,68) (230,68) ]
        },
        led_flags: { // 必须与上述布局的行数和列数相同
            [ NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE ]
            [ NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE      ]
            [ NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE      ]
            [ NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE      ]
            [                NONE NONE      NONE NONE      NONE NONE NONE NONE NONE           ]
        }
    }
}
```

:::note
您的背光矩阵不一定需要与您的开关矩阵具有相同的尺寸。

请注意，对于反应式效果，矩阵位置将直接映射到 LED 位置。例如，按下位于开关矩阵位置行 0、列 0 的键将对应于 LED 矩阵上行 0、列 0 的 LED。
:::

最后，您必须设置驱动程序。为此，您需要完成您的 `driver_setup_fn`，通过构造驱动程序。您可以[检查所选驱动程序的 API 参考](/rumcake/api/nrf52840/rumcake/drivers/index.html)以获取用于简化此过程的设置函数或宏。

根据驱动程序，您可能还需要实现与您选择的驱动程序相对应的适当 trait。检查 [可用背光驱动程序列表](#可用驱动程序) 以获取此信息。

例如，对于 `is31fl3731`，您必须实现 `IS31FL3731BacklightDriver`，并且您可以使用 `setup_is31fl3731!` 宏设置驱动程序：

```rust del={9-10} ins={3-5,11-23,25-34}
use rumcake::lighting::simple

_backlight_matrix::{SimpleBacklightMatrixDevice, SimpleBacklightMatrixDriver};
use rumcake::lighting::{BacklightMatrixDevice, setup_backlight_matrix};
use rumcake::hw::platform::setup_i2c;
use rumcake::drivers::is31fl3731::{
    get_led_from_matrix_coordinates, setup_is31fl3731, IS31FL3731BacklightDriver
};
struct MyKeyboardLighting;
async fn my_backlight_setup() -> impl SimpleBacklightMatrixDriver<MyKeyboardLighting> {
    // TODO: We will fill this out soon!
    todo!()
    setup_is31fl3731! {
        device: MyKeyboardLighting, // 必须是 IS31FL3731BacklightDriver 的一个实现
        address: 0b1110100, // see https://github.com/qmk/qmk_firmware/blob/d9fa80c0b0044bb951694aead215d72e4a51807c/docs/feature_rgb_matrix.md#is31fl3731-idis31fl3731
        i2c: setup_i2c! { // 注意：setup_i2c 的参数可能会根据平台而变化。此处假定为 STM32。
            event_interrupt: I2C1_EV,
            error_interrupt: I2C1_ER,
            i2c: I2C1,
            scl: PB6,
            sda: PB7,
            rx_dma: DMA1_CH7,
            tx_dma: DMA1_CH6,
        }
    }
}
impl IS31FL3731BacklightDriver for MyKeyboardLighting {
    //  这个数组必须与您的 `BacklightMatrixDevice` 实现中指定的行数和列数相同。
    get_led_from_matrix_coordinates! {
        [ C1_1 C1_2 C1_3 C1_4 C1_5  C1_6  C1_7  C1_8  C1_9  C1_10 C1_11 C1_12 C1_13 C1_14 C1_15 C2_15 ]
        [ C2_1 C2_2 C2_3 C2_4 C2_5  C2_6  C2_7  C2_8  C2_9  C2_10 C2_11 C2_12 C2_13 C2_14 C3_15 ]
        [ C3_1 C3_2 C3_3 C3_4 C3_5  C3_6  C3_7  C3_8  C3_9  C3_10 C3_11 C3_12 C3_13 C3_14 C4_15 ]
        [ C4_1 C4_2 C4_3 C4_4 C4_5  C4_6  C4_7  C4_8  C4_9  C4_10 C4_11 C4_12 C4_13 C4_14 C5_15 ]
        [ C5_2 C5_3 C5_6 C5_7 C5_10 C5_11 C5_12 C5_13 C5_14 ]
    }
}

impl SimpleBacklightMatrixDevice for MyKeyboardLighting { /* ... */ }
impl BacklightMatrixDevice for MyKeyboardLighting { /* ... */ }

```

:::note
以上的 IS31FL3731 驱动程序设置假定使用了 `simple-backlight-matrix`。如果您想要 RGB 矩阵，则有一个单独的 `rumcake::drivers::is31fl3731::backlight::get_led_from_rgb_matrix_coordinates` 宏。
:::

# 键值

根据您选择的背光类型，您可以在您的 `keyberon` 布局中使用特定版本的 `BacklightCommand` 枚举：

- [Simple Backlight Commands](/rumcake/api/nrf52840/rumcake/backlight/simple_backlight/animations/enum.BacklightCommand.html)
- [Simple Backlight Matrix Commands](/rumcake/api/nrf52840/rumcake/backlight/simple_backlight_matrix/animations/enum.BacklightCommand.html)
- [RGB Backlight Matrix Commands](/rumcake/api/nrf52840/rumcake/backlight/rgb_backlight_matrix/animations/enum.BacklightCommand.html)

```rust
Toggle,
TurnOn,
TurnOff,
NextEffect,
PrevEffect,
SetEffect(BacklightEffect), // 可用效果的列表取决于所选择的背光模式。
SetHue(u8), // 仅 RGB Matrix
IncreaseHue(u8), // 仅 RGB Matrix
DecreaseHue(u8), // 仅 RGB Matrix
SetSaturation(u8), // 仅 RGB Matrix
IncreaseSaturation(u8), // 仅 RGB Matrix
DecreaseSaturation(u8), // 仅 RGB Matrix
SetValue(u8),
IncreaseValue(u8),
DecreaseValue(u8),
SetSpeed(u8),
IncreaseSpeed(u8),
DecreaseSpeed(u8),
SaveConfig, // 通常在背光配置更改时内部调用，仅当 `storage` 已启用时可用。
ResetTime, // 通常在内部用于同步分体键盘的 LED。
```

在您的 `keyberon` 布局中，您可以使用 `{Custom(SimpleBacklight(<command>))}`、`{Custom(SimpleBacklightMatrix(<command>))}`、`{Custom(RGBBacklightMatrix(<command>))}`，具体取决于您使用的背光系统。

此外，您必须选择与键码对应的背光系统，方法是实现一个关联类型 `SimpleBacklightDeviceType`、`SimpleBacklightMatrixDeviceType`、`RGBBacklightDeviceType` 中的一个。

用法示例：

```rust ins={14}
use keyberon::action::Action::*;
use rumcake::lighting::simple_backlight_matrix::SimpleBacklightMatrixCommand::*;
use rumcake::keyboard::{build_layout, Keyboard, Keycode::*};

impl KeyboardLayout for MyKeyboard {
    /* ... */

    build_layout! {
        {
            [ Escape {Custom(SimpleBacklightMatrix(Toggle))} A B C]
        }
    }

    type SimpleBacklightMatrixDeviceType = MyKeyboardLighting;
}
```

# 待办事项列表

- [ ] RGB 背光动画

# 可用驱动程序

| 名称           | 特性标志         | 必需特性                                                                                                                                 |
| -------------- | ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| IS31FL3731     | `is31fl3731`     | [`IS31FL3731BacklightDriver`](/rumcake/api/nrf52840/rumcake/drivers/is31fl3731/backlight/trait.IS31FL3731BacklightDriver.html)           |
| WS2812 Bitbang | `ws2812_bitbang` | [`WS2812BitbangBacklightDriver`](/rumcake/api/nrf52840/rumcake/drivers/ws2812_bitbang/backlight/trait.WS2812BitbangBacklightDriver.html) |