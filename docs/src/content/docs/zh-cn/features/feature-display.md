---
title: 显示
description: 如何配置带有显示屏的键盘。
---

显示屏可以添加到您的键盘上以显示各种图形。这对于显示诸如电池电量、输出模式等实时信息特别有用。

# 设置

## 需要的 Cargo 特性

您必须启用以下 `rumcake` 特性：

- `display`
- 您希望使用的[可用显示驱动程序](#可用驱动程序)的特性标志

## 需要的代码

要设置显示屏，您必须在 `#[keyboard]` 宏调用中添加 `display(driver_setup_fn = <setup_fn>)`，并且您的键盘必须实现 `DisplayDevice` 特性。

`driver_setup_fn` 必须是一个没有参数并返回实现 [`DisplayDriver<T>`](/rumcake/api/nrf52840/rumcake/display/drivers/index.html) 特性的类型的异步函数。

```rust ins={5-7,11-21}
use rumcake::keyboard;

#[keyboard(
    // 在键盘宏调用中的某处 ...
    display(
        driver_setup_fn = my_display_setup
    )
)]
struct MyKeyboard;

// 显示配置
use rumcake::display::{DisplayDevice, DisplayDriver};
async fn my_display_setup() -> impl DisplayDriver<MyKeyboard> {
    // TODO: 我们很快会填充这部分内容！
    todo!()
}
impl DisplayDevice for MyKeyboard {
    // 可选：设置超时和帧率
    const FPS: usize = 0 // 只有当信息更改时才更新显示。如果正在显示动画，请更改此设置。
    const TIMEOUT: usize = 20
}
```

最后，您必须设置驱动程序。为此，您需要完成您的 `driver_setup_fn`，通过构建驱动程序来完成。您可以[查看所选驱动程序的 API 参考](/rumcake/api/nrf52840/rumcake/drivers/index.html)以获得设置函数或宏以使此过程更轻松的信息。

根据驱动程序，您可能还需要实现与您选择的驱动程序相对应的适当特性。查看[可用显示驱动程序的列表](#可用驱动程序)以获取此信息。

例如，对于 `ssd1306`，您必须实现 `Ssd1306I2cDisplayDriver`，并且您可以使用 `setup_ssd1306!` 宏来设置驱动程序：

```rust del={4-5} ins={2,6-19,21}
use rumcake::display::{DisplayDevice, DisplayDriver};
use rumcake::drivers::ssd1306::{setup_ssd1306, Ssd1306I2cDisplayDriver};
async fn my_display_setup() -> impl DisplayDriver<MyKeyboard> {
    // TODO: 我们很快会填充这部分内容！
    todo!()
    setup_ssd1306! {
        i2c: setup_i2c! { // 注意：setup_i2c 的参数可能会根据平台而变化。这假设 STM32。
            event_interrupt: I2C1_EV,
            error_interrupt: I2C1_ER,
            i2c: I2C1,
            scl: PB6,
            sda: PB7,
            rx_dma: DMA1_CH7,
            tx_dma: DMA1_CH6,
        },
        // 有关 `size` 和 `rotation` 值，请参阅 ssd1306 crate 的 API 参考：https://docs.rs/ssd1306/latest/ssd1306/
        size: DisplaySize96x16,
        rotation: Rotate90,
    }
}
impl Ssd1306I2cDisplayDriver for MyKeyboard {}
impl DisplayDevice for MyKeyboard { /* ... */ }
```

# 自定义图形

默认情况下，显示将显示与正在使用的功能相关的键盘信息。如果您正在使用任何蓝牙功能（例如 `bluetooth`），则会显示电池电量。如果您正在通过 USB 和蓝牙与主机设备通信（启用了 `usb` 和 `bluetooth`），则还会显示操作模式。

您还可以使用 `embedded-graphics` crate 显示自定义内容。在每个驱动程序特性中，您可以更改 `on_update` 的默认实现，该实现将根据您设置 `DisplayDevice::FPS` 的值在每帧调用，如果将其设置为 0，则仅在信息更改时调用。

以下是一个示例，显示屏上显示文本“test”：

```rust
use embedded_graphics::mono_font::ascii::FONT_5X8;
use embedded_graphics::mono_font::{MonoTextStyle, MonoTextStyleBuilder};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::Point;
use embedded_graphics::text::Text;
use embedded_graphics::Drawable;
use rumcake::drivers::ssd1306::driver::mode::BufferedGraphicsMode;
use rumcake::drivers::ssd1306::driver::prelude::I2CInterface;
use rumcake::drivers::ssd1306::driver::size::{DisplaySize, DisplaySize128x32};
use rumcake::drivers::ssd1306::driver::Ssd1306;
use rumcake::drivers::ssd1306::Ssd1306I2cDisplayDriver;

pub static DEFAULT_STYLE: MonoTextStyle<'_, BinaryColor> = MonoTextStyleBuilder::new()
    .font(&FONT_5X8)
    .text_color(BinaryColor::On)
    .build();

impl Ssd1306I2cDisplayDriver for MyKeyboard {
    /* ... 在您的特性实现中 ... */

    fn on_update<S: DisplaySize>(
        display: &mut Ssd1306<
            I2CInterface<impl Write<Error = impl Debug>>,
            S,
            BufferedGraphicsMode<S>,
        >,
    ) {
        Text::with_baseline(
            "test",
            Point::new(0, 16),
            DEFAULT_STYLE,
            embedded_graphics::text::Baseline::Top,
        )
        .draw(display)
        .unwrap();
    }
}
```

# 可用驱动程序

| 名称        | 特性标志  | 必需特性                                                                                                              |
| ----------- | --------- | --------------------------------------------------------------------------------------------------------------------- |
| SSD1306[^1] | `ssd1306` | [`Ssd1306I2cDisplayDriver`](/rumcake/api/nrf52840/rumcake/drivers/ssd1306/display/trait.Ssd1306I2cDisplayDriver.html) |

[^1]: 仅支持 I2C