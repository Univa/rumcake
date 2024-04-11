---
title: Underglow
description: 如何配置键盘以启用底部照明。
---

以下是配置键盘使用底部照明的步骤：

# 设置

## 必要的 Cargo 功能

您必须启用以下 `rumcake` 功能：

- `underglow`
- 选择您想要使用的[可用底部照明驱动程序](#可用驱动程序)之一的功能标志
- `storage`（可选，如果您想要保存背光设置）

## 必要的代码

要设置底部照明，您必须添加一个新的类型来实现特性。然后，您可以在您的 `#[keyboard]` 宏调用中添加 `underglow(id = <type>, driver_setup_fn = <setup_fn>)`。您的新类型必须实现 `UnderglowDevice` 特性。

`driver_setup_fn` 必须是一个没有参数的异步函数，并返回一个实现 [`UnderglowDriver<T>`](/rumcake/api/nrf52840/rumcake/lighting/underglow/trait.UnderglowDriver.html) 特性的类型。

```rust ins={5-7,13-22}
use rumcake::keyboard;

#[keyboard(
    // 在您的键盘宏调用中的某处...
    underglow(
        id = MyKeyboardUnderglow,
        driver_setup_fn = my_underglow_setup,
    )
)]
struct MyKeyboard;

// 底部照明配置
use rumcake::lighting::underglow::{UnderglowDevice, UnderglowDriver};
struct MyKeyboardUnderglow; // 实现底部照明特性的新类型
async fn my_underglow_setup() -> impl UnderglowDriver<MyKeyboardUnderglow> {
    // TODO: 我们很快会填写这部分！
    todo!()
}
impl UnderglowDevice for MyKeyboardUnderglow {
    // 必填项：设置 LED 的数量
    const NUM_LEDS: usize = 20
}
```

:::caution
默认情况下，当键盘处于开启状态时（例如更改亮度、色调、饱和度、效果等），对底部照明设置所做的更改**不会**被默认保存。

可选地，您可以添加 `use_storage`，以及一个 `storage` 驱动程序来保存底部照明配置数据。

```rust ins={8,10}
use rumcake::keyboard;

#[keyboard(
    // 在键盘宏调用的某处...
    underglow(
        id = MyKeyboardUnderglow,
        driver_setup_fn = my_underglow_setup,
        use_storage // 可选项，如果您想要保存底部照明配置
    )
    storage(driver = "internal") // 如果您指定了 `use_storage`，则需要指定存储驱动程序。有关更多信息，请参阅 feature-storage.md。
)]
struct MyKeyboard;
```

您还需要为所选的存储驱动程序进行额外的设置。有关更多信息，请参阅[存储功能](../feature-storage/)的文档。
:::

最后，您必须设置驱动程序。为此，您需要完成您的 `driver_setup_fn`，构建驱动程序。
您可以[检查您选择的驱动程序的 API 参考](/rumcake/api/nrf52840/rumcake/drivers/index.html)以查找设置函数或宏，以使此过程更加简单。

根据驱动程序的不同，您可能还需要在 `#[keyboard]` 宏中实现与您选择的驱动程序对应的适当特性。
查看[可用底部照明驱动程序列表](#可用驱动程序)以获取此信息。

例如，对于 `ws2812_bitbang`，您可以使用 `setup_ws2812_bitbang!` 宏来设置驱动程序：

```rust del={7-8} ins={4,9}
// 在您的文件中的后面...

use rumcake::lighting::underglow::{UnderglowDevice, UnderglowDriver};
use rumcake::drivers::ws2812_bitbang::setup_ws2812_bitbang;
struct MyKeyboardUnderglow; // 新类型以实现底部照明特性
async fn my_underglow_setup() -> impl UnderglowDriver<MyKeyboardUnderglow> {
    // TODO: 我们很快会填写这部分！
    todo!()
    setup_ws2812_bitbang! { pin: PA10 }
}
impl UnderglowDevice for MyKeyboardUnderglow { /* ... */ }
```

# 键值

在您的 Keyberon 布局中，您可以使用 `UnderglowCommand` 中定义的任何枚举成员：

```rust
Toggle,
TurnOn,
TurnOff,
NextEffect,
PrevEffect,
SetEffect(UnderglowEffect),
SetHue(u8),
IncreaseHue(u8),
DecreaseHue(u8),
SetSaturation(u8),
IncreaseSaturation(u8),
DecreaseSaturation(u8),
SetValue(u8),
IncreaseValue(u8),
DecreaseValue(u8),
SetSpeed(u8),
IncreaseSpeed(u8),
DecreaseSpeed(u8),
SaveConfig，// 通常在底部照明配置更改时在内部调用，仅在启用了 `storage` 时可用
ResetTime，// 通常在分割键盘中用于同步 LED
```

在您的 `KeyboardLayout` 实现中，您必须通过实现 `UnderglowDeviceType` 来选择与按键码对应的底部照明系统。

使用示例：

```rust ins={14}
use keyberon::action::Action::*;
use rumcake::lighting::underglow::UnderglowCommand::*;
use rumcake::keyboard::{build_layout, Keyboard, Keycode::*};

impl KeyboardLayout for MyKeyboard {
    /* ... */

    build_layout! {
        {
            [ Escape {Custom(Underglow(Toggle))} A B C]
        }
    }

    type UnderglowDeviceType = MyKeyboardUnderglow;
}
```

# 可用驱动程序

| 名称           | 特性标志         | 必需特性 |
| -------------- | ---------------- | -------- |
| WS2812 Bitbang | `ws2812-bitbang` | N/A      |
