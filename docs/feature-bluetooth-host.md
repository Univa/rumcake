# Bluetooth host communication

<!--toc:start-->

- [Setup](#setup)
  - [Required Cargo features](#required-cargo-features)
  - [Required code](#required-code)
- [Keycodes](#keycodes)
- [USB host communication interoperability](#usb-host-communication-interoperability)
- [To-do List](#to-do-list)
<!--toc:end-->

## Setup

### Required Cargo features

You must enable the following `rumcake` features:

- `bluetooth`
- `nrf-ble` if you are using an nRF-based keyboard

Since `nrf-softdevice` has its own critical section implementation, **you must disable any other critical section implementation**.
For example, if you used one of the rumcake templates, you may have to remove `critical-section-single-core` from the `cortex-m` dependency:

```toml
# cortex-m = { version = "0.7.6", features = ["critical-section-single-core"] }
cortex-m = { version = "0.7.6" }
```

### Required code

To set up your keyboard for bluetooth host communication, you must add `bluetooth` to your `#[keyboard]` macro invocation, and your keyboard must implement the `BluetoothKeyboard` and `BluetoothDevice` trait:

```rust
use rumcake::keyboard;

#[keyboard(bluetooth)]
struct MyKeyboard;

use rumcake::hw::mcu::BluetoothDevice;
impl BluetoothDevice for WingpairLeft {
    // This addresses can be whatever you want, as long as it is a valid "Random Static" bluetooth addresses.
    // See "Random Static Address" in this link: https://novelbits.io/bluetooth-address-privacy-ble/
    const BLUETOOTH_ADDRESS: [u8; 6] = [0x41, 0x5A, 0xE3, 0x1E, 0x83, 0xE7]; // TODO: Change this
}

// Bluetooth configuration
use rumcake::bluetooth::BluetoothKeyboard;
impl BluetoothKeyboard for MyKeyboard {
    const BLE_VID: u16 = 0x0000; // Change this
    const BLE_PID: u16 = 0x0000; // Change this
}
```

## Keycodes

In your keyberon layout, you can use any of the enum members defined in `BluetoothCommand`:

```rust
ToggleOutput // Only available if the `usb` feature flag is also enabled. More information below.
OutputUSB // Only available if the `usb` feature flag is also enabled. More information below.
OutputBluetooth // Only available if the `usb` feature flag is also enabled. More information below.
```

## USB host communication interoperability

By default, your keyboard will use bluetooth to communicate with your device.
You can use the `ToggleOutput`, `OutputUSB` or `OutputBluetooth` keycode to switch
to USB and back. This won't disconnect your keyboard from your USB or Bluetooth
host, but it will simply determine the device to send keyboard reports to.

## To-do List

- [ ] Multiple bluetooth profiles
- [ ] LE Secure Connections (I believe this requires `nrf-softdevice` changes)
