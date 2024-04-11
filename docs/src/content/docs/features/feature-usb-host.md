---
title: USB
description: How to configure your keyboard to communicate with a device over USB.
---

This document contains information about how to make your keyboard communicate
with a host device over USB.

# Setup

## Required Cargo features

You must enable the following `rumcake` features:

- `usb`

## Required code

To set up your keyboard for USB host communication, your keyboard must implement the
`USBKeyboard` trait, and add `usb` to your `keyboard` macro invocation:

```rust ins={5,9-14}
use rumcake::keyboard;

#[keyboard(
    // somewhere in your keyboard macro invocation ...
    usb
)]
struct MyKeyboard;

// USB configuration
use rumcake::usb::USBKeyboard;
impl USBKeyboard for MyKeyboard {
    const USB_VID: u16 = 0x0000;
    const USB_PID: u16 = 0x0000;
}
```

:::note
This should already be mostly done for you if you are using a template.
If so, make sure to change `USB_VID` and `USB_PID`.
:::
