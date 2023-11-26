---
title: Underglow
description: How to configure your keyboard with underglow lighting.
---

Underglow refers to the lighting provided by LEDs underneath the keyboard.
`rumcake` makes the assumption that your underglow LEDs are RGB, and individually
addressable.

# Setup

## Required Cargo features

You must enable the following `rumcake` features:

- `underglow`
- `drivers` (optional built-in drivers to power underglow)
- `storage` (optional, if you want to save your backlight settings)

## Required code

To set up underglow, you must add `underglow(driver = "<driver>")` to your `#[keyboard]` macro invocation,
and your keyboard must implement the `UnderglowDevice` trait. Optionally, you can add `use_storage` to the
macro invocation to use the specified storage driver to save underglow config data.

```rust ins={5-7,11-16}
use rumcake::keyboard;

#[keyboard(
    // somewhere in your keyboard macro invocation ...
    underglow(
        driver = "ws2812_bitbang", // TODO: change this to your desired underglow driver, and implement the appropriate trait (info below)
    )
)]
struct MyKeyboard;

// Underglow configuration
use rumcake::underglow::UnderglowDevice;
impl UnderglowDevice for MyKeyboard {
    // Mandatory: set number of LEDs
    const NUM_LEDS: usize = 20
}
```

:::caution
By default, changes you make to underglow settings while the keyboard is on (e.g. changing brightness,
hue, saturation, effect, etc.) will **NOT** be saved by default.

Optionally, you can add `use_storage`, and a `storage` driver to save underglow config data.

```rust ins={7,9}
use rumcake::keyboard;

#[keyboard(
    // somewhere in your keyboard macro invocation ...
    underglow(
        driver = "ws2812_bitbang", // TODO: change this to your desired underglow driver, and implement the appropriate trait (info below)
        use_storage // Optional, if you want to save underglow configuration
    )
    storage = "internal" // You need to specify a storage driver if you specified `use_storage`. See feature-storage.md for more information.
)]
struct MyKeyboard;

// Underglow configuration
use rumcake::underglow::UnderglowDevice;
impl UnderglowDevice for MyKeyboard {
    // Mandatory: set number of LEDs
    const NUM_LEDS: usize = 20
}
```

You will need to do additional setup for your selected storage driver as well.
For more information, see the docs for the [storage feature](../feature-storage).
:::

Lastly, you must also implement the appropriate trait that corresponds to your chosen driver in the `#[keyboard]` macro. For example, with `ws2812_bitbang`, you must implement `WS2812BitbangUnderglowDriver`:

```rust ins={3-6}
// later in your file...

use rumcake::drivers::ws2812_bitbang::underglow::WS2812BitbangUnderglowDriver;
impl WS2812BitbangUnderglowDriver for MyKeyboard {
    ws2812_pin! { PA10 }
}
```

# Keycodes

In your keyberon layout, you can use any of the enum members defined in `UnderglowCommand`:

```rust
Toggle,
TurnOn,
TurnOff,
NextEffect,
PrevEffect,
SetEffect(UnderglowEffect),
SetHue(u8),
AdjustHue(i16),
SetSaturation(u8),
AdjustSaturation(i16),
SetValue(u8),
AdjustValue(i16),
SetSpeed(u8),
AdjustSpeed(i16),
SetConfig(UnderglowConfig),
SaveConfig, // normally called internally when the underglow config changes, only available if `storage` is enabled
SetTime(u32), // normally used internally for syncing LEDs for split keyboards
```

Example of usage:

```rust
use keyberon::action::Action::*;
use rumcake::underglow::animations::UnderglowCommand::*;
use rumcake::keyboard::{Keyboard, Keycode::*};

/* ... */

    build_layout! {
        {
            [ Escape {Custom(Underglow(Toggle))} A B C]
        }
    }
```
