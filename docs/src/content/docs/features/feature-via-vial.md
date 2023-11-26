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
Optionally, you can add `use_storage` to the macro invocation to use the specified storage driver to save changes you make in
the Via or Vial app. If you specify `use_storage`, be sure to also add `setup_via_storage_buffers!(<struct_name>)` to your
`ViaKeyboard` implementation.

```rust ins={5,9-12}
use rumcake::keyboard;

#[keyboard(
    // somewhere in your keyboard macro invocation ...
    via
)]
struct MyKeyboard;

// Via setup
use rumcake::via::ViaKeyboard;
impl ViaKeyboard for MyKeyboard {}
```

:::caution
By default, changes you make to your keyboard in the Via app (e.g. changing your layout,
lighting settings, etc.) will **NOT** be saved by default.

Optionally, you can add `use_storage`, and a `storage` driver to save Via data.

```rust del={5,15} ins={6-9,16-18}
use rumcake::keyboard;

#[keyboard(
    // somewhere in your keyboard macro invocation ...
    via,
    via(
        use_storage // Optional, if you want to save via configuration
    ),
    storage = "internal" // You need to specify a storage driver if you specified `use_storage`. See feature-storage.md for more information.
)]
struct MyKeyboard;

// Via setup
use rumcake::via::ViaKeyboard;
impl ViaKeyboard for MyKeyboard {}
impl ViaKeyboard for MyKeyboard {
    rumcake::setup_via_storage_buffers!(MyKeyboard); // Required if you specify `use_storage`
}
```

You will need to do additional setup for your selected storage driver as well.
For more information, see the docs for the [storage feature](../feature-storage).
:::

If you are using Vial, you must also implement `VialKeyboard` in addition to `ViaKeyboard`.
Instead of using `via` in your `keyboard` macro invocation, you should use `vial`.

The following code example shows how to implement the `VialKeyboard` trait, and uses a build script to
implement `KEYBOARD_DEFINITION`. Please follow the instructions in the [Vial Definitions](#compiling-vial-definitions) section.

```rust del={7} ins={1-3,8,20-26}
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
use rumcake::via::ViaKeyboard;
impl ViaKeyboard for MyKeyboard {
    // rumcake::setup_via_storage_buffers!(MyKeyboard); // Optional, only required if you specify `use_storage`
}

use rumcake::vial::VialKeyboard;
impl VialKeyboard for MyKeyboard {
    const VIAL_KEYBOARD_UID: [u8; 8] = [0; 8]; // Change this
    const VIAL_UNLOCK_COMBO: &'static [(u8, u8)] = [(0, 1), (0, 0)]; // Matrix positions used to unlock VIAL (row, col), set it to whatever you want
    const KEYBOARD_DEFINITION: &'static [u8] = &GENERATED_KEYBOARD_DEFINITION;
    // rumcake::setup_vial_storage_buffers!(MyKeyboard); // Optional, only required if you specify `use_storage`
}
```

:::caution
Similarly to the previous caution, if you specify `use_storage`, be sure to also add
`setup_vial_storage_buffers!(<struct_name>)` to the `VialKeyboard` implementation.
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

# Keycode support

`rumcake` does not support all the keycodes that Via/Vial shows in the app. Currently, the following keycodes are functional:

- Basic keycodes (Basic tab in Via/Vial, or those available in HID keyboard reports)
- Lighting keycodes, except for `QK_BACKLIGHT_TOGGLE_BREATHING`. RGB keycodes only work for underglow, not an RGB backlight matrix.
- Momentary layers, and default layers (MO(x) and DF(x))
- Custom keycodes (`customKeycodes` in your JSON definition)
- QK_OUTPUT_BLUETOOTH and QK_OUTPUT_USB

Attempts to use unsupported keycodes will not result in any changes to your layout. It may show in the app, but reloading will revert the keycodes back to their previous state.

For more information on how these keycodes get converted into `keyberon` actions, see [rumcake/src/via/protocol_12/keycodes.rs](https://github.com/Univa/rumcake/blob/4a7dfb8f9b04c321a43c35bc0d96fbc6afaabad2/rumcake/src/via/protocol_12/keycodes.rs#L1082)

# To-do List

- [ ] Tap dance, one shot, layer toggling, one shot layer keycodes (and other keycodes in the "Layers" submenu)
- [ ] Dynamic keymap macros (Via)
- [ ] QMK settings (Vial)
- [ ] Dynamic keymap tap dance, combo, key overrides (Vial)
