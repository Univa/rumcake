# USB host communication

<!--toc:start-->
  - [Setup](#setup)
    - [Required Cargo features](#required-cargo-features)
    - [Required code](#required-code)
<!--toc:end-->

## Setup

### Required Cargo features

You must enable the following `rumcake` features:

- `usb`

### Required code

To set up your keyboard for USB host communication, your keyboard must implement the
`USBKeyboard` trait, and add `usb` to your `keyboard` macro invocation:

```rust
use rumcake::keyboard;

#[keyboard(usb)]
struct MyKeyboard;

// USB configuration
use rumcake::usb::USBKeyboard;
impl USBKeyboard for MyKeyboard {
    const USB_VID: u16 = 0x0000;
    const USB_PID: u16 = 0x0000;
}
```
