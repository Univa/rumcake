#[cfg(feature = "display-driver-ssd1306")]
pub mod ssd1306;
#[cfg(feature = "display-driver-ssd1306")]
pub use ssd1306::setup_display_driver;

pub trait DisplayDriver {
    /// Called every time a data source updates, or every frame if DisplayDevice::FPS is non-zero.
    fn on_update(&self);
}
