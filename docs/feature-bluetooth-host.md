# Bluetooth host communication

<!--toc:start-->

- [Setup](#setup)
  - [Required Cargo features](#required-cargo-features)
  - [Required code](#required-code)
- [USB host communication interoperability](#usb-host-communication-interoperability)
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

To set up your keyboard for bluetooth host communication, your keyboard must implement the `NRFBluetoothKeyboard` trait:

```rust
use rumcake::keyboard;

#[keyboard]
struct MyKeyboard;

// Bluetooth configuration
use rumcake::nrf_ble::NRFBluetoothKeyboard;
impl NRFBluetoothKeyboard for MyKeyboard {
    const BLE_VID: u16 = 0x0000; // Change this
    const BLE_PID: u16 = 0x0000; // Change this
}
```

## USB host communication interoperability

By default, your keyboard will use bluetooth to communicate with your device.
You can use the `ToggleUSB` keycode to switch to USB and back.

## Keycodes

In your keyberon layout, you can use any of the enum members defined in `BluetoothCommand`:

```rust
ToggleUSB // Only available if the `usb` feature flag is also enabled. Allows you to switch between USB and bluetooth host communication. Useful if you swap between a USB and a bluetooth host.
```

## To-do List

- [ ] Multiple bluetooth profiles
- [ ] LE Secure Connections (I believe this requires `nrf-softdevice` changes)
