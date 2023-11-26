---
title: Device Info, Matrix and Layout
description: How to configure your keyboard matrix and layout.
sidebar:
  order: 3
---

This document contains some information on how to define some basic information for your keyboard,
and how to set up a basic matrix and layout setup for your keyboard.

:::note
The following examples are for a non-split keyboard, which places the `KeyboardMatrix` and `KeyboardLayout`
implementations in the same entrypoint. If you are using a split keyboard, you can continue reading to learn how to
implement the `KeyboardMatrix` and `KeyboardLayout` traits, but the placement of the `impl` blocks will depend
on your split keyboard setup.

See the [split keyboard document](../../features/feature-split) for more information.
:::

# Keyboard Information

The basic trait that every device must implement to use `rumcake` is the `Keyboard` trait.
Here, you can define some basic information, including the name of the keyboard, the manufacturer,
version numbers, etc.

```rust ins={6-11}
use rumcake::keyboard;

#[keyboard(usb)]
pub struct MyKeyboard;

use rumcake::keyboard::Keyboard;
impl Keyboard for MyKeyboard {
    const MANUFACTURER: &'static str = "Me";
    const PRODUCT: &'static str = "MyKeyboard";
    const SERIAL_NUMBER: &'static str = "1";
}
```

# Keyboard Matrix

