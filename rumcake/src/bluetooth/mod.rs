#[cfg(all(feature = "nrf", feature = "bluetooth"))]
pub mod nrf_ble;

use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;

use crate::keyboard::Keyboard;

pub trait BluetoothKeyboard: Keyboard {
    const BLE_VID: u16;
    const BLE_PID: u16;
    const BLE_PRODUCT_VERSION: &'static str = Self::HARDWARE_REVISION;
}

#[derive(Debug, Clone, Copy)]
pub enum BluetoothCommand {
    #[cfg(feature = "usb")]
    ToggleUSB, // Switch between bluetooth and USB operation
}

pub static BLUETOOTH_COMMAND_CHANNEL: Channel<ThreadModeRawMutex, BluetoothCommand, 2> =
    Channel::new();

pub static USB_STATE_LISTENER: Signal<ThreadModeRawMutex, ()> = Signal::new();
pub static BATTERY_LEVEL_LISTENER: Signal<ThreadModeRawMutex, ()> = Signal::new();
