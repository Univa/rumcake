#![no_std]
#![feature(stdsimd)]
#![feature(macro_metavar_expr)]
#![feature(generic_const_exprs)]
#![feature(type_alias_impl_trait)]
#![feature(associated_type_defaults)]
#![warn(missing_docs)]
#![doc = include_str!("../../README.md")]

use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::{Mutex, MutexGuard};
use embassy_sync::signal::Signal;

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
    data: Mutex<ThreadModeRawMutex, T>,
    listeners: &'a [&'a Signal<ThreadModeRawMutex, ()>],
}

impl<'a, T: Clone + PartialEq> State<'a, T> {
    /// Create some new state, with the specified listeners.
    pub const fn new(data: T, listeners: &'a [&'a Signal<ThreadModeRawMutex, ()>]) -> State<'a, T> {
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
        updater: impl FnOnce(&mut MutexGuard<'_, ThreadModeRawMutex, T>) -> R,
    ) -> (bool, R) {
        let mut data = self.data.lock().await;
        let old = data.clone();
        let update_result = updater(&mut data);
        (old != *data, update_result)
    }

    /// Update state using a function, and notify listeners
    pub async fn update<R>(
        &self,
        updater: impl FnOnce(&mut MutexGuard<'_, ThreadModeRawMutex, T>) -> R,
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
        updater: impl FnOnce(&mut MutexGuard<'_, ThreadModeRawMutex, T>) -> R,
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

// TODO: remove re-exports

pub use embedded_hal;
pub use embedded_hal_async;
pub use embedded_storage_async;
pub use keyberon;

pub use rumcake_macros::main as keyboard;

pub mod keyboard;
mod math;

#[cfg(feature = "storage")]
pub mod storage;

#[cfg(feature = "underglow")]
pub mod underglow;

#[cfg(feature = "_backlight")]
pub mod backlight;

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

#[cfg(feature = "drivers")]
pub mod drivers;

pub mod tasks {
    pub use crate::hw::__output_switcher;
    pub use crate::keyboard::{__layout_collect, __matrix_poll};

    #[cfg(feature = "simple-backlight")]
    pub use crate::backlight::simple_backlight::__simple_backlight_task;
    #[cfg(all(feature = "storage", feature = "simple-backlight"))]
    pub use crate::backlight::simple_backlight::storage::__simple_backlight_storage_task;

    #[cfg(feature = "simple-backlight-matrix")]
    pub use crate::backlight::simple_backlight_matrix::__simple_backlight_matrix_task;
    #[cfg(all(feature = "storage", feature = "simple-backlight-matrix"))]
    pub use crate::backlight::simple_backlight_matrix::storage::__simple_backlight_matrix_storage_task;

    #[cfg(feature = "rgb-backlight-matrix")]
    pub use crate::backlight::rgb_backlight_matrix::__rgb_backlight_matrix_task;
    #[cfg(all(feature = "storage", feature = "rgb-backlight-matrix"))]
    pub use crate::backlight::rgb_backlight_matrix::storage::__rgb_backlight_matrix_storage_task;

    #[cfg(feature = "underglow")]
    pub use crate::underglow::__underglow_task;
    #[cfg(all(feature = "underglow", feature = "storage"))]
    pub use crate::underglow::storage::__underglow_storage_task;

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
    #[cfg(all(feature = "via", feature = "storage"))]
    pub use crate::via::storage::__via_storage_task;

    #[cfg(feature = "vial")]
    pub use crate::vial::__vial_process_task;
    #[cfg(all(feature = "vial", feature = "storage"))]
    pub use crate::vial::storage::__vial_storage_task;

    #[cfg(feature = "split-central")]
    pub use crate::split::central::__central_task;

    #[cfg(feature = "split-peripheral")]
    pub use crate::split::peripheral::__peripheral_task;

    #[cfg(feature = "nrf")]
    pub use crate::hw::mcu::__adc_task;

    #[cfg(feature = "nrf-ble")]
    pub use crate::hw::mcu::__softdevice_task;

    #[cfg(all(feature = "nrf", feature = "bluetooth"))]
    pub use crate::bluetooth::nrf_ble::__nrf_ble_task;

    #[cfg(all(feature = "drivers", feature = "nrf-ble", feature = "split-central"))]
    pub use crate::drivers::nrf_ble::central::__nrf_ble_central_task;
    #[cfg(all(feature = "drivers", feature = "nrf-ble", feature = "split-peripheral"))]
    pub use crate::drivers::nrf_ble::peripheral::__nrf_ble_peripheral_task;
}
