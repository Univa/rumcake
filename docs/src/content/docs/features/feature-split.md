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
- Feature flag for one of the [available split drivers](#available-drivers) that you would like to use

## Required code for central device

To set up the central device, you must add `split_central(driver_setup_fn = <setup_fn>)` to your `#[keyboard]` macro invocation,
and your keyboard must implement the `CentralDevice` trait. Your `CentralDevice` implementation should include `type Layout = Self;`.
This will tell rumcake to redirect matrix events (received from other peripherals) to the layout, to be processed as keycodes.

The `driver_setup_fn` must be an async function that has no parameters, and returns a type that implements the
[`CentralDeviceDriver`](/rumcake/api/nrf52840/rumcake/split/drivers/trait.CentralDeviceDriver.html) trait.

```rust ins={6-8,17-24}
// left.rs
use rumcake::keyboard;

#[keyboard(
    // somewhere in your keyboard macro invocation ...
    split_central(
        driver_setup_fn = my_central_setup
    )
)]
struct MyKeyboardLeftHalf;

// KeyboardLayout should already be implemented
use rumcake::keyboard::KeyboardLayout;
impl KeyboardLayout for MyKeyboardLeftHalf { /* ... */ }

// Split central setup
use rumcake::split::central::{CentralDevice, CentralDeviceDriver};
async fn my_central_setup() -> impl CentralDeviceDriver {
    // TODO: We will fill this out soon!
    todo!()
}
impl CentralDevice for MyKeyboardLeftHalf {
    type Layout = Self;
}
```

:::caution
Make sure your central device communicates with a host device over [USB](../feature-usb-host/)
or [Bluetooth](../feature-bluetooth-host/). Please follow those documents to implement
your desired functionality.

Although it is possible to compile a central device without them, your keyboard won't
be able to communicate with the host device that you want to use it with.
:::

Lastly, you must set up the driver. To do this, you need to complete your `driver_setup_fn` by constructing the driver.
You can [check the API reference for your chosen driver](/rumcake/api/nrf52840/rumcake/drivers/index.html) for a set up
function or macro to make this process easier.

Depending on the driver, you may also need to implement the appropriate trait that corresponds to your chosen driver in the `#[keyboard]` macro.
Check the [list of available split drivers](#available-drivers) for this information.

For example, with the `SerialSplitDriver` struct, you can construct it like so:

```rust del={11-12} ins={13-23}
// KeyboardLayout should already be implemented
use rumcake::keyboard::KeyboardLayout;
impl KeyboardLayout for MyKeyboardLeftHalf { /* ... */ }

// Split central setup
use rumcake::split::central::{CentralDevice, CentralDeviceDriver};
use rumcake::drivers::SerialSplitDriver;
use rumcake::hw::platform::setup_buffered_uarte;
async fn my_central_setup() -> impl CentralDeviceDriver {
    // TODO: We will fill this out soon!
    todo!()
    SerialSplitDriver {
        serial: setup_buffered_uarte! { // Note: this assumes nRF5x, other MCUs have their own macros with their own arguments.
            interrupt: UARTE0_UART0,
            uarte: UARTE0,
            timer: TIMER1,
            ppi_ch0: PPI_CH0,
            ppi_ch1: PPI_CH1,
            ppi_group: PPI_GROUP0,
            rx_pin: P0_29,
            tx_pin: P0_31,
        },
    }
}
impl CentralDevice for MyKeyboardLeftHalf {
    type Layout = Self;
}
```

:::note
If you would like to use nRF BLE as the driver for split keyboard communication, see the [nRF-BLE](#nrf-ble-driver) section for more instruction.
:::

# Peripheral setup

The "peripheral" device in a split keyboard setup has a switch matrix, and sends matrix events to the central device. A split keyboard setup could have more than one peripheral.
If the split keyboard also uses extra features, then all the peripherals should receive the related commands from the central device.

## Required Cargo features for peripheral device

You must compile a binary with the following `rumcake` features:

- `split-peripheral`
- Feature flag for one of the [available split drivers](#available-drivers) that you would like to use

## Required code for peripheral device

To set up the peripheral device, you must add `split_peripheral(driver_setup_fn = <setup_fn>)` to your `#[keyboard]` macro invocation,
and your keyboard must implement the `PeripheralDevice` trait. Your `KeyboardMatrix` implementation (which should already be implemented)
should include `type PeripheralDeviceType = Self`. This will tell rumcake to redirect matrix events to the peripheral device driver, to
be sent to the central device.

The `driver_setup_fn` must be an async function that has no parameters, and returns a type that implements the
[`PeripheralDeviceDriver`](/rumcake/api/nrf52840/rumcake/split/drivers/trait.PeripheralDeviceDriver.html) trait.

```rust ins={6-8,12-24}
// right.rs
use rumcake::keyboard;

#[keyboard(
    // somewhere in your keyboard macro invocation ...
    split_peripheral(
        driver_setup_fn = my_peripheral_setup
    )
)]
struct MyKeyboardRightHalf;

// KeyboardMatrix should already be implemented
use rumcake::keyboard::KeyboardMatrix;
impl KeyboardMatrix for MyKeyboardRightHalf {
    type PeripheralDeviceType = Self;
}

// Split peripheral setup
use rumcake::split::peripheral::{PeripheralDevice, PeripheralDeviceDriver};
async fn my_peripheral_setup() -> impl PeripheralDeviceDriver {
    // TODO: We will fill this out soon!
    todo!()
}
impl PeripheralDevice for MyKeyboardRightHalf {}
```

:::note
For a peripheral device, you do not have to implement `KeyboardLayout`. Only `KeyboardMatrix` is required.
:::

Lastly, you must set up the driver. To do this, you need to complete your `driver_setup_fn` by constructing the driver.
You can [check the API reference for your chosen driver](/rumcake/api/nrf52840/rumcake/drivers/index.html) for a set up
function or macro to make this process easier.

Depending on the driver, you may also need to implement the appropriate trait that corresponds to your chosen driver in the `#[keyboard]` macro.
Check the [list of available split drivers](#available-drivers) for this information.

For example, with the `SerialSplitDriver` struct, you can construct it like so:

```rust del={10-11} ins={12-23}
// KeyboardLayout should already be implemented
use rumcake::keyboard::KeyboardLayout;
impl KeyboardLayout for MyKeyboardLeftHalf { /* ... */ }

// Split central setup
use rumcake::drivers::SerialSplitDriver;
use rumcake::hw::platform::setup_buffered_uarte;
use rumcake::split::peripheral::{PeripheralDevice, PeripheralDeviceDriver};
async fn my_peripheral_setup() -> impl PeripheralDeviceDriver {
    // TODO: We will fill this out soon!
    todo!()
    SerialSplitDriver {
        serial: setup_buffered_uarte! { // Note: this assumes nRF5x, other MCUs have their own macros with their own arguments.
            interrupt: UARTE0_UART0,
            uarte: UARTE0,
            timer: TIMER1,
            ppi_ch0: PPI_CH0,
            ppi_ch1: PPI_CH1,
            ppi_group: PPI_GROUP0,
            rx_pin: P0_31,
            tx_pin: P0_29,
        },
    }
}
impl PeripheralDevice for MyKeyboardRightHalf {}
```

:::note
If you would like to use nRF BLE as the driver for split keyboard communication, see the [nRF-BLE](#nrf-ble-driver) section for more instruction.
:::

# Central Device Without a Matrix (Dongle)

An example of a central device without a matrix is a dongle. If you would like
to implement such a device, you can add `no_matrix` to your `#[keyboard]` macro invocation.

Doing so will remove the need to implement `KeyboardMatrix`, so you will only have to implement
`KeyboardLayout`.

```rust ins={6}
// dongle.rs
use rumcake::keyboard;

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

# nRF-BLE Driver

If you are using an nRF5x MCU, and want to use BLE for split keyboard communication, there are additional changes
you need to make it work.

For both central and peripheral devices, the [`BluetoothDevice`](/rumcake/api/nrf52840/rumcake/hw/platform/trait.BluetoothDevice.html) trait must be implemented:

`BLUETOOTH_ADDRESS` can be whatever you want, as long as it is a valid "Random Static" bluetooth address.
See "Random Static Address" here: https://novelbits.io/bluetooth-address-privacy-ble/

```rust ins={2-5}
// central file
use rumcake::hw::platform::BluetoothDevice;
impl BluetoothDevice for MyKeyboardLeftHalf {
    const BLUETOOTH_ADDRESS: [u8; 6] = [0x41, 0x5A, 0xE3, 0x1E, 0x83, 0xE7]; // TODO: Change this to something else
}
```

```rust ins={2-5}
// peripheral file
use rumcake::hw::platform::BluetoothDevice;
impl BluetoothDevice for MyKeyboardRightHalf {
    const BLUETOOTH_ADDRESS: [u8; 6] = [0x92, 0x32, 0x98, 0xC7, 0xF6, 0xF8]; // TODO: Change this to something else
}
```

:::note
In case you are using the `ble` driver, if your keyboard also communicates with your host device using Bluetooth
(basically if you followed the [Bluetooth doc](../feature-bluetooth-host/) or chose a template with Bluetooth),
then the `BluetoothDevice` trait should already be implemented for you.
:::

You will also need to change the `#[keyboard]` macro invocation to add `driver_type = "nrf-ble"`.
This will change the requirements for the signature of `driver_setup_fn`.

```rust ins={6}
// central file
#[keyboard(
    // somewhere in your keyboard macro invocation ...
    split_central(
        driver_setup_fn = my_central_setup,
        driver_type = "nrf-ble"
    )
)]
struct MyKeyboardLeftHalf;
```

```rust ins={6}
// peripheral file
#[keyboard(
    // somewhere in your keyboard macro invocation ...
    split_peripheral(
        driver_setup_fn = my_peripheral_setup,
        driver_type = "nrf-ble"
    )
)]
struct MyKeyboardRightHalf;
```

Now, your `driver_setup_fn` will need to change it's signature.

For central devices, it will need to return:

- `CentralDeviceDriver` implementor
- A slice containing peripheral addresses to connect to

For peripheral devices, it will need to return:

- `PeripheralDeviceDriver` implementor
- Address of the central device to connect to

The `setup_nrf_ble_split_central!` and `setup_nrf_ble_split_peripheral!` driver can be used to
implement your `driver_setup_fn`.

```rust del={3} ins={4-9}
// central file
use rumcake::drivers::nrf_ble::central::setup_nrf_ble_split_central;
async fn my_central_setup() -> impl CentralDeviceDriver {
async fn my_central_setup() -> (impl CentralDeviceDriver, &'static [[u8; 6]]) {
    setup_nrf_ble_split_central! {
        peripheral_addresses: [
            [0x92, 0x32, 0x98, 0xC7, 0xF6, 0xF8] // address of peripheral we specified in the peripheral device's file
        ]
    }
}
```

```rust del={3} ins={4-7}
// peripheral file
use rumcake::drivers::nrf_ble::peripheral::setup_nrf_ble_split_peripheral;
async fn my_peripheral_setup() -> impl PeripheralDeviceDriver {
async fn my_peripheral_setup() -> (impl PeripheralDeviceDriver, [u8; 6]) {
    setup_nrf_ble_split_peripheral! {
        central_address: [0x41, 0x5A, 0xE3, 0x1E, 0x83, 0xE7] // address of central device we specified in the central device's file
    }
}
```

# To-do List

- [ ] Method of syncing backlight and underglow commands from central to peripherals on split keyboard setups
- [ ] Single device that can act as both a peripheral and central device
- [ ] Serial (half duplex) driver
- [ ] I2C driver

# Available Drivers

| Name             | Feature Flag               | Required Traits                                                                      |
| ---------------- | -------------------------- | ------------------------------------------------------------------------------------ |
| Serial[^1]       | N/A (available by default) | N/A                                                                                  |
| nRF Bluetooth LE | `nrf-ble`                  | [`BluetoothDevice`](/rumcake/api/nrf52840/rumcake/hw/mcu/trait.BluetoothDevice.html) |

[^1]:
    Compatible with any type that implements both `embedded_io_async::Read` and `embedded_io_async::Write`.
    This includes `embassy_nrf::buffered_uarte::BufferedUarte` (nRF UARTE) and `embassy_stm32::usart::BufferedUart` (STM32 UART).
