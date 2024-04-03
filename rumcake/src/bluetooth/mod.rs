//! Bluetooth host communication.
//!
//! To use Bluetooth host communication, keyboards must implement
//! [`rumcake::hw::platform::BluetoothDevice`], and [`BluetoothKeyboard`].

#[cfg(any(all(feature = "nrf", feature = "bluetooth"), doc))]
pub mod nrf_ble;

use embassy_sync::signal::Signal;

use crate::hw::platform::RawMutex;
use crate::hw::HIDDevice;
use crate::keyboard::Keyboard;
use crate::State;

/// A trait that keyboards must implement to communicate with host devices over Bluetooth (LE).
pub trait BluetoothKeyboard: Keyboard + HIDDevice {
    /// Vendor ID for the keyboard.
    const BLE_VID: u16;

    /// Product ID for the keyboard.
    const BLE_PID: u16;

    /// Product version for the keyboard.
    const BLE_PRODUCT_VERSION: &'static str = Self::HARDWARE_REVISION;
}

pub(crate) static BLUETOOTH_CONNECTED_STATE: State<bool> =
    State::new(false, &[&crate::hw::BLUETOOTH_CONNECTED_STATE_LISTENER]);

pub(crate) static CURRENT_OUTPUT_STATE_LISTENER: Signal<RawMutex, ()> = Signal::new();
pub(crate) static BATTERY_LEVEL_LISTENER: Signal<RawMutex, ()> = Signal::new();
