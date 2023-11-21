# Underglow

<!--toc:start-->

- [Setup](#setup)
  - [Required Cargo features](#required-cargo-features)
  - [Required code](#required-code)
- [To-do List](#to-do-list)
<!--toc:end-->

## Setup

### Required Cargo features

You must enable the following `rumcake` features:

- `underglow`
- `drivers` (optional built-in drivers to power underglow)
- `storage` (optional, if you want to save your backlight settings)

### Required code

To set up underglow, you must add `underglow = "<driver>"` to your `#[keyboard]` macro invocation, your keyboard must implement the `UnderglowDevice` trait:

```rust
use rumcake::keyboard;

#[keyboard(underglow = "ws2812_bitbang")] // TODO: change this to your desired underglow driver, and implement the appropriate trait (info below)
struct MyKeyboard;

// Underglow configuration
use rumcake::underglow::UnderglowDevice;
impl UnderglowDevice for MyKeyboard {
    // Mandatory: set number of LEDs
    const NUM_LEDS: usize = 20
}
```

Lastly, you must also implement the appropriate trait that corresponds to your chosen driver in the `#[keyboard]` macro. For example, with `ws2812_bitbang`, you must implement `WS2812BitbangUnderglowDriver`:

```rust
use rumcake::drivers::ws2812_bitbang::underglow::WS2812BitbangUnderglowDriver;
impl WS2812BitbangUnderglowDriver for MyKeyboard {
    ws2812_pin! { PA10 }
}
```

## Keycodes

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

# To-do List

Nothing for now.
