---
title: 设备信息，矩阵与布局
description: 如何配置你的键盘矩阵和布局。
sidebar:
  order: 3
---

本文档包含一些有关如何定义键盘基本信息的信息、以及如何为键盘设置基本矩阵和布局设置的信息。

:::note
以下示例针对的是非分体式键盘，它将 `KeyboardMatrix` 和 `KeyboardLayout` 实现放在同一个入口点中。
如果您使用的是分体式键盘，可以继续阅读以了解 `KeyboardMatrix`和 `KeyboardLayout` 特质的实现，但 `impl` 块的位置将取决于取决于您的分体式键盘设置。

有关详细信息，请参阅[拆分键盘文档](../../features/feature-split)。
:::

# 键盘信息

每个设备必须实现 `rumcake` 的基本特质 `Keyboard` 才能使用。在这里，您可以定义一些基本信息，包括键盘的名称、制造商、版本号等。

```rust ins={6-11}
use rumcake::keyboard;

#[keyboard(usb)]
pub struct MyKeyboard;

use rumcake::keyboard::Keyboard;
impl Keyboard for MyKeyboard {
    const MANUFACTURER: &'static str = "Me"; // 制造商名称
    const PRODUCT: &'static str = "MyKeyboard"; // 键盘名称
    const SERIAL_NUMBER: &'static str = "1"; // 版本号
}
```

# 键盘矩阵

