use crate::backlight::drivers::{RGBBacklightMatrixDriver, SimpleBacklightMatrixDriver};
use crate::backlight::BacklightMatrixDevice;
use crate::hw::drivers::is31fl3731::is31fl3731::{gamma, Error, IS31FL3731};

use core::fmt::Debug;
use embassy_time::Delay;
use embedded_hal_async::i2c::I2c;
use smart_leds::RGB8;

use super::SimpleBacklightDriver;

pub trait IS31FL3731BacklightDriver: BacklightMatrixDevice
where
    [(); Self::MATRIX_COLS]:,
    [(); Self::MATRIX_ROWS]:,
{
    const LED_DRIVER_ADDR: u32;

    fn get_led_from_matrix_coordinates(x: u8, y: u8) -> u8;
    fn setup_i2c() -> impl I2c<Error = impl Debug>;
}

#[cfg(feature = "rgb-backlight-matrix-driver-is31fl3731")]
#[macro_export]
macro_rules! get_led_from_matrix_coordinates {
    ([] -> [$($body:tt)*]) => {
        [$($body)*]
    };
    ([No $($rest:tt)*] -> [$($body:tt)*]) => {
        get_led_from_matrix_coordinates!([$($rest)*] -> [$($body)* 255,])
    };
    ([$pos:ident $($rest:tt)*] -> [$($body:tt)*]) => {
        get_led_from_matrix_coordinates!([$($rest)*] -> [$($body)* $crate::hw::drivers::is31fl3731::Position::$pos as u8,])
    };
    ({$([$($r_pos:ident)*])*} {$([$($g_pos:ident)*])*} {$([$($b_pos:ident)*])*}) => {
        fn get_led_from_matrix_coordinates(x: u8, y: u8) -> u8 {
            let lookup: [[u8; { Self::MATRIX_COLS * 3 }]; Self::MATRIX_ROWS] = [
                $(
                    get_led_from_matrix_coordinates!([$($r_pos)* $($g_pos)* $($b_pos)*] -> [])
                ),*
            ];

            lookup[y as usize][x as usize] as u8
        }
    };
}

#[cfg(feature = "simple-backlight-matrix-driver-is31fl3731")]
#[macro_export]
macro_rules! get_led_from_matrix_coordinates {
    ([] -> [$($body:tt)*]) => {
        [$($body)*]
    };
    ([No $($rest:tt)*] -> [$($body:tt)*]) => {
        get_led_from_matrix_coordinates!([$($rest)*] -> [$($body)* 255,])
    };
    ([$pos:ident $($rest:tt)*] -> [$($body:tt)*]) => {
        get_led_from_matrix_coordinates!([$($rest)*] -> [$($body)* $crate::hw::drivers::is31fl3731::Position::$pos as u8,])
    };
    ($([$($pos:ident)*])*) => {
        fn get_led_from_matrix_coordinates(x: u8, y: u8) -> u8 {
            let lookup: [[u8; Self::MATRIX_COLS]; Self::MATRIX_ROWS] = [
                $(
                    get_led_from_matrix_coordinates!([$($pos)*] -> [])
                ),*
            ];

            lookup[y as usize][x as usize] as u8
        }
    };
}

pub async fn setup_backlight_driver<K: IS31FL3731BacklightDriver>(
) -> IS31FL3731<impl I2c<Error = impl Debug>>
where
    [(); K::MATRIX_COLS]:,
    [(); K::MATRIX_ROWS]:,
{
    let mut driver = IS31FL3731::new(
        K::setup_i2c(),
        K::LED_DRIVER_ADDR as u8,
        K::MATRIX_COLS as u8,
        K::MATRIX_ROWS as u8,
        K::get_led_from_matrix_coordinates,
    );

    driver.setup(&mut Delay).await.unwrap();

    driver
}

impl<I2CError: Debug, I2C: I2c<Error = I2CError>, K: IS31FL3731BacklightDriver>
    SimpleBacklightDriver<K> for IS31FL3731<I2C>
where
    [(); K::MATRIX_COLS]:,
    [(); K::MATRIX_ROWS]:,
{
    type DriverError = Error<I2CError>;

    async fn write(&mut self, brightness: u8) -> Result<(), Self::DriverError> {
        let payload = [gamma(brightness); 144];

        self.all_pixels(&payload).await?;

        Ok(())
    }
}

impl<I2CError: Debug, I2C: I2c<Error = I2CError>, K: IS31FL3731BacklightDriver>
    SimpleBacklightMatrixDriver<K> for IS31FL3731<I2C>
where
    [(); K::MATRIX_COLS]:,
    [(); K::MATRIX_ROWS]:,
{
    type DriverError = Error<I2CError>;

    async fn write(
        &mut self,
        buf: &[[u8; K::MATRIX_COLS]; K::MATRIX_ROWS],
    ) -> Result<(), Self::DriverError> {
        let mut payload = [0; 144];

        // Map the frame data to LED offsets and set the brightness of the LED in the payload
        for (row_num, row) in buf.iter().enumerate() {
            for (col_num, val) in row.iter().enumerate() {
                let offset = K::get_led_from_matrix_coordinates(col_num as u8, row_num as u8);
                if offset != 255 {
                    payload[offset as usize] = gamma(*val);
                }
            }
        }

        self.all_pixels(&payload).await?;

        Ok(())
    }
}

impl<I2CError: Debug, I2C: I2c<Error = I2CError>, K: IS31FL3731BacklightDriver>
    RGBBacklightMatrixDriver<K> for IS31FL3731<I2C>
where
    [(); K::MATRIX_COLS]:,
    [(); K::MATRIX_ROWS]:,
{
    type Color = RGB8;
    type DriverError = Error<I2CError>;

    async fn write(
        &mut self,
        buf: &[[Self::Color; K::MATRIX_COLS]; K::MATRIX_ROWS],
    ) -> Result<(), Self::DriverError> {
        let mut payload = [0; 144];

        // Map the frame data to LED offsets and set the brightness of the LED in the payload
        for (row_num, row) in buf.iter().enumerate() {
            for (col_num, color) in row.iter().enumerate() {
                for (component, val) in color.iter().enumerate() {
                    let offset = K::get_led_from_matrix_coordinates(
                        col_num as u8 + (component * K::MATRIX_COLS) as u8,
                        row_num as u8,
                    );
                    if offset != 255 {
                        payload[offset as usize] = gamma(val);
                    }
                }
            }
        }

        self.all_pixels(&payload).await?;

        Ok(())
    }
}
