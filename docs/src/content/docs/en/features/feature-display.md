---
title: Displays
description: How to configure your keyboard with a display.
---

A display can be added to your keyboard to show any kind of graphics. This is
especially useful for displaying live information such as battery level, output mode,
etc.

# Setup

## Required Cargo features

You must enable the following `rumcake` features:

- `display`
- Feature flag for one of the [available display drivers](#available-drivers) that you would like to use

## Required code

To set up your display, you must add `display(driver_setup_fn = <setup_fn>)` to your `#[keyboard]` macro invocation,
and your keyboard must implement the `DisplayDevice` trait.

The `driver_setup_fn` must be an async function that has no parameters, and returns a type that implements the
[`DisplayDriver<T>`](/rumcake/api/nrf52840/rumcake/display/drivers/index.html) trait.

```rust ins={5-7,11-21}
use rumcake::keyboard;

#[keyboard(
    // somewhere in your keyboard macro invocation ...
    display(
        driver_setup_fn = my_display_setup
    )
)]
struct MyKeyboard;

// Display configuration
use rumcake::display::{DisplayDevice, DisplayDriver};
async fn my_display_setup() -> impl DisplayDriver<MyKeyboard> {
    // TODO: We will fill this out soon!
    todo!()
}
impl DisplayDevice for MyKeyboard {
    // Optional: set timeout and FPS
    const FPS: usize = 0 // Only update the display when information changes. Change this if you are displaying animations.
    const TIMEOUT: usize = 20
}
```

Lastly, you must set up the driver. To do this, you need to complete your `driver_setup_fn` by constructing the driver.
You can [check the API reference for your chosen driver](/rumcake/api/nrf52840/rumcake/drivers/index.html) for a set up
function or macro to make this process easier.

Depending on the driver, you may also need to implement the appropriate trait that corresponds to your chosen driver.
Check the [list of available display drivers](#available-drivers) for this information.

For example, with `ssd1306`, you must implement `Ssd1306I2cDisplayDriver`, and you can use the `setup_ssd1306!` macro to set up the driver:

```rust del={4-5} ins={2,6-19,21}
use rumcake::display::{DisplayDevice, DisplayDriver};
use rumcake::drivers::ssd1306::{setup_ssd1306, Ssd1306I2cDisplayDriver};
async fn my_display_setup() -> impl DisplayDriver<MyKeyboard> {
    // TODO: We will fill this out soon!
    todo!()
    setup_ssd1306! {
        i2c: setup_i2c! { // Note: The arguments of setup_i2c may change depending on platform. This assumes STM32.
            event_interrupt: I2C1_EV,
            error_interrupt: I2C1_ER,
            i2c: I2C1,
            scl: PB6,
            sda: PB7,
            rx_dma: DMA1_CH7,
            tx_dma: DMA1_CH6,
        },
        // See the API reference for the ssd1306 crate for `size` and `rotation` values: https://docs.rs/ssd1306/latest/ssd1306/
        size: DisplaySize96x16,
        rotation: Rotate90,
    }
}
impl Ssd1306I2cDisplayDriver for MyKeyboard {}
impl DisplayDevice for MyKeyboard { /* ... */ }
```

# Custom graphics

By default, the display will show information about the keyboard depending on
what features are being used. If you're using any bluetooth features (e.g. `bluetooth`),
then the battery level will be displayed. If you are communicating
with your host device over USB and Bluetooth (`usb` and `bluetooth` enabled),
then it will also show the operation mode.

You are also able to display custom content using the `embedded-graphics` crate.
In every driver trait, you can change the default implementation of `on_update`,
which is called either every frame if you set `DisplayDevice::FPS` to a value
greater than 0, or only when information changes if it was set to 0.

Here's an example that shows the text "test" on the display:

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
    /* ... in your trait implementation */

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

# Available Drivers

| Name        | Feature Flag | Required Traits                                                                                                       |
| ----------- | ------------ | --------------------------------------------------------------------------------------------------------------------- |
| SSD1306[^1] | `ssd1306`    | [`Ssd1306I2cDisplayDriver`](/rumcake/api/nrf52840/rumcake/drivers/ssd1306/display/trait.Ssd1306I2cDisplayDriver.html) |

[^1]: I2C only
