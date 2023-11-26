---
title: Split keyboards
description: How to configure a split keyboard system.
---

:::caution
This feature is still a work in progress. For a list of features that still need
to be implemented, check the [to-do list](#to-do-list)
:::

A split keyboard consists of one or more peripheral devices communicating matrix events to
one central device, which then creates HID keyboard reports to send to a host device.

Generally, a split keyboard will require compiling multiple binaries, one for each
device/part of the split keyboard. For example, you will need one binary for
the left half, and another binary for the right half.

Continue reading to see how to implement a "central" and a "peripheral" device using `rumcake`.

# Example

The following documentation will show an example for a split keyboard with a left and right half,
no dongle.

The central device code will be placed in `left.rs`, and the peripheral device code will be
placed in `right.rs`.

For a similar full example of how to implement a split keyboard, check the
[template repo](https://github.com/Univa/rumcake-templates).

# Central setup

The "central" device in a split keyboard setup defines the keyboard layout, communicates with the host device, and receives matrix events from other peripherals. There should only be one central device.
If the split keyboard also uses extra features like backlighting or underglow, the central device will also be responsible for sending their related commands to the peripherals.

Typically, the central device could be a dongle (good for saving battery life), or one of the keyboard halves.

## Required Cargo features for central device

You must compile a binary with the following `rumcake` features:

- `split-central`
- `drivers` (optional built-in drivers to connect your central device to peripheral devices)

## Required code for central device

To set up the central device, you must add `split_central(driver = "<driver>")` to your `#[keyboard]` macro invocation,
and your keyboard must implement the appropriate trait for the driver you're using. For example, with `ble` and an nRF5x
chip selected, you must implement `NRFBLECentralDevice`, and `BluetoothDevice`:

```rust ins={6-8,20-37}
// left.rs
use rumcake::keyboard;

#[keyboard(
    // somewhere in your keyboard macro invocation ...
    split_central(
        driver = "ble" // TODO: change this to your desired split driver, and implement the appropriate trait
    )
)]
struct MyKeyboardLeftHalf;

// KeyboardLayout should already be implemented
use rumcake::keyboard::KeyboardLayout;
impl KeyboardLayout for MyKeyboard {
    // ...
}

// later in your file ...

// Bluetooth device setup
use rumcake::hw::mcu::BluetoothDevice;
impl BluetoothDevice for MyKeyboardLeftHalf {
    // This addresses can be whatever you want, as long as it is a valid "Random Static" bluetooth addresses.
    // See "Random Static Address" in this link: https://novelbits.io/bluetooth-address-privacy-ble/
    const BLUETOOTH_ADDRESS: [u8; 6] = [0x41, 0x5A, 0xE3, 0x1E, 0x83, 0xE7]; // TODO: Change this to something else
}

// Split central setup
use rumcake::split::drivers::nrf_ble::central::NRFBLECentralDevice;
impl NRFBLECentralDevice for MyKeyboardLeftHalf {
    // Must be valid "Random Static" bluetooth addresses.
    // This central device can connect to one other peripheral. Feel free to add more addresses to connect more peripherals.
    const PERIPHERAL_ADDRESSES: &'static [[u8; 6]] = [
        [0x92, 0x32, 0x98, 0xC7, 0xF6, 0xF8],
    ];
}
```

:::note
In case you are using the `ble` driver, if your keyboard also communicates with your host device using Bluetooth
(basically if you followed the [Bluetooth doc](../feature-bluetooth-host) or chose a template with Bluetooth),
then the `BluetoothDevice` trait should already be implemented for you.
:::

:::caution
Make sure your central device communicates with a host device over [USB](../feature-usb-host)
or [Bluetooth](../feature-bluetooth-host). Please follow those documents to implement
your desired functionality.

Although it is possible to compile a central device without them, your keyboard won't
be able to communicate with the host device that you want to use it with.
:::

# Peripheral setup

The "peripheral" device in a split keyboard setup has a switch matrix, and sends matrix events to the central device. A split keyboard setup could have more than one peripheral.
If the split keyboard also uses extra features, then all the peripherals should receive the related commands from the central device.

## Required Cargo features for peripheral device

You must compile a binary with the following `rumcake` features:

- `split-peripheral`
- `drivers` (optional built-in drivers to your peripheral device to a central device)

## Required code for peripheral device

To set up the peripheral device, you must add `split_peripheral(driver = "<driver>")` to your `#[keyboard]` macro invocation,
and your keyboard must implement the appropriate trait for the driver you're using. For example, with `ble` and an nRF5x chip
selected, you must implement `NRFBLEPeripheralDevice`, and `BluetoothDevice`:

```rust ins={6-8,12-24}
// right.rs
use rumcake::keyboard;

#[keyboard(
    // somewhere in your keyboard macro invocation ...
    split_peripheral(
        driver = "ble" // TODO: change this to your desired split driver, and implement the appropriate trait below
    )
)]
struct MyKeyboardRightHalf;

// Bluetooth device setup
use rumcake::hw::mcu::BluetoothDevice;
impl BluetoothDevice for WingpairLeft {
    // Must be valid "Random Static" bluetooth address.
    const BLUETOOTH_ADDRESS: [u8; 6] = [0x92, 0x32, 0x98, 0xC7, 0xF6, 0xF8]; // TODO: Change this to something else
}

// Split peripheral setup
use rumcake::split::drivers::nrf_ble::peripheral::NRFBLEPeripheralDevice;
impl NRFBLEPeripheralDevice for MyKeyboardRightHalf {
    // Must be valid "Random Static" bluetooth address.
    const CENTRAL_ADDRESS: [u8; 6] = [0x41, 0x5A, 0xE3, 0x1E, 0x83, 0xE7]; // Must match the BLUETOOTH_ADDRESS specified in the left half
}
```

:::note
For a peripheral device, you do not have to implement `KeyboardLayout`. Only `KeyboardMatrix` is required.
:::

:::note
In case you are using the `ble` driver, if your keyboard also communicates with your host device using Bluetooth
(basically if you followed the [Bluetooth doc](../feature-bluetooth-host) or chose a template with Bluetooth),
then the `BluetoothDevice` trait should already be implemented for you.
:::

# Central Device Without a Matrix (Dongle)

An example of a central device without a matrix is a dongle. If you would like
to implement such a device, you can add `no_matrix` to your `#[keyboard]` macro invocation.

Doing so will remove the need to implement `KeyboardMatrix`, so you will only have to implement
`KeyboardLayout`.

```rust ins={3}
// dongle.rs
#[keyboard(
    // somewhere in your keyboard macro invocation ...
    no_matrix,
    split_central(
        driver = "ble" // TODO: change this to your desired split driver, and implement the appropriate trait
    )
)]
struct MyKeyboardDongle;

// rest of your config ...
```

# To-do List

- [ ] Method of syncing backlight and underglow commands from central to peripherals on split keyboard setups
- [ ] Single device that can act as both a peripheral and central device
- [ ] Serial (half duplex) driver
- [ ] I2C driver
