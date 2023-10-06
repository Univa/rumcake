#![no_std]
#![feature(stdsimd)]
#![feature(macro_metavar_expr)]
#![feature(generic_const_exprs)]
#![feature(type_alias_impl_trait)]
#![feature(return_position_impl_trait_in_trait)]
#![feature(async_fn_in_trait)]

pub trait StaticArray {
    const LEN: usize;
}

impl<T, const N: usize> StaticArray for [T; N] {
    const LEN: usize = N;
}

trait LEDEffect {
    fn is_animated(&self) -> bool;
}

trait Cycle {
    fn increment(&mut self);
    fn decrement(&mut self);
}

// TODO: remove re-exports

#[cfg(feature = "stm32")]
pub use embassy_stm32;

#[cfg(feature = "nrf")]
pub use embassy_nrf;

pub use embassy_executor;
pub use embassy_sync;
pub use embassy_usb;
pub use embedded_hal;
pub use embedded_hal_async;
pub use keyberon;
pub use smart_leds;
pub use static_cell;

pub use rumcake_macros::main as keyboard;

pub mod keyboard;
mod math;

#[cfg(feature = "eeprom")]
pub mod eeprom;

#[cfg(feature = "underglow")]
pub mod underglow;

#[cfg(feature = "backlight")]
pub mod backlight;

#[cfg(feature = "usb")]
pub mod usb;

#[cfg(feature = "via")]
pub mod via;

#[cfg(feature = "vial")]
pub mod vial;

#[cfg(any(feature = "split-peripheral", feature = "split-central"))]
pub mod split;

#[cfg(all(feature = "nrf", feature = "bluetooth"))]
pub mod nrf_ble;

pub mod hw;

pub mod tasks {
    pub use crate::keyboard::{__layout_collect_task, __layout_register_task, __matrix_poll_task};

    #[cfg(feature = "backlight")]
    pub use crate::backlight::__backlight_task_task;

    #[cfg(feature = "underglow")]
    pub use crate::underglow::__underglow_task_task;

    #[cfg(feature = "usb")]
    pub use crate::usb::{__start_usb_task, __usb_hid_kb_write_task_task};

    #[cfg(feature = "via")]
    pub use crate::via::__usb_hid_via_read_task_task;
    #[cfg(feature = "via")]
    pub use crate::via::__usb_hid_via_write_task_task;

    #[cfg(feature = "vial")]
    pub use crate::vial::__usb_hid_vial_write_task_task;

    #[cfg(feature = "split-central")]
    pub use crate::split::central::__central_task_task;

    #[cfg(feature = "split-peripheral")]
    pub use crate::split::peripheral::__peripheral_task_task;

    #[cfg(feature = "nrf")]
    pub use crate::hw::mcu::__adc_task_task;

    #[cfg(feature = "nrf-ble")]
    pub use crate::hw::mcu::__softdevice_task_task;

    #[cfg(all(feature = "nrf", feature = "bluetooth"))]
    pub use crate::nrf_ble::__nrf_ble_task_task;

    #[cfg(all(
        feature = "nrf",
        feature = "split-driver-ble",
        feature = "split-central"
    ))]
    pub use crate::split::drivers::nrf_ble::central::__nrf_ble_central_task_task;

    #[cfg(all(
        feature = "nrf",
        feature = "split-driver-ble",
        feature = "split-peripheral"
    ))]
    pub use crate::split::drivers::nrf_ble::peripheral::__nrf_ble_peripheral_task_task;
}
