# Keyboard matrix and layout

<!--toc:start-->

- [Keyboard matrix](#keyboard-matrix)
  - [Matrix setup](#matrix-setup)
  - [Duplex matrix](#duplex-matrix)
- [Keyboard Layout](#keyboard-layout)
<!--toc:end-->

This doc contains some more information on matrix and layout setup for your keyboard.

# Keyboard matrix

## Matrix setup

In the [templates](https://github.com/Univa/rumcake-templates), you will see that
to implement a keyboard matrix, you need to implement the `KeyboardMatrix` trait:

```rust
use rumcake::keyboard;

#[keyboard]
pub struct MyKeyboard;

use rumcake::keyboard::KeyboardMatrix;
impl KeyboardMatrix for MyKeyboard {
    build_matrix! {
        { P0_02 P1_13 P1_11 P0_10 } // Rows here
        { P0_22 P0_24 P1_00 P0_11 P1_04 P1_06 } // Columns here
    }
}
```

The identifiers used for the matrix pins must match the identifiers used by the respective
HAL (hardware abstraction library) for your MCU (select your chip in the links provided):

- For nRF-based keyboards, [embassy-rs](https://docs.embassy.dev/embassy-nrf/git/nrf52840/gpio/trait.Pin.html#implementors)
- For STM32-based keyboards, [embassy-stm32](https://docs.embassy.dev/embassy-stm32/git/stm32f072cb/gpio/trait.Pin.html#implementors)

## Duplex matrix

Some keyboards use a "duplex matrix" to save MCU pins. This is usually accomplished
by making an electrical column span two physical columns, and by using two electrical
row per physical row.

Here's an example portion of a duplex matrix:

![image](https://github.com/Univa/rumcake/assets/41708691/96d35331-ee9d-4be0-990c-64aaed083c3d)

As you can imagine, this would be hard to track in your firmware code.

So, `rumcake` includes a `remap_matrix` macro to help "re-visualize" your matrix to look
more like your physical layout. It creates a `remap` macro for you to use in parts of
the code that require you to configure something that looks like your matrix.

This can be useful for your keyboard layout config, or your backlight matrix config:

```rust
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

/* ... later in your config ... */

use rumcake::keyboard::Keyboard;
impl KeyboardLayout for MyKeyboard {
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

# Keyboard Layout

To implement a keyboard layout, you must implement the `KeyboardLayout` trait. See above for an example.
It's recommended to use rumcake's `build_layout!` macro, which is simply a wrapper around `keyberon`'s [`layout!` macro](https://github.com/TeXitoi/keyberon/blob/a423de29a9cf0e9e4d3bdddc6958657662c46e01/src/layout.rs#L5).

Please follow keyberon's macro instructions there to set up your keyboard layout.

# To-dos

- [ ] Macro to quickly remap Pro Micro-like boards
