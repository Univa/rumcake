---
title: Via 与 Vial
description: 如何配置您的键盘以支持 Via 和 Vial 应用程序。
---

:::caution
这项功能仍在开发中。有关仍需要的功能列表，请参见实现，请检查[待办事项列表](#待办事项列表)。
:::

[Via](https://www.caniusevia.com/) 提供了使用 [Via 应用程序](https://usevia.app/) 重新配置您的键盘的能力，而不是构建和重新刷新您的固件。Via 通过读取特定于您的键盘的 JSON 定义来工作。为了 Via 应用程序支持您的键盘，这些定义需要提交到 [存储库](https://github.com/the-via/keyboards)，或者通过 Via 的设计选项卡进行侧载。

[Vial](https://get.vial.today/) 是一种替代方案，旨在去中心化。为了支持您的键盘，JSON 定义内置到固件中，并且 Vial 应用程序将在运行时加载 JSON 数据。这消除了将 JSON 定义上传到中央存储库的需要。

`rumcake` 提供了使用 Via 或 Vial 的选项。

目前，`rumcake` 实现了：

- Via 协议版本 12，需要 Via V3 定义。
- Vial 协议版本 6，基于 Via V2。

# 设置

## 必须的 Cargo 特性

您必须启用以下 `rumcake` 功能：

- `via` 或 `vial`（取决于您想要使用哪个）
- `storage`（可选项，如果您希望保存在 Via/Vial 应用程序中所做的更改）

## 必要的代码

要设置 Via 和 Vial 支持，您必须添加一个新类型来实现 `ViaKeyboard` 特性。然后，您可以在您的 `keyboard` 宏调用中添加 `via(id = <type>)`。

某些 Via 功能还需要手动配置：

- 对于宏，您需要实现 [`DYNAMIC_KEYMAP_MACRO_BUFFER_SIZE`](/rumcake/api/nrf52840/rumcake/via/trait.ViaKeyboard.html#associatedconstant.DYNAMIC_KEYMAP_MACRO_EEPROM_SIZE) 和 [`DYNAMIC_KEYMAP_MACRO_COUNT`](/rumcake/api/nrf52840/rumcake/via/trait.ViaKeyboard.html#associatedconstant.DYNAMIC_KEYMAP_MACRO_COUNT)
  - 这可以通过 `setup_macro_buffer` 宏轻松完成
- 如果您正在使用某种形式的背光（`simple-backlight`、`simple-backlight-matrix` 或 `rgb-backlight-matrix`），您需要更改 [`BACKLIGHT_TYPE`](https://univa.github.io/rumcake/api/nrf52840/rumcake/via/trait.ViaKeyboard.html#associatedconstant.BACKLIGHT_TYPE)。
  这控制了 `QK_BACKLIGHT` 按键码如何转换为 `keyberon` 动作。换句话说，它控制了 Via 应用程序中以 `BL_` 为前缀的按键码的行为。

对于其他可配置的 Via 选项，请参阅 [`ViaKeyboard` 特性](/rumcake/api/nrf52840/rumcake/via/trait.ViaKeyboard.html)。

```rust ins={5-7,16-26}
use rumcake::keyboard;

#[keyboard(
    // 在键盘宏调用的某处...
    via(
        id = MyKeyboardVia
    )
)]
struct MyKeyboard;

impl KeyboardLayout for MyKeyboard {
    /* ... */
}

// Via 设置
use rumcake::via::{BacklightType, setup_macro_buffer, ViaKeyboard};
struct MyKeyboardVia;
impl ViaKeyboard for MyKeyboardVia {
    type Layout = MyKeyboard; // 必须是实现 `KeyboardLayout` 的类型

    // 可选的，此示例假设您正在使用 simple-backlight-matrix。
    const BACKLIGHT_TYPE: Option<BacklightType> = Some(BacklightType::SimpleBacklightMatrix)

    // 可选的，如果您想要使用 Via 应用程序创建宏，请包括此内容。
    setup_macro_buffer!(buffer_size: 512, macro_count: 16) // 宏所占用的最大字节数，然后是可以创建的最大宏数。
}

```

:::caution
默认情况下，在 Via 应用程序中对键盘所做的更改（例如更改布局、灯光设置等）**不会**被默认保存。

可选地，您可以添加 `use_storage` 和一个 `storage` 驱动程序来保存 Via 数据。

另外，您需要在您的 `ViaKeyboard` 实现中调用 `connect_storage_service`。

```rust del={5} ins={6-10,18}
use rumcake::keyboard;

#[keyboard(
    // 在键盘宏调用的某处...
    via,
    via(
        id = MyKeyboardVia,
        use_storage // 如果您想要保存 Via 配置，则为可选项
    ),
    storage(driver = "internal") // 如果您指定了 `use_storage`，则需要指定存储驱动程序。有关更多信息，请参阅 feature-storage.md。
)]
struct MyKeyboard;

//...
use rumcake::via::connect_storage_service;
impl ViaKeyboard for MyKeyboardVia {
    //...
    connect_storage_service!(MyKeyboard)
}
``` 

您还需要对所选的存储驱动程序进行额外的设置。
有关更多信息，请参阅 [存储功能](../feature-storage/) 的文档。
:::

如果您正在使用 Vial，除了实现 `ViaKeyboard` 外，还必须实现 `VialKeyboard`。
在您的 `keyboard` 宏调用中，您应该使用 `vial`，而不是 `via`。

以下代码示例展示了如何实现 `VialKeyboard` 特性，并使用构建脚本实现 `KEYBOARD_DEFINITION`。请按照 [Vial 定义](#compiling-vial-definitions) 部分的说明操作。

对于其他可配置的 Vial 选项，请参阅 [`VialKeyboard` 特性](/rumcake/api/nrf52840/rumcake/vial/trait.VialKeyboard.html)。

```rust del={7} ins={1-3,10,30-35}
// GENERATED_KEYBOARD_DEFINITION 来自 _generated.rs，由 build.rs 脚本生成。
#[cfg(vial)]
include!(concat!(env!("OUT_DIR"), "/_generated.rs"));

#[keyboard(
    // 在键盘宏调用的某处...
    via(
        id = MyKeyboardVia
    )
    vial(
        id = MyKeyboardVia
    )
)]
struct MyKeyboard;

// ...

// Via 设置
use rumcake::via::{setup_macro_buffer, ViaKeyboard};
impl ViaKeyboard for MyKeyboard {
    type Layout = MyKeyboard; // 必须是实现 `KeyboardLayout` 的类型

    // 可选的，此示例假设您正在使用 simple-backlight-matrix。
    const BACKLIGHT_TYPE: Option<BacklightType> = Some(BacklightType::SimpleBacklightMatrix)

    // 可选的，如果您想要使用 Via 应用程序创建宏，请包括此内容。
    setup_macro_buffer!(buffer_size: 512, macro_count: 16) // 宏所占用的最大字节数，然后是可以创建的最大宏数。
}

use rumcake::vial::VialKeyboard;
impl VialKeyboard for MyKeyboard {
    const VIAL_KEYBOARD_UID: [u8; 8] = [0; 8]; // 更改此处
    const VIAL_UNLOCK_COMBO: &'static [(u8, u8)] = &[(0, 1), (0, 0)]; // 用于解锁 VIAL 的矩阵位置 (行, 列)，可以设置为您想要的任何值
    const KEYBOARD_DEFINITION: &'static [u8] = &GENERATED_KEYBOARD_DEFINITION;
}
```

:::caution
与之前的警告类似，您需要指定 `use_storage`，以保存 Vial 数据。
`connect_storage_service!` 仍然在 `ViaKeyboard` 内部实现：

```rust del={5} ins={6-10,18}
use rumcake::keyboard;

#[keyboard(
    // 在键盘宏调用的某处...
    vial,
    vial(
        id = MyKeyboardVia,
        use_storage // 如果您想要保存 Vial 配置，则为可选项
    ),
    storage = "internal" // 如果您指定了 `use_storage`，则需要指定存储驱动程序。有关更多信息，请参阅 feature-storage.md。
)]
struct MyKeyboard;

//...
use rumcake::via::connect_storage_service;
impl ViaKeyboard for MyKeyboardVia {
    //...
    connect_storage_service!(MyKeyboard)
}
```

:::

## 编译 Vial 定义

要将您的 Vial 定义编译到固件中，您必须将您的 JSON 定义文件进行缩小和 LZMA 压缩，并将原始字节传递给 `VialKeyboard` 特性实现中的 `KEYBOARD_DEFINITION`。

[基本模板](https://github.com/Univa/rumcake-templates/tree/main/rumcake-basic-template) 展示了您如何使用构建脚本 (`build.rs`) 实现此目的。
构建脚本执行以下操作：

- 构建脚本将搜索并打开您键盘的 JSON 定义文件。将其放置在 `./src/definition.json`。
- 构建脚本将检查 `vial` 特性标志，然后对 JSON 数据进行缩小和 LZMA 压缩。
- 结果字节被写入 `GENERATED_KEYBOARD_DEFINITION` 常量中。

`GENERATED_KEYBOARD_DEFINITION` 常量可以在您的 `VialKeyboard` 特性实现中用于 `KEYBOARD_DEFINITION`。
请查看先前显示的代码示例，了解如何使用此常量。

## 推荐的 Via V3 自定义 UI 定义

如果您正在使用常规的 Via（非 Vial），建议使用下面提供的自定义 UI 菜单与 `rumcake` 的额外功能进行交互。请随意选择您需要的菜单。

要添加所需的菜单，请直接将 JSON 添加到您键盘定义文件中的 `"menus"` 字段中。

:::note
尽管 Via V3 提供了[内置的 `qmk_*` 菜单](https://www.caniusevia.com/docs/built_in_menus)来使用灯光功能，但 `rumcake` 的灯光系统并不设计为与这些菜单兼容。
这是因为 `rumcake` 处理效果 ID、灯光速度、启用/禁用等方面的微妙差异，因此，如果您使用常规的 Via，则更倾向于使用下面的自定义 UI。如果您使用 Vial，`rumcake` 将尝试支持 Via/Vial 应用程序的内置灯光菜单。
:::

:::note
未提供 RGB 矩阵的菜单。RGB 背光动画仍需实现。
:::

### Underglow 菜单

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

### Simple Backlight 菜单

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

### Simple Backlight Matrix 菜单

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

# 按键码支持

`rumcake` 并不支持 Via/Vial 应用程序中显示的所有按键码。目前，以下按键码是可用的：

- 基本按键码（在 Via/Vial 的基本选项卡中，或在 HID 键盘报告中可用的按键码）
- 灯光按键码，除了 `QK_BACKLIGHT_TOGGLE_BREATHING`。RGB 按键码仅适用于底部照明，而不适用于 RGB 背光矩阵。
- 瞬时层（`MO(x)`）
- 默认层（`DF(x)`）
- 切换层（`TG(x)`）
- 单次触发层（`OSL(x)`）
- 宏按键码（`M0`、`M1`、`M2` ...）
- 自定义按键码（在您的 JSON 定义中的 `customKeycodes`）
- 某些媒体按键码。必须手动启用此功能。请查看 ["媒体按键" 文档](../feature-media-keys/)
- QK_OUTPUT_BLUETOOTH 和 QK_OUTPUT_USB

您可以假设上述未列出的任何按键码都不受支持。

尝试使用不支持的按键码将不会对您的布局产生任何更改。它可能会在 Via 应用程序中短暂显示，但重新加载应用程序将会将按键码恢复到其先前的状态。

有关这些按键码如何转换为 `keyberon` 动作的更多信息，请参阅 [rumcake/src/via/protocol_12/keycodes.rs](https://github.com/Univa/rumcake/blob/4a7dfb8f9b04c321a43c35bc0d96fbc6afaabad2/rumcake/src/via/protocol_12/keycodes.rs#L1082)。

## 限制

对于支持的按键码，仍然存在一些限制。通常，这些限制是由于内存限制，同时仍然允许 `keyberon` 的动作提供的灵活性。

建议将任何非基本动作直接编译到固件中，而不是通过 Via 分配它们。

- 通过 Via 分配的单次触发层按键码仅适用于层 0 到 15。
  - 已经编译到您的 `keyberon` 布局中的单次触发动作仍然可以在更大的层编号下工作。
- 编译到您的 `keyberon` 布局中的序列动作不会显示在 Via 应用程序中，它将显示为 `0xFFFF`。
- 对于 Vial，使用延迟事件和在宏中使用非基本按键码（大于 0x00FF）的轻击/按/释放事件将无效。在执行事件时使用它们将中止宏。
- 要使背光按键码起作用，您需要修改您的 `ViaKeyboard` 实现中的 `BACKLIGHT_TYPE` 常量。这定义了如何转换背光按键码。
- RGB 按键码仅适用于底部照明，而不适用于 RGB 背光矩阵。

# 待办事项列表

- [ ] 轻击切换，单次触发修改按键码（以及“层”子菜单中的其他按键码）
- [ ] QMK 设置（Vial）
- [ ] 动态按键映射轻击舞蹈，组合，按键覆盖（Vial）
- [ ] Vial 宏支持（延迟和非基本按键码）
