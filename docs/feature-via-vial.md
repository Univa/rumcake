# Via/Vial

**⚠ This feature is still a work in progress. VIA and VIAL are not yet functional**

<!--toc:start-->

- [Setup](#setup)
  - [Required Cargo features](#required-cargo-features)
  - [Required code](#required-code)
  - [VIAL Definitions](#vial-definitions)
- [To-do List](#to-do-list)
<!--toc:end-->

## Setup

### Required Cargo features

You must enable the following `rumcake` features:

- `via` or `vial` (depending on what you want to use)
- `storage`

### Required code

To set up VIA support, your keyboard must implement the `ViaKeyboard` and `KeyboardWithEEPROM` trait:

```rust
use rumcake::keyboard;

#[keyboard]
struct MyKeyboard;

// EEPROM setup
use rumcake::eeprom::KeyboardWithEEPROM;
impl KeyboardWithEEPROM for MyKeyboard {}

// Via setup
use rumcake::via::ViaKeyboard;
impl ViaKeyboard for MyKeyboard {}
```

If you are using VIAL, you must also implement `VialKeyboard` in addition to the previous traits.
You must also follow the instructions in the [VIAL Definitions](#vial-definitions) section.

```rust
// GENERATED_KEYBOARD_DEFINITION comes from _generated.rs
#[cfg(vial)]
include!(concat!(env!("OUT_DIR"), "/_generated.rs"));

use rumcake::vial::VialKeyboard;
impl VialKeyboard for MyKeyboard {
    const VIAL_KEYBOARD_UID: [u8; 8] = [0; 8]; // Set this to whatever you want
    const VIAL_UNLOCK_COMBO: &'static [(u8, u8)] = [(0, 1), (0, 0)]; // Matrix positions used to unlock VIAL (row, col), set it to whatever you want
    const KEYBOARD_DEFINITION: &'static [u8] = &GENERATED_KEYBOARD_DEFINITION;
}
```

### VIAL Definitions

_To compile your VIAL definition into the firmware, you must place a `definition.json` file in your `./src` folder._

You will notice in the trait definition from the previous section, we used a keyboard definition from `_generated.rs`.
At build time, your `definition.json` file gets lzma-compressed, and the bytes are written to a constant called `GENERATED_KEYBOARD_DEFINITION` in `_generated.rs`.

This constant must be used in your trait implementation as shown previously.

## To-do List

- [ ] Dynamic keymaps (VIA)
- [ ] QMK settings (VIAL)
