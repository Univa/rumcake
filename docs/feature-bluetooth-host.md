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

If USB is plugged in, rumcake will automatically switch to USB communication, and disable bluetooth communications.

## To-do List

- [ ] LE Secure Connections (I believe this requires `nrf-softdevice` changes)
