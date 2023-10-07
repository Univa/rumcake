#[cfg(feature = "display-driver-ssd1306")]
pub mod ssd1306;
#[cfg(feature = "display-driver-ssd1306")]
pub use ssd1306::setup_display_driver;

use super::DisplayDevice;

pub trait DisplayDriver<K: DisplayDevice> {
    /// Called every time a data source updates, or every frame if DisplayDevice::FPS is non-zero.
    fn on_update(&mut self);
}
