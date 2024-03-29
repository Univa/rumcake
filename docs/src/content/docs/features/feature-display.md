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

To set up your display, you must add `display(driver = "<driver>")` to your `#[keyboard]` macro invocation,
and your keyboard must implement the `DisplayDevice` trait.

```rust ins={5-7,11-17}
use rumcake::keyboard;

#[keyboard(
    // somewhere in your keyboard macro invocation ...
    display(
        driver = "ssd1306" // TODO: change this to your desired display driver, and implement the appropriate trait (info below)
    )
)]
struct MyKeyboard;

// Display configuration
use rumcake::display::DisplayDevice;
impl DisplayDevice for MyKeyboard {
    // Optional: set timeout and FPS
    const FPS: usize = 0 // Only update the display when information changes. Change this if you are displaying animations.
    const TIMEOUT: usize = 20
}
```

Lastly, you must also implement the appropriate trait that corresponds to your chosen driver in the `#[keyboard]` macro.
Check the [list of available display drivers](#available-drivers) for this information.

For example, with `ssd1306`, you must implement `Ssd1306I2cDriverSettings` and `Ssd1306I2cDisplayDriver`:

```rust ins={3-23}
// later in your file...

use rumcake::hw::mcu::setup_i2c_blocking;
use rumcake::drivers::ssd1306::driver::size::DisplaySize128x32;
use rumcake::drivers::ssd1306::display::Ssd1306I2cDisplayDriver;
// Note: The Ssd1306I2cDriverSettings trait does NOT come from the `rumcake` library. It is generated by the `keyboard` macro.
impl Ssd1306I2cDriverSettings for MyKeyboard {
    // Set size of the display
    type SIZE_TYPE = DisplaySize128x32;
    const SIZE: Self::SIZE_TYPE = DisplaySize128x32;

    // Optional: set rotation
    const ROTATION: DisplayRotation = DisplayRotation::Rotate90;

    // Set up the I2C peripheral to communicate with the SSD1306 screen
    setup_i2c_blocking! {
        SPIM0_SPIS0_TWIM0_TWIS0_SPI0_TWI0,
        TWISPI0,
        P0_17,
        P0_20
    }
}
impl Ssd1306I2cDisplayDriver for MyKeyboard {}
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
use rumcake::drivers::ssd1306::display::Ssd1306I2cDisplayDriver;

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
```

# Available Drivers

| Name        | Feature Flag | `keyboard` Macro Driver String | Required Traits                                                                                                                                       |
| ----------- | ------------ | ------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| SSD1306[^1] | `ssd1306`    | `"ssd1306"`                    | `Ssd1306I2cDriverSettings`[^2], [`Ssd1306I2cDisplayDriver`](/rumcake/api/nrf52840/rumcake/drivers/ssd1306/display/trait.Ssd1306I2cDisplayDriver.html) |

[^1]: I2C only
[^2]: This trait is generated by the `keyboard` macro, and not included in the `rumcake` API.
