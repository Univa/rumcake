---
title: Via and Vial
description: How to configure your keyboard to support the Via and Vial app.
---

:::caution
This feature is still a work in progress. For a list of features that still need
to be implemented, check the [to-do list](#to-do-list).
:::

[Via](https://www.caniusevia.com/) provides you the ability to re-configure your keyboard using the [Via app](https://usevia.app/),
instead of building and re-flashing your firmware. Via works by reading a JSON definition specific to your
keyboard. In order for the Via app to support your keyboard, these definitions need to be submitted
[a repository](https://github.com/the-via/keyboards), or sideloaded using Via's Design tab.

[Vial](https://get.vial.today/) is an alternative, which aims to be decentralized.
To support your keyboard, the JSON definitions are built into the firmware itself, and the Vial
app will load the JSON data at runtime. This removes the need for JSON definitions to be uploaded
to a central repository.

`rumcake` provides you with the option of using Via, or Vial.

At this time, `rumcake` implements:

- Via protocol version 12, which requires Via V3 definitions.
- Vial protocol version 6, which is based on Via V2.

# Setup

## Required Cargo features

You must enable the following `rumcake` features:

- `via` or `vial` (depending on what you want to use)
- `storage` (optional, if you want to save changes you make in the Via/Vial app)

## Required code

To set up Via and Vial support, your keyboard must implement the `ViaKeyboard` trait, and add `via` to your `keyboard` macro invocation.

Certain Via features need to be configured manually as well:

- For macros, you need to implement [`DYNAMIC_KEYMAP_MACRO_BUFFER_SIZE`](/rumcake/api/nrf52840/rumcake/via/trait.ViaKeyboard.html#associatedconstant.DYNAMIC_KEYMAP_MACRO_EEPROM_SIZE)
  and [`DYNAMIC_KEYMAP_MACRO_COUNT`](/rumcake/api/nrf52840/rumcake/via/trait.ViaKeyboard.html#associatedconstant.DYNAMIC_KEYMAP_MACRO_COUNT)
  - This can be done trivially with the `setup_macro_buffer` macro
- If you are using some form of backlighting (`simple-backlight`, `simple-backlight-matrix` or `rgb-backlight-matrix`), you need to change [`BACKLIGHT_TYPE`](https://univa.github.io/rumcake/api/nrf52840/rumcake/via/trait.ViaKeyboard.html#associatedconstant.BACKLIGHT_TYPE).
  This controls how `QK_BACKLIGHT` keycodes get converted to `keyberon` actions. In other words, it controls the behaviour of `BL_` prefixed keycodes in the Via app.

For other configurable Via options, see the [`ViaKeyboard` trait](/rumcake/api/nrf52840/rumcake/via/trait.ViaKeyboard.html).

```rust ins={5,9-17}
use rumcake::keyboard;

#[keyboard(
    // somewhere in your keyboard macro invocation ...
    via
)]
struct MyKeyboard;

// Via setup
use rumcake::via::{BacklightType, setup_macro_buffer, ViaKeyboard};
impl ViaKeyboard for MyKeyboard {
    // OPTIONAL, this example assumes you are using simple-backlight-matrix.
    const BACKLIGHT_TYPE: Option<BacklightType> = Some(BacklightType::SimpleBacklightMatrix)

    // OPTIONAL, include this if you want to create macros using the Via app.
    setup_macro_buffer!(512, 16) // Max number of bytes that can be taken up by macros, followed by the max number of macros that can be created.
}
```

:::caution
By default, changes you make to your keyboard in the Via app (e.g. changing your layout,
lighting settings, etc.) will **NOT** be saved by default.

Optionally, you can add `use_storage`, and a `storage` driver to save Via data.

```rust del={5} ins={6-9,22-23}
use rumcake::keyboard;

#[keyboard(
    // somewhere in your keyboard macro invocation ...
    via,
    via(
        use_storage // Optional, if you want to save Via configuration
    ),
    storage = "internal" // You need to specify a storage driver if you specified `use_storage`. See feature-storage.md for more information.
)]
struct MyKeyboard;
```

You will need to do additional setup for your selected storage driver as well.
For more information, see the docs for the [storage feature](../feature-storage/).
:::

If you are using Vial, you must also implement `VialKeyboard` in addition to `ViaKeyboard`.
Instead of using `via` in your `keyboard` macro invocation, you should use `vial`.

The following code example shows how to implement the `VialKeyboard` trait, and uses a build script to
implement `KEYBOARD_DEFINITION`. Please follow the instructions in the [Vial Definitions](#compiling-vial-definitions) section.

For other configurable Vial options, see the [`VialKeyboard` trait](/rumcake/api/nrf52840/rumcake/vial/trait.VialKeyboard.html)

```rust del={7} ins={1-3,8,24-30}
// GENERATED_KEYBOARD_DEFINITION comes from _generated.rs, which is made by the build.rs script.
#[cfg(vial)]
include!(concat!(env!("OUT_DIR"), "/_generated.rs"));

#[keyboard(
    // somewhere in your keyboard macro invocation ...
    via
    vial
)]
struct MyKeyboard;

// ...

// Via setup
use rumcake::via::{setup_macro_buffer, ViaKeyboard};
impl ViaKeyboard for MyKeyboard {
    // OPTIONAL, this example assumes you are using simple-backlight-matrix.
    const BACKLIGHT_TYPE: Option<BacklightType> = Some(BacklightType::SimpleBacklightMatrix)

    // OPTIONAL, include this if you want to create macros using the Via app.
    setup_macro_buffer!(512, 16) // Max number of bytes that can be taken up by macros, followed by the max number of macros that can be created.
}

use rumcake::vial::VialKeyboard;
impl VialKeyboard for MyKeyboard {
    const VIAL_KEYBOARD_UID: [u8; 8] = [0; 8]; // Change this
    const VIAL_UNLOCK_COMBO: &'static [(u8, u8)] = &[(0, 1), (0, 0)]; // Matrix positions used to unlock VIAL (row, col), set it to whatever you want
    const KEYBOARD_DEFINITION: &'static [u8] = &GENERATED_KEYBOARD_DEFINITION;
}
```

:::caution
Similarly to the previous caution, you need to specify `use_storage`, to save Vial data:

```rust del={5} ins={6-9,22-23}
use rumcake::keyboard;

#[keyboard(
    // somewhere in your keyboard macro invocation ...
    vial,
    vial(
        use_storage // Optional, if you want to save Vial configuration
    ),
    storage = "internal" // You need to specify a storage driver if you specified `use_storage`. See feature-storage.md for more information.
)]
struct MyKeyboard;
```

:::

## Compiling Vial Definitions

To compile your Vial definition into the firmware, you must minify and LZMA compress your JSON definition file, and
pass the raw bytes to `KEYBOARD_DEFINITION` in the `VialKeyboard` trait implementation.

The [basic template](https://github.com/Univa/rumcake-templates/tree/main/rumcake-basic-template) shows how you can achieve this using a build script (`build.rs`).
The build script does the following:

- The build script will search for and open the JSON definition file for your keyboard. Place it at `./src/definition.json`.
- The build script will check for the `vial` feature flag, then minify and LZMA compress the JSON data.
- The resulting bytes are written to a `GENERATED_KEYBOARD_DEFINITION` constant.

The `GENERATED_KEYBOARD_DEFINITION` constant can be used in your `VialKeyboard` trait implementation for `KEYBOARD_DEFINITION`.
Check the code example shown previously to see how to use this constant.

## Recommended Via V3 Custom UI Definitions

If you are using regular Via (non-Vial), it is recommended to use the provided Custom UI
menus below to interact with `rumcake`'s extra features using Via. Feel free to choose
only the ones you need.

To add the menus you need, add the JSON directly to your `"menus"` field in your keyboard definition file.

:::note
Although Via V3 provides [built-in `qmk_*` menus](https://www.caniusevia.com/docs/built_in_menus) to
use lighting features, `rumcake`'s lighting system is not designed to be compatible with these menus.
This is due to subtle differences in how `rumcake` handles effect IDs, lighting speed, enabling/disabling, etc,
so using the custom UI below is preferred if you are using regular Via. If you are using Vial, `rumcake`
will attempt to support the Via/Vial app's built-in lighting menus.
:::

:::note
No menu for RGB matrix is provided. RGB backlight animations still need to be implemented.
:::

### Underglow Menu

```json ins={10-79}
{
  "name": "My Keyboard",
  "vendorId": "0xDEAD",
  "productId": "0xBEEF",
  // ...
  "menus": [
    {
      "label": "Lighting",
      "content": [
        {
          "label": "Underglow",
          "content": [
            {
              "label": "Enabled",
              "type": "toggle",
              "content": [
                "rumcake__via__protocol_12__ViaRGBLightValue__Enabled",
                2,
                5
              ]
            },
            {
              "showIf": "{rumcake__via__protocol_12__ViaRGBLightValue__Enabled} == 1",
              "label": "Brightness",
              "type": "range",
              "options": [0, 255],
              "content": [
                "rumcake__via__protocol_12__ViaRGBLightValue__Brightness",
                2,
                1
              ]
            },
            {
              "showIf": "{rumcake__via__protocol_12__ViaRGBLightValue__Enabled} == 1",
              "label": "Color",
              "type": "color",
              "content": [
                "rumcake__via__protocol_12__ViaRGBLightValue__Color",
                2,
                4
              ]
            },
            {
              "showIf": "{rumcake__via__protocol_12__ViaRGBLightValue__Enabled} == 1",
              "label": "Speed",
              "type": "range",
              "options": [0, 255],
              "content": [
                "rumcake__via__protocol_12__ViaRGBLightValue__EffectSpeed",
                2,
                3
              ]
            },
            {
              "showIf": "{rumcake__via__protocol_12__ViaRGBLightValue__Enabled} == 1",
              "label": "Effect",
              "type": "dropdown",
              "options": [
                "Solid",
                "Breathing",
                "Rainbow Mood",
                "Rainbow Swirl",
                "Snake",
                "Knight",
                "Christmas",
                "Static Gradient",
                "RGB Test",
                "Alternating",
                "Twinkle",
                "Reactive"
              ],
              "content": [
                "rumcake__via__protocol_12__ViaRGBLightValue__Effect",
                2,
                2
              ]
            }
          ]
        }
      ]
    }
  ]
  // ...
}
```

### Simple Backlight Menu

```json ins={10-56}
{
  "name": "My Keyboard",
  "vendorId": "0xDEAD",
  "productId": "0xBEEF",
  // ...
  "menus": [
    {
      "label": "Lighting",
      "content": [
        {
          "label": "Backlight",
          "content": [
            {
              "label": "Enabled",
              "type": "toggle",
              "content": [
                "rumcake__via__protocol_12__ViaBacklightValue__Enabled",
                1,
                4
              ]
            },
            {
              "showIf": "{rumcake__via__protocol_12__ViaBacklightValue__Enabled} == 1",
              "label": "Brightness",
              "type": "range",
              "options": [0, 255],
              "content": [
                "rumcake__via__protocol_12__ViaBacklightValue__Brightness",
                1,
                1
              ]
            },
            {
              "showIf": "{rumcake__via__protocol_12__ViaBacklightValue__Enabled} == 1",
              "label": "Speed",
              "type": "range",
              "options": [0, 255],
              "content": [
                "rumcake__via__protocol_12__ViaBacklightValue__EffectSpeed",
                1,
                3
              ]
            },
            {
              "showIf": "{rumcake__via__protocol_12__ViaBacklightValue__Enabled} == 1",
              "label": "Effect",
              "type": "dropdown",
              "options": ["Solid", "Breathing", "Reactive"],
              "content": [
                "rumcake__via__protocol_12__ViaBacklightValue__Effect",
                1,
                2
              ]
            }
          ]
        }
      ]
    }
  ]
  // ...
}
```

### Simple Backlight Matrix Menu

```json ins={10-81}
{
  "name": "My Keyboard",
  "vendorId": "0xDEAD",
  "productId": "0xBEEF",
  // ...
  "menus": [
    {
      "label": "Lighting",
      "content": [
        {
          "label": "Backlight",
          "content": [
            {
              "label": "Enabled",
              "type": "toggle",
              "content": [
                "rumcake__via__protocol_12__ViaLEDMatrixValue__Enabled",
                5,
                4
              ]
            },
            {
              "showIf": "{rumcake__via__protocol_12__ViaLEDMatrixValue__Enabled} == 1",
              "label": "Brightness",
              "type": "range",
              "options": [0, 255],
              "content": [
                "rumcake__via__protocol_12__ViaLEDMatrixValue__Brightness",
                5,
                1
              ]
            },
            {
              "showIf": "{rumcake__via__protocol_12__ViaLEDMatrixValue__Enabled} == 1",
              "label": "Speed",
              "type": "range",
              "options": [0, 255],
              "content": [
                "rumcake__via__protocol_12__ViaLEDMatrixValue__EffectSpeed",
                5,
                3
              ]
            },
            {
              "showIf": "{rumcake__via__protocol_12__ViaLEDMatrixValue__Enabled} == 1",
              "label": "Effect",
              "type": "dropdown",
              "options": [
                "Solid",
                "Alphas Mods",
                "Gradient Up Down",
                "Gradient Left Right",
                "Breathing",
                "Band",
                "Band Pin Wheel",
                "Band Spiral",
                "Cycle Left Right",
                "Cycle Up Down",
                "Cycle Out In",
                "Raindrops",
                "Dual Beacon",
                "Wave Left Right",
                "Wave Up Down",
                "Reactive",
                "Reactive Wide",
                "Reactive Cross",
                "Reactive Nexus",
                "Reactive Splash"
              ],
              "content": [
                "rumcake__via__protocol_12__ViaLEDMatrixValue__Effect",
                5,
                2
              ]
            }
          ]
        }
      ]
    }
  ]
  // ...
}
```

# Keycode support

`rumcake` does not support all the keycodes that Via/Vial shows in the app. Currently, the following keycodes are functional:

- Basic keycodes (Basic tab in Via/Vial, or those available in HID keyboard reports)
- Lighting keycodes, except for `QK_BACKLIGHT_TOGGLE_BREATHING`. RGB keycodes only work for underglow, not an RGB backlight matrix.
- Momentary layers (`MO(x)`)
- Default layers (`DF(x)`)
- Toggle layers (`TG(x)`)
- One-shot layers (`OSL(x)`)
- Macro keycodes (`M0`, `M1`, `M2` ...)
- Custom keycodes (`customKeycodes` in your JSON definition)
- Certain media keycodes. Support for this must be enabled manually. Check the ["Media Keys" doc](../feature-media-keys/)
- QK_OUTPUT_BLUETOOTH and QK_OUTPUT_USB

You can assume that any keycodes not listed above are not supported.

Attempts to use unsupported keycodes will not result in any changes to your layout. It may show in the Via app briefly, but reloading the app will revert the keycodes back to their previous state.

For more information on how these keycodes get converted into `keyberon` actions, see [rumcake/src/via/protocol_12/keycodes.rs](https://github.com/Univa/rumcake/blob/4a7dfb8f9b04c321a43c35bc0d96fbc6afaabad2/rumcake/src/via/protocol_12/keycodes.rs#L1082)

## Limitations

For the keycodes that are supported, there are still some limitations that apply. Usually these limitations
are due to memory constraints, while still allowing for the flexibility that `keyberon`'s actions provide.

It's recommended to compile any non-basic actions directly into your firmware as opposed to assigning them
through Via.

- One-shot Layer keycodes **assigned through Via** only works for layers 0 to 15.
  - One-shot actions that are already compiled into your `keyberon` layout can still work with greater layer numbers.
- Sequence actions compiled into your `keyberon` layout will not show up in the Via app, it will show up as `0xFFFF`.
- For Vial, using delay events and tap/press/release events with non-basic keycodes (higher than 0x00FF) in macros will not work. Using them will abort the macro when the event is executed.
- For backlighting keycodes to work, you need to modify the `BACKLIGHT_TYPE` constant in your `ViaKeyboard` implementation. This defines how the backlighting keycodes get converted.
- RGB keycodes only work for underglow, not an RGB backlight matrix.

# To-do List

- [ ] Tap-toggle, one shot mod keycodes (and other keycodes in the "Layers" submenu)
- [ ] QMK settings (Vial)
- [ ] Dynamic keymap tap dance, combo, key overrides (Vial)
- [ ] Vial macro support (delays and non-basic keycodes)