在[模板](https://github.com/Univa/rumcake-templates)中，您将看到要实现键盘矩阵，您需要使用一个 `build_<matrix_type>_matrix!` 宏来实现 `KeyboardMatrix` 特质：

```rust ins={13-21}
use rumcake::keyboard;

#[keyboard(usb)]
pub struct MyKeyboard;

use rumcake::keyboard::Keyboard;
impl Keyboard for MyKeyboard {
    const MANUFACTURER: &'static str = "Me";
    const PRODUCT: &'static str = "MyKeyboard";
    const SERIAL_NUMBER: &'static str = "1";
}

use rumcake::keyboard::{build_standard_matrix, KeyboardMatrix};
impl KeyboardMatrix for MyKeyboard {
    type Layout = Self; // 先不用担心这里的错误。一旦你实现了`KeyboardLayout`，它就会被修复

    build_standard_matrix! {
        rows: [ PB2 PB10 PB11 PA3 ],
        cols: [ PB12 PB1 PB0 PA7 PA6 PA5 PA4 PA2 PB3 PB4 PA15 PB5 ]
    }
}
```

如果您看到有关 `Self` 未实现 `KeyboardLayout` 的错误，请不要担心。一旦您按照下一节进行操作，此问题就会得到解决。请注意，此关联类型用于将矩阵事件重定向到已实现的布局。

用于矩阵引脚的标识符必须与 MCU 的相应 HAL（硬件抽象库）使用的标识符相匹配。下面链接的网站顶部有一个下拉菜单，可让您选择芯片。选择您的芯片以查看可用的引脚：

- 对于基于 nRF 的键盘，[embassy-nrf](https://docs.embassy.dev/embassy-nrf/git/nrf52840/gpio/trait.Pin.html#implementors)
- 对于基于 STM32 的键盘，[embassy-stm32](https://docs.embassy.dev/embassy-stm32/git/stm32f072cb/gpio/trait.Pin.html#implementors)

定义矩阵后，您可以设置[键盘布局]((#键盘布局))。如果您有双工矩阵，请考虑在设置键盘布局之前[检查该部分](#双工矩阵)。

:::note
上面的例子假设一个矩阵是一个标准矩阵（开关按行和列连接，带有二极管）。首先定义行，然后定义列。行和列从 0 开始从左到右枚举。在此示例中， `PB2` 是第 0 行， `PA3` 是第 3 行。

对于其他矩阵类型，请参阅[其他矩阵类型](#其他矩阵类型)部分。
:::

# 键盘布局

要实现键盘布局，您必须实现 `KeyboardLayout` 特征。建议使用 rumcake 的 `build_layout!` 宏，它只是 `keyberon` 的 [`layout!`](https://github.com/TeXitoi/keyberon/blob/a423de29a9cf0e9e4d3bdddc6958657662c46e01/src/layout.rs#L5) 宏的包装。

请按照 `keyberon` 的宏说明设置您的键盘布局。

以下示例展示了一个 3 层键盘布局，旨在与我们之前定义的矩阵一起使用：


```rust ins={24-46}
use rumcake::keyboard;

#[keyboard(usb)]
pub struct MyKeyboard;

use rumcake::keyboard::Keyboard;
impl Keyboard for MyKeyboard {
    const MANUFACTURER: &'static str = "Me";
    const PRODUCT: &'static str = "MyKeyboard";
    const SERIAL_NUMBER: &'static str = "1";
}

use rumcake::keyboard::{build_standard_matrix, KeyboardMatrix};
impl KeyboardMatrix for MyKeyboard {
    type Layout = Self;

    build_standard_matrix! {
        rows: [ PB2 PB10 PB11 PA3 ],
        cols: [ PB12 PB1 PB0 PA7 PA6 PA5 PA4 PA2 PB3 PB4 PA15 PB5 ]
    }
}


use rumcake::keyboard::{build_layout, KeyboardLayout};
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

恭喜！您已经实现了一个基本的键盘。您现在可以继续构建和刷写固件，或者尝试在 “功能” 侧栏中实现的其他功能。

# 其他矩阵类型

## 直接引脚矩阵（无二极管矩阵）

如果您的 MCU 引脚直接连接到开关（而不是引脚连接到一行/列开关），那么您可以使用 `build_direct_pin_matrix!` 宏来代替。


```rust ins={3-11}
// 其余的配置...

use rumcake::keyboard::{build_direct_pin_matrix, KeyboardMatrix};
impl KeyboardMatrix for MyKeyboard {
    type Layout = Self;

    build_direct_pin_matrix! {
        [ PB2  PB10 PB11 PA3 ]
        [ PB12 PB1  PB0  No  ]
    }
}

use rumcake::keyboard::{build_layout, KeyboardLayout};
impl KeyboardLayout for MyKeyboard {
    build_layout! {
        {
            [ Tab    Q  W  E ]
            [ LCtrl  A  S  D ]
        }
        {
            [ LGui F1 F2 F3 ]
            [ t    t  t  t  ]
        }
    }
}
```

每个引脚将直接映射到一个（行，列）位置，这决定了它对应的布局中的键。每行必须具有相同的列数。如果有未使用的矩阵位置，可以使用 `No` 忽略它们。

在此示例中，连接到 `PB10` 的开关映射到第 0 行、第 1 列。基于 `KeyboardLayout` 的实现，该开关将对应于 `Q`/`F1` 键。

## 模拟矩阵

:::caution
模拟矩阵仍在开发中，可能并不完全稳定。
:::

如果您的开关由模数转换外设供电（例如，霍尔效应开关通常就是这种情况），那么您可以使用 `build_analog_matrix!` 宏。此外，您还需要使用 `setup_adc_sampler!` 宏指定 ADC 采样器配置。


```rust ins={4-15,17-31}
// 其他配置...

// 创建一个 ADC 采样器，其中 MCU 的引脚连接到多路复用器，或者直接连接到模拟源
setup_adc_sampler! {
    (interrupt: ADC1_2, adc: ADC2) => {
        Multiplexer {
            pin: PA2, // 连接到多路复用器的 MCU 模拟引脚
            select_pins: [ PA3 No PA4 ] // 连接到多路复用器上的选择引脚的引脚
        },
        Direct {
            pin: PA5 // MCU 模拟引脚直接连接到模拟源
        },
    }
}

use rumcake::keyboard::{build_analog_matrix, KeyboardMatrix};
impl KeyboardMatrix for MyKeyboard {
    type Layout = Self;

    build_analog_matrix! {
        channels: {
            [ (1,0) (0,1) (0,4) (0,5) ]
            [ (0,0) No    No    No    ]
        },
        ranges: {
            [ 3040..4080 3040..4080 3040..4080 3040..4080 ]
            [ 3040..4080 No         No         No         ]
        }
    }
}
```

首先，提供 ADC 采样器定义。在此示例中， `ADC2` 外设（由 `ADC1_2` 中断控制）连接到两个引脚。引脚 `PA2` 连接到多路复用器，引脚 `PA5` 直接连接到模拟源（本例中为开关）。

对于 `PA2` ，多路复用器输出选择由 `PA3` 和 `PA4` 控制。第二个选择引脚未使用，因此用 `No` 表示。引脚按最低有效位在前排列。因此，如果 `PA4` 为高电平且 `PA2` 为低电平，则选择多路复用器输出 `4` 。

:::note
`setup_adc_sampler!` 中的所有多路复用器定义必须具有相同数量的选择引脚。如果您的多路复用器具有不同数量的选择引脚，则可以使用 `No` 填充较小的多路复用器，直到定义具有相同数量的选择引脚。
:::

:::note
请注意，`setup_adc_sampler!` 宏的参数将取决于您构建的平台。检查 API 参考以获取需要调用的特定参数 `setup_adc_sampler!`
:::

`build_analog_matrix!` 提供的矩阵有两个用途：

- 定义从矩阵位置（行、列）到模拟引脚索引和多路复用器输出（如果适用）的映射。
- 定义模拟源可以从 ADC 过程生成的可能值范围。

当我们查看矩阵的第 0 行、第 0 列时，我们发现：

- 它对应于 ADC 引脚 `0` （连接到多路复用器 `PA2` ）和多路复用器输出 `0` （当选择引脚 `PA3` 和 `PA4` 设置为低）。
- ADC 预计会产生范围从 `3040` 到 `4080` 的值。

对于矩阵的第 1 行第 0 列，我们发现：

- 它对应于 ADC 引脚 `1` （通过 `PA5` 直接连接到模拟源）。 `(1,0)` 中的 `0` 被忽略，因为它没有连接到多路复用器。
- ADC 预计会产生范围从 `3040` 到 `4080` 的值。

请注意，未使用的矩阵位置由 `No` 表示。

# 重新可视化矩阵（例如双工矩阵）

有时，您的键盘可能具有复杂的矩阵方案，这可能会导致您难以阅读部分配置。

例如，某些键盘使用“双工矩阵”来节省 MCU 引脚。这通常是通过使电气列跨越两个物理列并通过每个物理行使用两个电气行来实现的。

这是双工矩阵的示例部分：

![image](https://github.com/Univa/rumcake/assets/41708691/96d35331-ee9d-4be0-990c-64aaed083c3d)

正如您可以想象的那样，这将很难在您的固件代码中进行跟踪。

因此，`rumcake` 包含一个 `remap_matrix` 宏来帮助“重新可视化”矩阵，使其看起来更具可读性。它创建一个 `remap` 宏供您在需要您配置类似于矩阵的代码部分中使用。

这对于您的键盘布局配置或背光矩阵配置很有用：


```rust del={52-65} ins={1-26,66-77}
// 这将创建一个 `remap!` 宏，您可以在配置的其他部分使用它。
remap_matrix! {
    // 它的行数和列数与您在矩阵中指定的相同。
    // 请注意，`No` 用于表示未使用的矩阵位置。
    original: {
        [ K00 K01 K02 K03 K04 K05 K06 K07 ]
        [ K08 K09 K10 K11 K12 K13 K14 No  ]
        [ K15 K16 K17 K18 K19 K20 K21 K22 ]
        [ K23 K24 K25 K26 K27 K28 K29 No  ]
        [ K30 K31 K32 K33 K34 K35 K36 K37 ]
        [ K38 K39 K40 K41 K42 K43 K44 No  ]
        [ K45 K46 K47 K48 K49 K50 K51 K52 ]
        [ K53 K54 K55 K56 K57 K58 K59 No  ]
        [ K60 K61 K62 K63 K64 K65 K66 K67 ]
        [ No  No  No  No  No  K68 K69 No  ]
    },

    // 这可以是你想要的任何样子。让它看起来像你的物理布局！
    remapped: {
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

use rumcake::keyboard::{build_standard_matrix, KeyboardMatrix};
impl KeyboardMatrix for MyKeyboard {
    type Layout = Self;

    build_standard_matrix! {
        rows: [ PB3 PB4 PA15 PB5 PA0 PA1 PB2 PB10 PB11 PA3 ],
        cols: [ PB12 PB1 PB0 PA7 PA6 PA5 PA4 PA2 ]
    }
}

use rumcake::keyboard::{build_layout, KeyboardLayout};
impl KeyboardLayout for MyKeyboard {
    build_layout! { // 不使用 remap!
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
    // 使用 `remap!` 创建键盘布局
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
