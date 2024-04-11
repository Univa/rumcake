---
title: 编码器
description: 如何将 EC11 兼容编码器添加到您的设备。
---

:::caution
此功能仍在开发中。有关尚需实现的功能列表，请查看[待办事项列表](#待办事项列表)。
:::

本文档包含有关如何将 EC11 兼容编码器添加到您的设备的信息。

# 设置

## 需要的代码

要为您的键盘设置编码器，您必须在 `#[keyboard]` 宏调用中添加 `encoders`，并且您的键盘必须实现 `DeviceWithEncoders` 特性。

您可以通过使用 `setup_encoders!` 宏轻松完成此操作：

```rust ins={5,9-31}
use rumcake::keyboard;

#[keyboard(
    // 在键盘宏调用中的某处 ...
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

`sw_pin` 对应于连接到编码器按钮的引脚。 `output_a_pin` 和 `output_b_pin` 对应于随着编码器旋转而脉冲的引脚。

:::note
编码器的当前实现依赖于中断以避免不断轮询编码器。

对于 STM32，这意味着您需要为 `sw_pin` 和 `output_a_pin` 指定 EXTI 通道。这可以通过向 `input_pin!` 宏添加额外的参数来完成，如上例所示。对于其他平台，可以省略此步骤。
:::

编码器通过将其输出映射到布局上的位置来工作。
`type Layout = Self` 告诉 rumcake 将编码器事件重定向到 `MyKeyboard` 实现的 `KeyboardLayout`。

在上面的示例中，以下是以下映射：

- 编码器 1 按钮：`A` 键（或第二层上的 `G`）
- 编码器 1 顺时针旋转：`B` 键（或第二层上的 `H`）
- 编码器 1 逆时针旋转：`C` 键（或第二层上的 `I`）
- 编码器 2 按钮：`D` 键（或第二层上的 `J`）
- 编码器 2 顺时针旋转：`H` 键（或第二层上的 `K`）
- 编码器 2 逆时针旋转：`I` 键（或第二层上的 `L`）

# 待办事项列表

- [ ] Via(l) 支持