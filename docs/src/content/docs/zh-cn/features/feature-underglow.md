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
- Feature flag for one of the [available underglow drivers](#available-drivers) that you would like to use
- `storage` (optional, if you want to save your backlight settings)

## Required code

To set up underglow, you must add a new type to implement traits on.
Then, you can add `underglow(id = <type>, driver_setup_fn = <setup_fn>)` to your `#[keyboard]` macro
invocation. Your new type must implement the `UnderglowDevice` trait.

The `driver_setup_fn` must be an async function that has no parameters, and returns a type that implements the
[`UnderglowDriver<T>`](/rumcake/api/nrf52840/rumcake/lighting/underglow/trait.UnderglowDriver.html) trait.

```rust ins={5-7,13-22}
use rumcake::keyboard;

#[keyboard(
    // somewhere in your keyboard macro invocation ...
    underglow(
        id = MyKeyboardUnderglow,
        driver_setup_fn = my_underglow_setup,
    )
)]
struct MyKeyboard;

// Underglow configuration
use rumcake::lighting::underglow::{UnderglowDevice, UnderglowDriver};
struct MyKeyboardUnderglow; // New type to implement underglow traits on
async fn my_underglow_setup() -> impl UnderglowDriver<MyKeyboardUnderglow> {
    // TODO: We will fill this out soon!
    todo!()
}
impl UnderglowDevice for MyKeyboardUnderglow {
    // Mandatory: set number of LEDs
    const NUM_LEDS: usize = 20
}
```

:::caution
By default, changes you make to underglow settings while the keyboard is on (e.g. changing brightness,
hue, saturation, effect, etc.) will **NOT** be saved by default.

Optionally, you can add `use_storage`, and a `storage` driver to save underglow config data.

```rust ins={8,10}
use rumcake::keyboard;

#[keyboard(
    // somewhere in your keyboard macro invocation ...
    underglow(
        id = MyKeyboardUnderglow,
        driver_setup_fn = my_underglow_setup,
        use_storage // Optional, if you want to save underglow configuration
    )
    storage(driver = "internal") // You need to specify a storage driver if you specified `use_storage`. See feature-storage.md for more information.
)]
struct MyKeyboard;
```

You will need to do additional setup for your selected storage driver as well.
For more information, see the docs for the [storage feature](../feature-storage/).
:::

Lastly, you must set up the driver. To do this, you need to complete your `driver_setup_fn` by constructing the driver.
You can [check the API reference for your chosen driver](/rumcake/api/nrf52840/rumcake/drivers/index.html) for a set up
function or macro to make this process easier.

Depending on the driver, you may also need to implement the appropriate trait that corresponds to your chosen driver in the `#[keyboard]` macro.
Check the [list of available underglow drivers](#available-drivers) for this information.

For example, with `ws2812_bitbang`, you can use the `setup_ws2812_bitbang!` macro to set up the driver:

```rust del={7-8} ins={4,9}
// later in your file...

use rumcake::lighting::underglow::{UnderglowDevice, UnderglowDriver};
use rumcake::drivers::ws2812_bitbang::setup_ws2812_bitbang;
struct MyKeyboardUnderglow; // New type to implement underglow traits on
async fn my_underglow_setup() -> impl UnderglowDriver<MyKeyboardUnderglow> {
    // TODO: We will fill this out soon!
    todo!()
    setup_ws2812_bitbang! { pin: PA10 }
}
impl UnderglowDevice for MyKeyboardUnderglow { /* ... */ }
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
SaveConfig, // normally called internally when the underglow config changes, only available if `storage` is enabled
ResetTime, // normally used internally for syncing LEDs for split keyboards
```

In your `KeyboardLayout` implementation, you must choose the underglow system that the keycodes will
correspond to by implementing `UnderglowDeviceType`.

Example of usage:

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

# Available Drivers

| Name           | Feature Flag     | Required Traits |
| -------------- | ---------------- | --------------- |
| WS2812 Bitbang | `ws2812-bitbang` | N/A             |
