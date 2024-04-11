---
title: 存储
description: 如何配置键盘以在板载上存储设置。
---

:::caution
这项功能仍在开发中。有关仍需要的功能列表，请参见实现，请检查[待办事项列表](#待办事项列表)。
:::

存储提供了一种机制，使您的设备能够保存数据，在电源循环之间保持数据的持久性。这使您能够根据自己的喜好配置键盘，而不会在重新启动之间丢失您所做的任何更改。

## 设置

### 所需 Cargo 功能

你必须启用以下 `rumcake` 功能：

- `storage`

目前，以下功能可以使用存储：

- `backlight`（用于存储背光色调、饱和度、亮度、速度、效果等）
- `underglow`（用于存储底部灯光色调、饱和度、亮度、速度、效果等）
- `via`/`vial`（用于存储动态按键映射）

请查看它们各自的文档，了解如何为这些功能启用存储使用。通常，任何能够将数据存储到存储外设的功能都需要在 `#[keyboard]` 宏调用中明确指定 `use_storage`。

如果你的 `#[keyboard]` 宏调用中没有在任何地方指定 `use_storage`，则无需设置存储驱动。

### 使用 MCU 闪存作为存储的必要代码

接下来的说明适用于**如果你想使用选定 MCU 上的现有闪存空间作为存储**。

要设置存储，你必须在 `memory.x` 文件中添加一个 `CONFIG` 部分，并使用 `__config_start` 和 `__config_end` 指定 `CONFIG` 部分的起始和结束地址。这将涉及从 `FLASH` 部分中获取一些空间，因此请确保你仍有足够的空间来刷写编译后的固件二进制文件。

如果不确定要为存储分配多少空间，请参阅[此部分](#存储空间注意事项)。

以下示例显示了 `STM32F303CBx` 芯片的 `memory.x` 可能是什么样子：

```
MEMORY
{
    FLASH : ORIGIN = 0x08000000, LENGTH =  120K /* 减少为芯片的最大 128K 中的 8K，用于下面 CONFIG 部分 */
    CONFIG: ORIGIN = ORIGIN(FLASH) + LENGTH(FLASH), LENGTH = 8K /* 添加此行 */
    RAM   : ORIGIN = 0x20000000, LENGTH =   32K
}

__config_start = ORIGIN(CONFIG) - ORIGIN(FLASH); /* 添加此行 */
__config_end = __config_start + LENGTH(CONFIG); /* 添加此行 */
```

**`CONFIG` 部分的要求：**

- 大小必须是闪存外设的“擦除大小”的倍数。有时这也称为“页面大小”或“区域大小”。
  - 在上面的示例中，STM32F303CBx 的擦除大小为 2KiB。因此，`CONFIG` 部分的大小为 4 页。
- 起始地址（`__config_start`）必须对齐到擦除大小的倍数。
- `__config_start` 和 `__config_end` 的值必须**相对于 FLASH 部分的起始地址**。
  - 请注意，在上面的示例中，我们为此目的减去了 `ORIGIN(FLASH)`。

最后，你可以将 `storage(driver = "internal")` 添加到你的 `#[keyboard]` 宏调用中。

```rust ins={5,7-13}
#[keyboard(
    // 你的键盘宏调用中的某处...
    underglow(
        driver_setup_fn = my_underglow_setup,
        use_storage // 此底部灯光功能使用存储
    ),
    storage(
        driver = "internal",
        // 下面的 `flash_size` 仅适用于 RP2040。如果你不使用 RP2040，则省略。
        // 应该等于闪存芯片的总大小（不是你的 CONFIG 分区的大小）
        flash_size = 2097152,
        // RP2040 上用于处理闪存操作的 `dma` 通道。如果不使用 RP2040，则省略。
        dma = DMA_CH0
    )
)]
struct MyKeyboard;
```

:::note
**对于 RP2040 用户**：`#[keyboard]` 宏调用还必须包括 `flash_size` 和上面示例中显示的 `dma` 通道。如果你不使用 RP2040，则这些内容可以省略。
:::

:::tip
默认情况下，`StorageDevice` 特性中的 `setup_storage_buffer()` 函数创建一个大小为 1024 字节的缓冲区。你可以覆盖实现，将缓冲区的大小增加到存储可能更大的值，或者你可以减小大小以节省内存。这可以通过在你的宏调用中添加 `buffer_size` 来实现：

```rust ins={10}
#[keyboard(
    // 你的键盘宏调用中的某处...
    storage(
        driver = "internal",
        // 下面的 `flash_size` 仅适用于 RP2040。如果你不使用 RP2040，则省略。
        // 应该等于闪存芯片的总大小（不是你的 CONFIG 分区的大小）
        flash_size = 2097152,
        // RP2040 上用于处理闪存操作的 `dma` 通道。如果不使用 RP2040，则省略。
        dma = DMA_CH0,
        buffer_size = 512
    )
)]
struct MyKeyboard;
```

请注意，该缓冲区的大小必须足够大，以存储你从存储外设中读取或写入的可能最大值。
:::

## 存储空间注意事项

为存储分配的空间量取决于你的键盘使用了哪些功能。

在底层，TicKV 用作文件系统来存储数据。有关更多信息，请参阅他们的[规范文档](https://github.com/tock/tock/blob/master/libraries/tickv/SPEC.md)。你可能希望考虑分配多个页面以提高闪存的寿命（即使你可能不一定需要所有空间）。

# 待办事项列表

- [ ] QSPI 驱动