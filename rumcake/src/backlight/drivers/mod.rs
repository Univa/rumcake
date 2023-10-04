use core::fmt::Debug;

#[cfg(any(
    feature = "simple-backlight-driver-is31fl3731",
    feature = "simple-backlight-matrix-driver-is31fl3731",
    feature = "rgb-backlight-matrix-driver-is31fl3731"
))]
pub mod is31fl3731;
#[cfg(any(
    feature = "simple-backlight-driver-is31fl3731",
    feature = "simple-backlight-matrix-driver-is31fl3731",
    feature = "rgb-backlight-matrix-driver-is31fl3731"
))]
pub use is31fl3731::setup_backlight_driver;

#[cfg(any(
    feature = "simple-backlight-driver-ws2812-bitbang",
    feature = "simple-backlight-matrix-driver-ws2812-bitbang",
    feature = "rgb-backlight-matrix-driver-ws2812-bitbang"
))]
pub mod ws2812_bitbang;
#[cfg(any(
    feature = "simple-backlight-driver-ws2812-bitbang",
    feature = "simple-backlight-matrix-driver-ws2812-bitbang",
    feature = "rgb-backlight-matrix-driver-ws2812-bitbang"
))]
pub use ws2812_bitbang::setup_backlight_driver;

use super::{BacklightDevice, BacklightMatrixDevice};

pub trait SimpleBacklightDriver<K: BacklightDevice> {
    type DriverError: Debug;

    async fn write(&mut self, brightness: u8) -> Result<(), Self::DriverError>;
}

pub trait SimpleBacklightMatrixDriver<K: BacklightMatrixDevice>
where
    [(); K::MATRIX_COLS]:,
    [(); K::MATRIX_ROWS]:,
{
    type DriverError: Debug;

    async fn write(
        &mut self,
        buf: &[[u8; K::MATRIX_COLS]; K::MATRIX_ROWS],
    ) -> Result<(), Self::DriverError>;
}

pub trait RGBBacklightMatrixDriver<K: BacklightMatrixDevice>
where
    [(); K::MATRIX_COLS]:,
    [(); K::MATRIX_ROWS]:,
{
    type Color;
    type DriverError: Debug;

    async fn write(
        &mut self,
        buf: &[[Self::Color; K::MATRIX_COLS]; K::MATRIX_ROWS],
    ) -> Result<(), Self::DriverError>;
}
