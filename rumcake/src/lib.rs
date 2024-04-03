#![no_std]
#![feature(stdsimd)]
#![feature(generic_const_exprs)]
#![feature(associated_type_defaults)]
#![warn(missing_docs)]
#![doc = include_str!("../../README.md")]

use embassy_sync::mutex::{Mutex, MutexGuard};
use embassy_sync::signal::Signal;

use crate::hw::platform::RawMutex;

pub(crate) trait StaticArray {
    const LEN: usize;
}

impl<T, const N: usize> StaticArray for [T; N] {
    const LEN: usize = N;
}

trait LEDEffect {
    fn is_animated(&self) -> bool;
    fn is_reactive(&self) -> bool;
}

trait Cycle {
    fn increment(&mut self);
    fn decrement(&mut self);
}

/// Data structure that allows you to notify listeners about any changes to the data being managed.
/// This can be useful when you want a task to react to changes to certain data.
pub struct State<'a, T: Clone + PartialEq> {
    data: Mutex<RawMutex, T>,
    listeners: &'a [&'a Signal<RawMutex, ()>],
}

impl<'a, T: Clone + PartialEq> State<'a, T> {
    /// Create some new state, with the specified listeners.
    pub const fn new(data: T, listeners: &'a [&'a Signal<RawMutex, ()>]) -> State<'a, T> {
        Self {
            data: Mutex::new(data),
            listeners,
        }
    }

    /// Obtain the state's current value.
    pub async fn get(&self) -> T {
        self.data.lock().await.clone()
    }

    async fn set_inner(&self, value: T) -> bool {
        let mut data = self.data.lock().await;
        let changed = *data != value;
        *data = value;
        changed
    }

    /// Update state and notify listeners
    pub async fn set(&self, value: T) {
        if self.set_inner(value).await {
            self.notify_listeners();
        }
    }

    /// Update state without notifying listeners
    pub async fn quiet_set(&self, value: T) {
        self.set_inner(value).await;
    }

    async fn update_inner<R>(
        &self,
        updater: impl FnOnce(&mut MutexGuard<'_, RawMutex, T>) -> R,
    ) -> (bool, R) {
        let mut data = self.data.lock().await;
        let old = data.clone();
        let update_result = updater(&mut data);
        (old != *data, update_result)
    }

    /// Update state using a function, and notify listeners
    pub async fn update<R>(
        &self,
        updater: impl FnOnce(&mut MutexGuard<'_, RawMutex, T>) -> R,
    ) -> R {
        let (changed, update_result) = self.update_inner(updater).await;

        if changed {
            self.notify_listeners();
        }

        update_result
    }

    /// Update state using a function without notifying listeners
    pub async fn quiet_update<R>(
        &self,
        updater: impl FnOnce(&mut MutexGuard<'_, RawMutex, T>) -> R,
    ) -> R {
        let (_changed, update_result) = self.update_inner(updater).await;
        update_result
    }

    /// Send a signal to the listeners. Normally used to notify listeners of any changes to state.
    fn notify_listeners(&self) {
        for listener in self.listeners.iter() {
            listener.signal(());
        }
    }
}

pub use keyberon;
pub use once_cell;

pub use rumcake_macros::keyboard_main as keyboard;

pub mod keyboard;
mod math;

#[cfg(feature = "storage")]
pub mod storage;

#[cfg(feature = "lighting")]
pub mod lighting;

#[cfg(feature = "usb")]
pub mod usb;

#[cfg(feature = "via")]
pub mod via;

#[cfg(feature = "vial")]
pub mod vial;

#[cfg(any(feature = "split-peripheral", feature = "split-central"))]
pub mod split;

#[cfg(feature = "bluetooth")]
pub mod bluetooth;

#[cfg(feature = "display")]
pub mod display;

pub mod hw;

pub mod drivers;

pub mod tasks {
    pub use crate::hw::__output_switcher;
    pub use crate::keyboard::{__layout_collect, __matrix_poll};

    #[cfg(all(feature = "lighting", feature = "storage"))]
    pub use crate::lighting::__lighting_storage_task;
    #[cfg(feature = "lighting")]
    pub use crate::lighting::__lighting_task;

    #[cfg(feature = "display")]
    pub use crate::display::__display_task;

    #[cfg(feature = "usb")]
    pub use crate::usb::{__start_usb, __usb_hid_consumer_write_task, __usb_hid_kb_write_task};

    #[cfg(all(feature = "via", feature = "usb"))]
    pub use crate::usb::__usb_hid_via_read_task;
    #[cfg(all(feature = "via", feature = "usb"))]
    pub use crate::usb::__usb_hid_via_write_task;
    #[cfg(feature = "via")]
    pub use crate::via::__via_process_task;

    #[cfg(feature = "vial")]
    pub use crate::vial::__vial_process_task;

    #[cfg(feature = "split-central")]
    pub use crate::split::central::__central_task;

    #[cfg(feature = "split-peripheral")]
    pub use crate::split::peripheral::__peripheral_task;

    #[cfg(feature = "nrf")]
    pub use crate::hw::platform::__adc_task;

    #[cfg(feature = "nrf-ble")]
    pub use crate::hw::platform::__softdevice_task;

    #[cfg(all(feature = "nrf", feature = "bluetooth"))]
    pub use crate::bluetooth::nrf_ble::__nrf_ble_task;

    #[cfg(all(feature = "nrf-ble", feature = "split-central"))]
    pub use crate::drivers::nrf_ble::central::__nrf_ble_central_task;
    #[cfg(all(feature = "nrf-ble", feature = "split-peripheral"))]
    pub use crate::drivers::nrf_ble::peripheral::__nrf_ble_peripheral_task;
}
