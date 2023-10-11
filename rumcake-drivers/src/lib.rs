#![no_std]
#![feature(stdsimd)]
#![feature(macro_metavar_expr)]
#![feature(generic_const_exprs)]
#![feature(async_fn_in_trait)]
#![feature(return_position_impl_trait_in_trait)]

pub use embassy_executor;

pub mod is31fl3731;
pub mod nrf_ble;
pub mod ssd1306;
pub mod ws2812_bitbang;

pub mod tasks {
    pub use crate::nrf_ble::central::__nrf_ble_central_task_task;
    pub use crate::nrf_ble::peripheral::__nrf_ble_peripheral_task_task;
}
