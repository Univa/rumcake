# Backlight

<!--toc:start-->

- [Setup](#setup)
  - [Required Cargo features](#required-cargo-features)
  - [Required code](#required-code)
- [To-do List](#to-do-list)
<!--toc:end-->

## Setup

### Required Cargo features

You must enable the following `rumcake` features:

- `backlight`
- `drivers` (optional built-in drivers to power backlighting)
- `storage` (optional, if you want to save your backlight settings)

Some drivers may not be able to support all backlight types.

### Required code

To set up backlighting, you must add `backlight = "<driver>"` to your `#[keyboard]` macro invocation, your keyboard must implement the `BacklightDevice` trait:

```rust
use rumcake::keyboard;

#[keyboard(backlight = "is31fl3731")] // TODO: change this to your desired backlight driver, and implement the appropriate trait (info below)
struct MyKeyboard;

// Backlight configuration
use rumcake::backlight::BacklightDevice;
impl BacklightDevice for MyKeyboard {
    // optionally, set FPS
    const FPS: usize = 20;
}

```

If you're implementing a backlight matrix (either the `simple-backlight-matrix` or `rgb-backlight-matrix`), your keyboard must also implement the `BacklightMatrixDevice` trait:

```rust
use rumcake::backlight::BacklightMatrixDevice;
use rumcake::{led_flags, led_layout};
impl BacklightMatrixDevice for MyKeyboard {
    led_layout! {
        [ (0,0)   (17,0)  (34,0)  (51,0)   (68,0)   (85,0)   (102,0)  (119,0)  (136,0)  (153,0)  (170,0)  (187,0)  (204,0)  (221,0)  (238,0)  (255,0) ]
        [ (4,17)  (26,17) (43,17) (60,17)  (77,17)  (94,17)  (111,17) (128,17) (145,17) (162,17) (178,17) (196,17) (213,17) (234,17) (255,17) ]
        [ (6,34)  (30,34) (47,34) (64,34)  (81,34)  (98,34)  (115,34) (132,34) (149,34) (166,34) (183,34) (200,34) (227,34) (227,34) (255,34) ]
        [ (11,51) (0,0)   (38,51) (55,51)  (72,51)  (89,51)  (106,51) (123,51) (140,51) (157,51) (174,51) (191,51) (208,51) (231,51) (255,51) ]
        [ (28,68) (49,68) (79,68) (121,68) (155,68) (176,68) (196,68) (213,68) (230,68) ]
    }

    led_flags! {
        [ NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE ]
        [ NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE      ]
        [ NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE      ]
        [ NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE NONE      ]
        [                NONE NONE      NONE NONE      NONE NONE NONE NONE NONE           ]
    }
}
```

Lastly, you must also implement the appropriate trait that corresponds to your chosen driver in the `#[keyboard]` macro. For example, with `is31fl3731`, you must implement `IS31FL3731BacklightDriver`:

```rust
use rumcake::{setup_i2c, get_led_from_matrix_coordinates};
use rumcake::drivers::is31fl3731::backlight::IS31FL3731BacklightDriver;
impl IS31FL3731BacklightDriver for MyKeyboard {
    const LED_DRIVER_ADDR: u32 = 0b1110100; // see https://github.com/qmk/qmk_firmware/blob/d9fa80c0b0044bb951694aead215d72e4a51807c/docs/feature_rgb_matrix.md#is31fl3731-idis31fl3731
    setup_i2c! {
        I2C1_EV,
        I2C1,
        PB6,
        PB7,
        DMA1_CH7,
        DMA1_CH6
    }

    get_led_from_matrix_coordinates! {
        [ C1_1 C1_2 C1_3 C1_4 C1_5  C1_6  C1_7  C1_8  C1_9  C1_10 C1_11 C1_12 C1_13 C1_14 C1_15 C2_15 ]
        [ C2_1 C2_2 C2_3 C2_4 C2_5  C2_6  C2_7  C2_8  C2_9  C2_10 C2_11 C2_12 C2_13 C2_14 C3_15 ]
        [ C3_1 C3_2 C3_3 C3_4 C3_5  C3_6  C3_7  C3_8  C3_9  C3_10 C3_11 C3_12 C3_13 C3_14 C4_15 ]
        [ C4_1 C4_2 C4_3 C4_4 C4_5  C4_6  C4_7  C4_8  C4_9  C4_10 C4_11 C4_12 C4_13 C4_14 C5_15 ]
        [ C5_2 C5_3 C5_6 C5_7 C5_10 C5_11 C5_12 C5_13 C5_14 ]
    }
}
```

## Keycodes

In your keyberon layout, you can use any of the enum members defined in `BacklightCommand`:

```rust
Toggle,
NextEffect,
PrevEffect,
SetEffect(BacklightEffect), // List of available effects depends on the chosen backlight mode
SetHue(u8), // RGB Matrix only
AdjustHue(i16), // RGB Matrix only
SetSaturation(u8), // RGB Matrix only
AdjustSaturation(i16), // RGB Matrix only
SetValue(u8),
AdjustValue(i16),
SetSpeed(u8),
AdjustSpeed(i16),
SetConfig(BacklightConfig),
SaveConfig, // normally called internally when the backlight config changes, only available if `storage` is enabled
SetTime(u32), // normally used internally for syncing LEDs for split keyboards
```

Example of usage:

```rust
use keyberon::action::Action::*;
use rumcake::backlight::animations::BacklightCommand::*;
use rumcake::keyboard::{Keyboard, Keycode::*};

/* ... */

    build_layout! {
        {
            [ Escape {Custom(Backlight(Toggle))} A B C]
        }
    }
```

## To-do List

- [ ] RGB Backlight animations