In the [templates](https://github.com/Univa/rumcake-templates), you will see that
to implement a keyboard matrix, you need to implement the `KeyboardMatrix` trait
using the `build_matrix!` macro:

```rust ins={13-20}
use rumcake::keyboard;

#[keyboard(usb)]
pub struct MyKeyboard;

use rumcake::keyboard::Keyboard;
impl Keyboard for MyKeyboard {
    const MANUFACTURER: &'static str = "Me";
    const PRODUCT: &'static str = "MyKeyboard";
    const SERIAL_NUMBER: &'static str = "1";
}

use rumcake::keyboard::KeyboardMatrix;
use rumcake::build_matrix;
impl KeyboardMatrix for MyKeyboard {
    build_matrix! {
        { PB2 PB10 PB11 PA3 } // Rows
        { PB12 PB1 PB0 PA7 PA6 PA5 PA4 PA2 PB3 PB4 PA15 PB5 } // Columns
    }
}
```

Rows are defined first, followed by the columns. Row and columns are enumerated left-to-right, starting
from 0. In this example, `PB2` is row 0 and `PA3` is row 3.

The identifiers used for the matrix pins must match the identifiers used by the respective
HAL (hardware abstraction library) for your MCU. The linked sites below have a dropdown menu at
the top to allow you to select a chip. Choose your chip to see what pins are available:

- For nRF-based keyboards, [embassy-nrf](https://docs.embassy.dev/embassy-nrf/git/nrf52840/gpio/trait.Pin.html#implementors)
- For STM32-based keyboards, [embassy-stm32](https://docs.embassy.dev/embassy-stm32/git/stm32f072cb/gpio/trait.Pin.html#implementors)

After defining your matrix, you can set up your [keyboard layout](#keyboard-layout). If you have
a duplex matrix, consider [checking that section](#duplex-matrix) before setting up your keyboard layout.

# Keyboard Layout

To implement a keyboard layout, you must implement the `KeyboardLayout` trait.
It's recommended to use rumcake's `build_layout!` macro, which is simply a wrapper around `keyberon`'s [`layout!` macro](https://github.com/TeXitoi/keyberon/blob/a423de29a9cf0e9e4d3bdddc6958657662c46e01/src/layout.rs#L5).

Please follow `keyberon`'s macro instructions there to set up your keyboard layout.

The following example shows a 3-layer keyboard layout, meant to be used with the matrix we defined previously:

```rust ins={22-44}
use rumcake::keyboard;

#[keyboard(usb)]
pub struct MyKeyboard;

use rumcake::keyboard::Keyboard;
impl Keyboard for MyKeyboard {
    const MANUFACTURER: &'static str = "Me";
    const PRODUCT: &'static str = "MyKeyboard";
    const SERIAL_NUMBER: &'static str = "1";
}

use rumcake::keyboard::KeyboardMatrix;
impl KeyboardMatrix for MyKeyboard {
    build_matrix! {
        { PB2 PB10 PB11 PA3 } // Rows
        { PB12 PB1 PB0 PA7 PA6 PA5 PA4 PA2 PB3 PB4 PA15 PB5 } // Columns
    }
}


use rumcake::keyboard::KeyboardLayout;
impl KeyboardLayout for MyKeyboard {
    build_layout! {
        {
            [ Tab    Q  W  E   R      T    Y      U     I   O  P  '['  ]
            [ LCtrl  A  S  D   F      G    H      J     K   L  ;  '\'' ]
            [ Escape Z  X  C   V      B    N      M     ,   .  /  ']'  ]
            [ No     No No (1) LShift LAlt BSpace Space (2) No No No   ]
        }
        {
            [ LGui F1 F2 F3 F4 F5 F6      F7     F8   F9    F10 F11 ]
            [ t    t  t  t  t  t  Left    Down   Up   Right t   t   ]
            [ t    t  t  t  t  t  Home    PgDown PgUp End   t   F12 ]
            [ t    t  t  t  t  t  PScreen Enter  t    t     t   t   ]
        }
        {
            [ t   1 2 3 4 5      6 7 8 9 0    '(' ]
            [ t   t t t t t      - = t t t    t   ]
            [ '`' t t t t t      t t t t '\\' ')' ]
            [ t   t t t t Delete t t t t t    t   ]
        }
    }
}
```

Congratulations! You have implemented a basic keyboard. You can now move onto building
and flashing your firmware, or try implementing additional features in the "Features" sidebar.

# Duplex matrix

Some keyboards use a "duplex matrix" to save MCU pins. This is usually accomplished
by making an electrical column span two physical columns, and by using two electrical
rows per physical row.

Here's an example portion of a duplex matrix:

![image](https://github.com/Univa/rumcake/assets/41708691/96d35331-ee9d-4be0-990c-64aaed083c3d)

As you can imagine, this would be hard to track in your firmware code.

So, `rumcake` includes a `remap_matrix` macro to help "re-visualize" your matrix to look
more like your physical layout. It creates a `remap` macro for you to use in parts of
the code that require you to configure something that looks like your matrix.

This can be useful for your keyboard layout config, or your backlight matrix config:

```rust del={50-63} ins={1-26,64-75}
// This creates a `remap!` macro that you can use in other parts of your config.
remap_matrix! {
    // This has the same number of rows and columns that you specified in `build_matrix!`
    // Note that `#No` is used to denote an unused matrix position.
    {
        [ K00 K01 K02 K03 K04 K05 K06 K07 ]
        [ K08 K09 K10 K11 K12 K13 K14 #No ]
        [ K15 K16 K17 K18 K19 K20 K21 K22 ]
        [ K23 K24 K25 K26 K27 K28 K29 #No ]
        [ K30 K31 K32 K33 K34 K35 K36 K37 ]
        [ K38 K39 K40 K41 K42 K43 K44 #No ]
        [ K45 K46 K47 K48 K49 K50 K51 K52 ]
        [ K53 K54 K55 K56 K57 K58 K59 #No ]
        [ K60 K61 K62 K63 K64 K65 K66 K67 ]
        [ #No #No #No #No #No K68 K69 #No ]
    }

    // This can be whatever you want it to be. Make it look like your physical layout!
    {
        [ K00 K08 K01 K09 K02 K10 K03 K11 K04 K12 K05 K13 K06 K14 K07 K22 ]
        [ K15 K23 K16 K24 K17 K25 K18 K26 K19 K27 K20 K28 K21 K29 K37     ]
        [ K30 K38 K31 K39 K32 K40 K33 K41 K34 K42 K35 K43 K36 K44 K52     ]
        [ K45 K53 K46 K54 K47 K55 K48 K56 K49 K57 K50 K58 K51 K59 K67     ]
        [             K60 K61     K62 K63     K64 K65 K68 K66 K69         ]
    }
}

use rumcake::keyboard;

#[keyboard(usb)]
pub struct MyKeyboard;

use rumcake::keyboard::Keyboard;
impl Keyboard for MyKeyboard {
    const MANUFACTURER: &'static str = "Me";
    const PRODUCT: &'static str = "MyKeyboard";
    const SERIAL_NUMBER: &'static str = "1";
}

use rumcake::keyboard::KeyboardMatrix;
impl KeyboardMatrix for MyKeyboard {
    build_matrix! {
        { PB3 PB4 PA15 PB5 PA0 PA1 PB2 PB10 PB11 PA3 } // Rows
        { PB12 PB1 PB0 PA7 PA6 PA5 PA4 PA2 } // Columns
    }
}

use rumcake::keyboard::KeyboardLayout;
impl KeyboardLayout for MyKeyboard {
    build_layout! { // without remap!
        {
            [ Escape 2    4     6     8    0    =      Delete ]
            [ 1      3    5     7     9    -    '\\'   No     ]
            [ Tab    W    R     Y     I    P    ']'    Home   ]
            [ Q      E    T     U     O    '['  BSpace No     ]
            [ LCtrl  S    F     H     K    ;    No     PgUp   ]
            [ A      D    G     J     L    '\'' Enter  No     ]
            [ LShift Z    C     B     M    .    Up     PgDown ]
            [ No     X    V     N     ,    /    No     No     ]
            [ LGui   LAlt Space Space RAlt No   Down   End    ]
            [ No     No   No    No    No   Left Right  No     ]
        }
    }
    // Use the `remap!` to create the keyboard layout
    remap! {
        build_layout! {
            {
                [ Escape 1    2     3     4    5  6    7    8     9 0 -    =   '\\'   Delete Home ]
                [ Tab    Q    W     E     R    T  Y    U    I     O P '['  ']' BSpace PgUp   ]
                [ LCtrl  A    S     D     F    G  H    J    K     L ; '\'' No  Enter  PgDown ]
                [ LShift No   Z     X     C    V  B    N    M     , . /    Up  No     End    ]
                [ LGui   LAlt Space Space RAlt No Left Down Right ]
            }
        }
    }
}
```
