---
title: Media Keys / Consumer Controls
description: How to enable media keys for your keyboard layout.
---

`rumcake` can be configured to send HID reports that consist of [variants in the USB consumer usage page](https://docs.rs/usbd-human-interface-device/latest/usbd_human_interface_device/page/enum.Consumer.html).
This includes media controls, application launchers, application controls, etc.

# Setup

## Required Cargo features

You must enable the following `rumcake` features:

- `media-keycodes`

## Required code

After enabling the `media-keycodes` feature, you can start using the `Keycode::Media` variants in your `KeyboardLayout` implementation:
The `Keycode::Media` variant must contain a `usbd_human_interface_device::page::Consumer` variant, which is re-exported as `rumcake::keyboard::Consumer`.

Example of usage:

```rust ins={2} ins="{Custom(Media(VolumeIncrement))}"
use keyberon::action::Action::*;
use rumcake::keyboard::{Consumer::*, Keycode::Media};

/* ... */

    build_layout! {
        {
            [ Escape {Custom(Media(VolumeIncrement))} A B C]
        }
    }
```
