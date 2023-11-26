---
title: Setup - Using a Template
description: How to configure your Cargo workspace using a template.
next:
  label: Matrix and Layout
  link: ../matrix-and-layout
sidebar:
  order: 1
---

The easiest way to get started with `rumcake` is with one of the provided templates.

[Please consult the README.md in the templates repository to learn about each of the templates before continuing.](https://github.com/Univa/rumcake-templates)

# Cloning a template

To start with a template, you must install [cargo-generate](https://github.com/cargo-generate/cargo-generate#quickstart),
then run the following command to choose a template:

```bash
cargo generate --git https://github.com/Univa/rumcake-templates
```

## Template structure

Generally, `rumcake` templates provides the following:

- `.cargo/config.toml` file, which configures your Cargo runner (using [`probe-rs`](https://probe.rs/)), specific to your chip.
- A `Cargo.toml` file, which already contains the dependencies and feature flags needed to compile the firmware.
  - For `rumcake`, a feature flag corresponding to the chosen chip will be enabled, in addition to other extra features.
- A `src/main.rs` file, which contains a partially completed `rumcake` keyboard implementation.
- A `README.md` file with information on how to compile and flash the firmware to your chip.
- A `rust-toolchain.toml` file, containing information about the Rust toolchain that will be used
  for your Cargo workspace, including the build target for your chip and toolchain version.
- A `memory.x` file, used by [`cortex-m-rt`](https://docs.rs/cortex-m-rt/latest/cortex_m_rt/#memoryx), defining the memory layout of your target chip.

To learn how to add extra features to your keyboard, see the [templates](https://github.com/Univa/rumcake-templates)
or refer to the "Features" section in the sidebar.

:::note
Some templates have extra files. For example, the `rumcake-basic-template` has a build script to generate Vial
definitions. The `rumcake-split-template` has multiple entrypoints in `src/`, one each for the left and right half, which need
to be flashed separately.

Consult the corresponding template's `README.md` file for more information about anything that may not be listed above.
:::

# Next steps

After you have finished cloning a template, feel free to continue setting up your keyboard matrix and layout.

Depending on the chosen template, you will also need to search for `// TODO` comments to address before
building and flashing your firmware.
