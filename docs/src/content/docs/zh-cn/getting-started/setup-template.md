---
title: Setup - 选择一个模板
description: 如何使用模板配置你的Cargo工作区。
next:
  label: 矩阵与布局
  link: ../matrix-and-layout/
sidebar:
  order: 1
---

开始使用 `rumcake` 的最简单方法是使用提供的模板之一。

[请在继续之前查阅模板存储库中的 README.md 以了解每个模板。](https://github.com/Univa/rumcake-templates)

# 克隆模板

要从模板开始，您必须安装 [cargo-generate](https://github.com/cargo-generate/cargo-generate#quickstart)，
然后运行以下命令来选择模板：

````bash
cargo generate --git https://github.com/Univa/rumcake-templates
````

## 模板结构

一般来说，`rumcake` 模板提供以下内容：

- `.cargo/config.toml` 文件，用于配置您的 Cargo 运行程序（使用 [`probe-rs`](https://probe.rs/)），指定您的芯片。
- `Cargo.toml` 文件，其中已经包含编译固件所需的依赖项和功能标志。
   - 对于 `rumcake` ，除了其他额外功能外，还将启用与所选芯片相对应的功能标志。
- `src/main.rs` 文件，其中包含部分完成的 `rumcake` 键盘实现。
- `README.md` 文件，其中包含有关如何编译固件并将其刷写到芯片的信息。
- `rust-toolchain.toml` 文件，包含有关将使用的 Rust 工具链的信息
   用于您的 Cargo 工作区，包括芯片和工具链版本的构建目标。
- `memory.x` 文件，由 [`cortex-m-rt`](https://docs.rs/cortex-m-rt/latest/cortex_m_rt/#memoryx) 使用，定义目标的内存布局 芯片。

要了解如何向键盘添加额外功能，请参阅[模板](https://github.com/Univa/rumcake-templates)或参阅侧栏中的“功能”部分。

:::note
有些模板有额外的文件。 例如，`rumcake-basic-template` 有一个构建脚本来生成 Vial 定义。 `rumcake-split-template` 在 `src/` 中有多个入口，左半部分和右半部分各一个，需要单独刷写。

有关上面未列出的任何内容的更多信息，请参阅相应模板的 `README.md` 文件。
:::

# 下一步

完成克隆模板后，请继续设置键盘矩阵和布局。

根据所选模板的不同，在编译和刷写固件之前，您还需要搜索要处理的 `// TODO` 注释。