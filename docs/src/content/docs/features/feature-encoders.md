---
title: Encoders
description: How to add EC11-compatible encoders to your device.
---

:::caution
This feature is still a work in progress. For a list of features that still need
to be implemented, check the [to-do list](#to-do-list).
:::

This document contains information about how to add EC11-compatible encoders to your device.

# Setup

## Required code

To set up your keyboard for use with encoders, you must add `encoders` to your `#[keyboard]` macro invocation,
and your keyboard must implement the `DeviceWithEncoders` trait.

This can be done easily by using the `setup_encoders!` macro:

```rust ins={5,9-31}
use rumcake::keyboard;

#[keyboard(
    // somewhere in your keyboard macro invocation ...
    encoders
)]
struct MyKeyboard;

use rumcake::keyboard::DeviceWithEncoders;
impl DeviceWithEncoders for MyKeyboard {
    type Layout = Self;

    setup_encoders! {
        Encoder {
            sw_pin: input_pin!(PB12, EXTI12),
            sw_pos: (0, 0),
            output_a_pin: input_pin!(PB2, EXTI2),
            output_b_pin: input_pin!(PB1),
            cw_pos: (0, 1),
            ccw_pos: (0, 2),
        },
        Encoder {
            sw_pin: input_pin!(PA11, EXTI11),
            sw_pos: (1, 0),
            output_a_pin: input_pin!(PA3, EXTI3),
            output_b_pin: input_pin!(PA1),
            cw_pos: (1, 1),
            ccw_pos: (1, 2),
        },
    };
}

use rumcake::keyboard::{build_layout, KeyboardLayout};
impl KeyboardLayout for MyKeyboard {
    build_layout! {
        {
            [ A B C ]
            [ D E F ]
        }
        {
            [ G H I ]
            [ J K L ]
        }
    }
}
```

The `sw_pin` corresponds to the pin connected to the encoder's push button. `output_a_pin` and `output_b_pin`
correspond to the pins that pulse as the encoder rotates.

:::note
The current implementation of encoders relies on interrupts to avoid polling the encoders constantly.

For STM32, this means you need to specify the EXTI channels for `sw_pin` and `output_a_pin`. This can
be done by adding an extra argument to the `input_pin!` macro, as shown in the example above. This can
be omitted for other platforms.
:::

Encoders work by mapping their outputs to a position on your layout.
`type Layout = Self` tells rumcake to redirect encoder events to the implemented `KeyboardLayout` for `MyKeyboard`.

In the example above, here are the following mappings:

- Encoder 1 Button: `A` key (or `G` on the second layer)
- Encoder 1 Clockwise rotation: `B` key (or `H` on the second layer)
- Encoder 1 Counter-clockwise rotation: `C` key (or `I` on the second layer)
- Encoder 2 Button: `D` key (or `J` on the second layer)
- Encoder 2 Clockwise rotation: `H` key (or `K` on the second layer)
- Encoder 2 Counter-clockwise rotation: `I` key (or `L` on the second layer)

# To-do List

- [ ] Via(l) support
