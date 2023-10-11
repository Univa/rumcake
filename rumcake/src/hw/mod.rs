#[cfg(all(not(feature = "stm32"), not(feature = "nrf")))]
compile_error!("Please enable the appropriate feature flag for the chip you're using.");

#[cfg(all(feature = "stm32", feature = "nrf"))]
compile_error!("Please enable only one chip feature flag.");

#[cfg_attr(feature = "stm32", path = "mcu/stm32.rs")]
#[cfg_attr(feature = "nrf", path = "mcu/nrf.rs")]
pub mod mcu;

use crate::State;
pub static BATTERY_LEVEL_STATE: State<u8> = State::new(
    100,
    &[
        #[cfg(feature = "display")]
        &crate::display::BATTERY_LEVEL_LISTENER,
        #[cfg(feature = "bluetooth")]
        &crate::bluetooth::BATTERY_LEVEL_LISTENER,
    ],
);
