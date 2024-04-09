---
title: Backlighting
description: How to configure your keyboard with backlighting.
---

:::caution
This feature is still a work in progress. For a list of features that still need
to be implemented, check the [to-do list](#to-do-list).
:::

Backlighting can be used to provide lighting from underneath your keycaps, typically via an LED
inside or underneath the individual switches. `rumcake` supports different types of backlighting
depending on LED type, and whether LEDs are individually addressable.

# Setup

## Required Cargo features

You must enable the following `rumcake` features:

- Exactly one of:
  - `simple-backlight` (single color backlighting, all LEDs have the same brightness)
  - `simple-backlight-matrix` (single color backlighting, each LED in the matrix is individually addressable)
  - `rgb-backlight-matrix` (RGB backlighting, each LED in the matrix is individually addressable)
- Feature flag for one of the [available backlight drivers](#available-drivers) that you would like to use
- `storage` (optional, if you want to save your backlight settings)

Some drivers may not be able to support all backlight types.

## Required code

To set up backlighting, you must create a new type to implement traits on.
Then, you can add `<backlight_type>(id = <type>, driver_setup_fn = <setup_fn>)` to your `#[keyboard]` macro
invocation. Your new type must implement the appropriate traits depending on the type of lighting you're using:

- `simple_backlight`: [`SimpleBacklightDevice`](/rumcake/api/nrf52840/rumcake/lighting/simple_backlight/trait.SimpleBacklightDevice.html)
- `simple_backlight_matrix`: [`SimpleBacklightMatrixDevice`](/rumcake/api/nrf52840/rumcake/lighting/simple_backlight_matrix/trait.SimpleBacklightMatrixDevice.html)
- `rgb_backlight_matrix`: [`RGBBacklightMatrixDevice`](/rumcake/api/nrf52840/rumcake/lighting/rgb_backlight_matrix/trait.RGBBacklightMatrixDevice.html)

The `driver_setup_fn` must be an async function that has no parameters, and returns a type that implements the appropriate
driver trait:

- `simple_backlight`: [`SimpleBacklightDriver<T>`](/rumcake/api/nrf52840/rumcake/lighting/simple_backlight/trait.SimpleBacklightDriver.html)
- `simple_backlight_matrix`: [`SimpleBacklightMatrixDriver<T>`](/rumcake/api/nrf52840/rumcake/lighting/simple_backlight_matrix/trait.SimpleBacklightMatrixDriver.html)
- `rgb_backlight_matrix`: [`RGBBacklightMatrixDriver<T>`](/rumcake/api/nrf52840/rumcake/lighting/rgb_backlight_matrix/trait.RGBBacklightMatrixDriver.html)

```rust ins={5-8,13-22}
use rumcake::keyboard;

#[keyboard(
    // somewhere in your keyboard macro invocation ...
    simple_backlight_matrix( // TODO: Change this to `rgb_backlight_matrix` or `simple_backlight` if that's what you want.
        id = MyKeyboardLighting,
        driver_setup_fn = my_backlight_setup,
    )
)]
struct MyKeyboard;

// Backlight configuration
use rumcake::lighting::simple_backlight_matrix::{SimpleBacklightMatrixDevice, SimpleBacklightMatrixDriver};
struct MyKeyboardLighting; // New type to implement lighting traits on
async fn my_backlight_setup() -> impl SimpleBacklightMatrixDriver<MyKeyboardLighting> {
    // TODO: We will fill this out soon!
    todo!()
}
impl SimpleBacklightMatrixDevice for MyKeyboardLighting {
    // optionally, set FPS
    const FPS: usize = 20;
}
```

:::caution
By default, changes you make to backlight settings while the keyboard is on (e.g. changing brightness,
hue, saturation, effect, etc.) will **NOT** be saved by default.

Optionally, you can add `use_storage`, and a `storage` driver to save backlight config data.

```rust ins={8,10}
use rumcake::keyboard;

#[keyboard(
    // somewhere in your keyboard macro invocation ...
    simple_backlight_matrix( // TODO: Change this to `rgb_backlight_matrix` or `simple_backlight` if that's what you want.
        id = MyKeyboardLighting,
        driver_setup_fn = my_backlight_setup,
        use_storage // Optional, if you want to save backlight configuration
    ),
    storage(driver = "internal") // You need to specify a storage driver if you enabled `use_storage`. See feature-storage.md for more information.
)]
struct MyKeyboard;
```

You will need to do additional setup for your selected storage driver as well.
For more information, see the docs for the [storage feature](../feature-storage/).
:::

If you're implementing a backlight matrix (either the `simple-backlight-matrix` or `rgb-backlight-matrix`), your keyboard must also implement the `BacklightMatrixDevice` trait:

```rust ins={14,25-42}
use rumcake::keyboard;

#[keyboard(
    // somewhere in your keyboard macro invocation ...
    simple_backlight_matrix( // TODO: Change this to `rgb_backlight_matrix` or `simple_backlight` if that's what you want.
        id = MyKeyboardLighting,
        driver_setup_fn = my_backlight_setup,
    )
)]
struct MyKeyboard;

// Backlight configuration
use rumcake::lighting::simple_backlight_matrix::{SimpleBacklightMatrixDevice, SimpleBacklightMatrixDriver};
use rumcake::lighting::{BacklightMatrixDevice, setup_backlight_matrix};
struct MyKeyboardLighting;
async fn my_backlight_setup() -> impl SimpleBacklightMatrixDriver<MyKeyboardLighting> {
    // TODO: We will fill this out soon!
    todo!()
}
impl SimpleBacklightMatrixDevice for MyKeyboardLighting {
    // optionally, set FPS
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
        led_flags: { // must have same number of rows and columns as the layout above
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
Your backlighting matrix does not necessarily need to have the same dimensions as your switch matrix.

Note that for reactive effects, matrix positions will map directly to LED positions. For example, pressing
a key at switch matrix position row 0, column 0, will correspond to the LED at row 0, column 0 on your LED matrix.
:::

Lastly, you must set up the driver. To do this, you need to complete your `driver_setup_fn` by constructing the driver.
You can [check the API reference for your chosen driver](/rumcake/api/nrf52840/rumcake/drivers/index.html) for a set up
function or macro to make this process easier.

Depending on the driver, you may also need to implement the appropriate trait that corresponds to your chosen driver.
Check the [list of available backlight drivers](#available-drivers) for this information.

For example, with `is31fl3731`, you must implement `IS31FL3731BacklightDriver`, and you can use the `setup_is31fl3731!` macro to set up the driver:

```rust del={9-10} ins={3-5,11-23,25-34}
use rumcake::lighting::simple_backlight_matrix::{SimpleBacklightMatrixDevice, SimpleBacklightMatrixDriver};
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
        device: MyKeyboardLighting, // Must be a type that implements IS31FL3731BacklightDriver
        address: 0b1110100, // see https://github.com/qmk/qmk_firmware/blob/d9fa80c0b0044bb951694aead215d72e4a51807c/docs/feature_rgb_matrix.md#is31fl3731-idis31fl3731
        i2c: setup_i2c! { // Note: The arguments of setup_i2c may change depending on platform. This assumes STM32.
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
    // This must have the same number of rows and columns as specified in your `BacklightMatrixDevice` implementation.
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
The IS31FL3731 driver setup above assumes usage of a `simple-backlight-matrix`. If you want
an RGB matrix, there is a separate `rumcake::drivers::is31fl3731::backlight::get_led_from_rgb_matrix_coordinates` macro.
:::

# Keycodes

Depending on the backlight type you chose, you can use certain version of the `BacklightCommand`
enum in your `keyberon` layout:

- [Simple Backlight Commands](/rumcake/api/nrf52840/rumcake/backlight/simple_backlight/animations/enum.BacklightCommand.html)
- [Simple Backlight Matrix Commands](/rumcake/api/nrf52840/rumcake/backlight/simple_backlight_matrix/animations/enum.BacklightCommand.html)
- [RGB Backlight Matrix Commands](/rumcake/api/nrf52840/rumcake/backlight/rgb_backlight_matrix/animations/enum.BacklightCommand.html)

```rust
Toggle,
TurnOn,
TurnOff,
NextEffect,
PrevEffect,
SetEffect(BacklightEffect), // List of available effects depends on the chosen backlight mode
SetHue(u8), // RGB Matrix only
IncreaseHue(u8), // RGB Matrix only
DecreaseHue(u8), // RGB Matrix only
SetSaturation(u8), // RGB Matrix only
IncreaseSaturation(u8), // RGB Matrix only
DecreaseSaturation(u8), // RGB Matrix only
SetValue(u8),
IncreaseValue(u8),
DecreaseValue(u8),
SetSpeed(u8),
IncreaseSpeed(u8),
DecreaseSpeed(u8),
SaveConfig, // normally called internally when the backlight config changes, only available if `storage` is enabled
ResetTime, // normally used internally for syncing LEDs for split keyboards
```

In your `keyberon` layout, you can use `{Custom(SimpleBacklight(<command>))}`,
`{Custom(SimpleBacklightMatrix(<command>))}`, `{Custom(RGBBacklightMatrix(<command>))}`,
depending on what type of backlight system you are using.

Additionally, you must choose the backlight system that the keycodes will correspond to by
implementing one of the associated types `SimpleBacklightDeviceType`, `SimpleBacklightMatrixDeviceType`,
`RGBBacklightDeviceType`.

Example of usage:

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

# To-do List

- [ ] RGB Backlight animations

# Available Drivers

| Name           | Feature Flag     | Required Traits                                                                                                                          |
| -------------- | ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| IS31FL3731     | `is31fl3731`     | [`IS31FL3731BacklightDriver`](/rumcake/api/nrf52840/rumcake/drivers/is31fl3731/backlight/trait.IS31FL3731BacklightDriver.html)           |
| WS2812 Bitbang | `ws2812_bitbang` | [`WS2812BitbangBacklightDriver`](/rumcake/api/nrf52840/rumcake/drivers/ws2812_bitbang/backlight/trait.WS2812BitbangBacklightDriver.html) |
