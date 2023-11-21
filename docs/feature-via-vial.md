# Via/Vial

> [!WARNING]
> This feature is still a work in progress.

`rumcake` implements version 12 of the Via protocol, and version 6 of the Vial protocol.

<!--toc:start-->

- [Setup](#setup)
  - [Required Cargo features](#required-cargo-features)
  - [Required code](#required-code)
  - [Vial Definitions](#vial-definitions)
- [Keycode support](#keycode-support)
- [To-do List](#to-do-list)
<!--toc:end-->

## Setup

### Required Cargo features

You must enable the following `rumcake` features:

- `via` or `vial` (depending on what you want to use)
- `storage` (optional, if you want to save changes you make in the Via/Vial app)

### Required code

To set up Via support, your keyboard must implement the `ViaKeyboard` trait, and add `via` to your `keyboard` macro invocation:

```rust
use rumcake::keyboard;

#[keyboard(usb, via)]
struct MyKeyboard;

// ...

// Via setup
use rumcake::via::ViaKeyboard;
impl ViaKeyboard for MyKeyboard {}
```

If you are using Vial, you must also implement `VialKeyboard` in addition to the previous traits.
Instead of using `via` in your `keyboard` macro invocation, you should use `vial` instead.
You must also follow the instructions in the [VIAL Definitions](#vial-definitions) section.

```rust
// GENERATED_KEYBOARD_DEFINITION comes from _generated.rs, which is made by the build script.
#[cfg(vial)]
include!(concat!(env!("OUT_DIR"), "/_generated.rs"));

#[keyboard(usb, vial)] // use `vial` instead of `via`
struct MyKeyboard;

// ...

use rumcake::vial::VialKeyboard;
impl VialKeyboard for MyKeyboard {
    const VIAL_KEYBOARD_UID: [u8; 8] = [0; 8]; // Change this
    const VIAL_UNLOCK_COMBO: &'static [(u8, u8)] = [(0, 1), (0, 0)]; // Matrix positions used to unlock VIAL (row, col), set it to whatever you want
    const KEYBOARD_DEFINITION: &'static [u8] = &GENERATED_KEYBOARD_DEFINITION;
}
```

### Vial Definitions

To compile your Vial definition into the firmware, you must minify and LZMA compress your JSON definition file, and
pass the raw bytes to `KEYBOARD_DEFINITION` in the `VialKeyboard` trait implementation.

The [basic template](https://github.com/Univa/rumcake-templates/tree/main/rumcake-basic-template) shows how you can achieve this using a build script:

- Place a `definition.json` file in your `./src` folder.
- The build script will check for the `vial` feature flag, then minify and LZMA compress the JSON data.
- The bytes are written to a `GENERATED_KEYBOARD_DEFINITION` constant that can then be used in your `main.rs` file, similarly to how it's done in the previous section.

This constant must be used in your trait implementation as shown previously.

## Keycode support

`rumcake` does not support all the keycodes that Via/Vial shows in the app. Currently, the following keycodes are functional:

- Basic keycodes (Basic tab in Via/Vial, or those available in HID keyboard reports)
- Lighting keycodes, except for `QK_BACKLIGHT_TOGGLE_BREATHING`. RGB keycodes only work for underglow, not an RGB backlight matrix.
- Momentary layers, and default layers (MO(x) and DF(x))
- Custom keycodes (`customKeycodes` in your JSON definition)
- QK_OUTPUT_BLUETOOTH and QK_OUTPUT_USB

Attempts to use unsupported keycodes will not result in any changes to your layout. It may show in the app, but reloading will revert the keycodes back to their previous state.

For more information on how these keycodes get converted into `keyberon` actions, see [rumcake/src/via/protocol_12/keycodes.rs](../rumcake/src/via/protocol_12/keycodes.rs)

## To-do List

- [ ] Sync backlight and underglow commands from central to peripherals on split keyboard setups
- [ ] Tap dance, one shot, layer toggling, one shot layer keycodes (and other keycodes in the "Layers" submenu)
- [ ] Dynamic keymap macros (Via)
- [ ] QMK settings (Vial)
- [ ] Dynamic keymap tap dance, combo, key overrides (Vial)
