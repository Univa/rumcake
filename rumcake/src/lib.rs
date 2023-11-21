#![no_std]
#![feature(stdsimd)]
#![feature(macro_metavar_expr)]
#![feature(generic_const_exprs)]
#![feature(type_alias_impl_trait)]
#![feature(associated_type_defaults)]
#![feature(return_position_impl_trait_in_trait)]
#![feature(async_fn_in_trait)]
#![warn(missing_docs)]
#![doc = include_str!("../../README.md")]

use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::{Mutex, MutexGuard};
use embassy_sync::signal::Signal;

pub trait StaticArray {
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

pub struct State<'a, T: Clone + PartialEq> {
    data: Mutex<ThreadModeRawMutex, T>,
    listeners: &'a [&'a Signal<ThreadModeRawMutex, ()>],
}

impl<'a, T: Clone + PartialEq> State<'a, T> {
    const fn new(data: T, listeners: &'a [&'a Signal<ThreadModeRawMutex, ()>]) -> State<'a, T> {
        Self {
            data: Mutex::new(data),
            listeners,
        }
    }

    async fn get(&self) -> T {
        self.data.lock().await.clone()
    }

    async fn set_inner(&self, value: T) -> bool {
        let mut data = self.data.lock().await;
        let changed = *data != value;
        *data = value;
        changed
    }

    /// Update state and notify listeners
    async fn set(&self, value: T) {
        if self.set_inner(value).await {
            self.notify_listeners();
        }
    }

    /// Update state without notifying listeners
    async fn quiet_set(&self, value: T) {
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
    async fn update<R>(
        &self,
        updater: impl FnOnce(&mut MutexGuard<'_, ThreadModeRawMutex, T>) -> R,
    ) -> R {
        let (changed, update_result) = self.update_inner(updater).await;

        if changed {
            self.notify_listeners();
        }

        update_result
    }

    /// Update state using a function without notify listeners
    async fn quiet_update<R>(
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

#[cfg(feature = "stm32")]
pub use embassy_stm32;

#[cfg(feature = "nrf")]
pub use embassy_nrf;

pub use embassy_executor;
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
    pub use crate::keyboard::{__layout_collect_task, __matrix_poll_task};

    #[cfg(any(
        feature = "simple-backlight",
        feature = "simple-backlight-matrix",
        feature = "rgb-backlight-matrix"
    ))]
    pub use crate::backlight::__backlight_task_task;
    #[cfg(all(
        any(
            feature = "simple-backlight",
            feature = "simple-backlight-matrix",
            feature = "rgb-backlight-matrix"
        ),
        feature = "storage"
    ))]
    pub use crate::backlight::storage::__backlight_storage_task_task;

    #[cfg(feature = "underglow")]
    pub use crate::underglow::__underglow_task_task;
    #[cfg(all(feature = "underglow", feature = "storage"))]
    pub use crate::underglow::storage::__underglow_storage_task_task;

    #[cfg(feature = "display")]
    pub use crate::display::__display_task_task;

    #[cfg(feature = "usb")]
    pub use crate::usb::{__start_usb_task, __usb_hid_kb_write_task_task};

    #[cfg(feature = "via")]
    pub use crate::via::__usb_hid_via_read_task_task;
    #[cfg(feature = "via")]
    pub use crate::via::__usb_hid_via_write_task_task;
    #[cfg(all(feature = "via", feature = "storage"))]
    pub use crate::via::storage::__via_storage_task_task;

    #[cfg(feature = "vial")]
    pub use crate::vial::__usb_hid_vial_write_task_task;
    #[cfg(all(feature = "vial", feature = "storage"))]
    pub use crate::vial::storage::__vial_storage_task_task;

    #[cfg(feature = "split-central")]
    pub use crate::split::central::__central_task_task;

    #[cfg(feature = "split-peripheral")]
    pub use crate::split::peripheral::__peripheral_task_task;

    #[cfg(feature = "nrf")]
    pub use crate::hw::mcu::__adc_task_task;

    #[cfg(feature = "nrf-ble")]
    pub use crate::hw::mcu::__softdevice_task_task;

    #[cfg(all(feature = "nrf", feature = "bluetooth"))]
    pub use crate::bluetooth::nrf_ble::__nrf_ble_task_task;

    #[cfg(all(feature = "drivers", feature = "split-central"))]
    pub use crate::drivers::nrf_ble::central::__nrf_ble_central_task_task;
    #[cfg(all(feature = "drivers", feature = "split-peripheral"))]
    pub use crate::drivers::nrf_ble::peripheral::__nrf_ble_peripheral_task_task;
}
