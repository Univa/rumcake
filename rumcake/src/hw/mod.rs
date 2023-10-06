use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::pubsub::PubSubChannel;

#[cfg(all(not(feature = "stm32"), not(feature = "nrf")))]
compile_error!("Please enable the appropriate feature flag for the chip you're using.");

#[cfg(all(feature = "stm32", feature = "nrf"))]
compile_error!("Please enable only one chip feature flag.");

#[cfg_attr(feature = "stm32", path = "mcu/stm32.rs")]
#[cfg_attr(feature = "nrf", path = "mcu/nrf.rs")]
pub mod mcu;

pub static BATTERY_LEVEL: PubSubChannel<ThreadModeRawMutex, u8, 2, 2, 1> = PubSubChannel::new();
