use crate::backlight::drivers::{RGBBacklightMatrixDriver, SimpleBacklightMatrixDriver};
use crate::backlight::BacklightMatrixDevice;
use crate::hw::drivers::ws2812_bitbang::Ws2812;
use embedded_hal::digital::v2::OutputPin;
use smart_leds::{gamma, RGB8};

use super::SimpleBacklightDriver;

pub trait WS2812BitbangBacklightDriver: BacklightMatrixDevice
where
    [(); Self::MATRIX_COLS]:,
    [(); Self::MATRIX_ROWS]:,
{
    fn ws2812_pin() -> impl OutputPin;
    fn get_led_from_matrix_coordinates(x: u8, y: u8) -> Option<u8>;
}

#[macro_export]
macro_rules! get_led_from_matrix_coordinates {
    ($([$($no1:ident)* $($led:literal $($no2:ident)*)* ])*) => {
        fn get_led_from_matrix_coordinates(x: u8, y: u8) -> u8 {
            let lookup: [[u8; Self::MATRIX_COLS]; Self::MATRIX_ROWS] = [
                $([
                    $(${ignore(no1)} None,)*
                    $(Some($led), $(${ignore(no2)} None,)*)*
                ]),*
            ];

            lookup[y as usize][x as usize] as u8
        }
    };
}

pub async fn setup_backlight_driver<K: WS2812BitbangBacklightDriver>() -> Ws2812<impl OutputPin>
where
    [(); K::MATRIX_COLS]:,
    [(); K::MATRIX_ROWS]:,
{
    Ws2812::new(K::ws2812_pin())
}

impl<P: OutputPin, K: WS2812BitbangBacklightDriver> SimpleBacklightDriver<K> for Ws2812<P>
where
    [(); K::MATRIX_COLS]:,
    [(); K::MATRIX_ROWS]:,
    [(); K::MATRIX_ROWS * K::MATRIX_COLS]:,
{
    type DriverError = ();

    async fn write(&mut self, brightness: u8) -> Result<(), Self::DriverError> {
        let brightnesses =
            [(brightness, brightness, brightness).into(); { K::MATRIX_ROWS * K::MATRIX_COLS }];

        self.write_colors(gamma(brightnesses.iter().cloned()));

        Ok(())
    }
}

impl<P: OutputPin, K: WS2812BitbangBacklightDriver> SimpleBacklightMatrixDriver<K> for Ws2812<P>
where
    [(); K::MATRIX_COLS]:,
    [(); K::MATRIX_ROWS]:,
    [(); K::MATRIX_ROWS * K::MATRIX_COLS]:,
{
    type DriverError = ();

    async fn write(
        &mut self,
        buf: &[[u8; K::MATRIX_COLS]; K::MATRIX_ROWS],
    ) -> Result<(), Self::DriverError> {
        let mut brightnesses = [RGB8::default(); { K::MATRIX_ROWS * K::MATRIX_COLS }];

        for (row_num, row) in buf.iter().enumerate() {
            for (col_num, val) in row.iter().enumerate() {
                if let Some(offset) =
                    K::get_led_from_matrix_coordinates(col_num as u8, row_num as u8)
                {
                    brightnesses[offset as usize] = (*val, *val, *val).into();
                }
            }
        }

        self.write_colors(gamma(brightnesses.iter().cloned()));

        Ok(())
    }
}

impl<P: OutputPin, K: WS2812BitbangBacklightDriver> RGBBacklightMatrixDriver<K> for Ws2812<P>
where
    [(); K::MATRIX_COLS]:,
    [(); K::MATRIX_ROWS]:,
    [(); K::MATRIX_ROWS * K::MATRIX_COLS]:,
{
    type DriverError = ();
    type Color = RGB8;

    async fn write(
        &mut self,
        buf: &[[Self::Color; K::MATRIX_COLS]; K::MATRIX_ROWS],
    ) -> Result<(), Self::DriverError> {
        let mut colors = [RGB8::default(); { K::MATRIX_ROWS * K::MATRIX_COLS }];

        for (row_num, row) in buf.iter().enumerate() {
            for (col_num, val) in row.iter().enumerate() {
                if let Some(offset) =
                    K::get_led_from_matrix_coordinates(col_num as u8, row_num as u8)
                {
                    colors[offset as usize] = *val;
                }
            }
        }

        self.write_colors(gamma(colors.iter().cloned()));

        Ok(())
    }
}
