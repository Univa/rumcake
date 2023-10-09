# Split keyboards

**⚠ This feature is still a work in progress.**

<!--toc:start-->

- [Example](#example)
- [Central setup](#central-setup)
  - [Required Cargo features for central device](#required-cargo-features-for-central-device)
  - [Required code for central device](#required-code-for-central-device)
- [Peripheral setup](#peripheral-setup)
  - [Required Cargo features for peripheral device](#required-cargo-features-for-peripheral-device)
  - [Required code for peripheral device](#required-code-for-peripheral-device)
- [To-do List](#to-do-list)
<!--toc:end-->

## Example

For a detailed example of how to implement a split keyboard, check the
[template repo](https://github.com/Univa/rumcake-templates).

Generally, a split keyboard will require compiling multiple binaries. For example, you will need one binary for the left half, and another binary for the right half.

## Central setup

The "central" device in a split keyboard setup defines the keyboard layout, communicates with the host device, and receives matrix events from other peripherals. There should only be one central device.
If the split keyboard also uses extra features like backlighting or underglow, the central device will also be responsible for sending their related commands to the peripherals.

Typically, the central device could be a dongle (good for saving battery life), or one of the keyboard halves.

### Required Cargo features for central device

You must compile a binary with the following `rumcake` features:

- `split-central`
- `split-driver-<driver>` (e.g. `split-driver-ble`)
  - `driver` is one of:
    - `ble` (only for nRF-based keyboards, also requires the `nrf-ble` feature)
    - `serial` (not yet working)

### Required code for central device

To set up the split-central device, your keyboard must implement the appropriate trait for the driver you're using.
For example, with `split-driver-ble`, you must implement `NRFBLECentralDevice`:

```rust
use rumcake::keyboard;

#[keyboard]
struct MyKeyboardLeftHalf;

// Split central setup
use rumcake::split::drivers::nrf_ble::central::NRFBLECentralDevice;
// NRFBLECentralDevice<2>: This central device can connect to two different peripherals
impl NRFBLECentralDevice<2> for MyKeyboardLeftHalf {
    // These addresses can be whatever you want, as long as they are valid "Random Static" bluetooth addresses.
    // See "Random Static Address" in this link: https://novelbits.io/bluetooth-address-privacy-ble/
    const BLUETOOTH_ADDRESS: [u8; 6] = [0x41, 0x5A, 0xE3, 0x1E, 0x83, 0xE7];
    const PERIPHERAL_ADDRESSES: [[u8; 6]; 2] = [
        [0x92, 0x32, 0x98, 0xC7, 0xF6, 0xF8],
        [0x15, 0xD6, 0x88, 0x85, 0x98, 0xF7],
    ];
}
```

## Peripheral setup

The "peripheral" device in a split keyboard setup has a switch matrix, and sends matrix events to the central device. A split keyboard setup could have more than one peripheral.
If the split keyboard also uses extra features, then all the peripherals should receive the related commands from the central device.

### Required Cargo features for peripheral device

You must compile a binary with the following `rumcake` features:

- `split-peripheral`
- `split-driver-<driver>` (same as previous)

### Required code for peripheral device

To set up the split-central device, your keyboard must implement the appropriate trait for the driver you're using.
For example, with `split-driver-ble`, you must implement `NRFBLEPeripheralDevice`:

```rust
use rumcake::keyboard;

#[keyboard]
struct MyKeyboardRightHalf;

// Split peripheral setup
use rumcake::split::drivers::nrf_ble::peripheral::NRFBLEPeripheralDevice;
impl NRFBLEPeripheralDevice for MyKeyboardRightHalf {
    const BLUETOOTH_ADDRESS: [u8; 6] = [0x92, 0x32, 0x98, 0xC7, 0xF6, 0xF8];
    const CENTRAL_ADDRESS: [u8; 6] = [0x41, 0x5A, 0xE3, 0x1E, 0x83, 0xE7]; // Must match the address specified in the left half
}
```

## To-do List

- [ ] Disable keyboard matrix tasks for dongles
- [ ] Serial (half duplex) driver
- [ ] I2C driver
