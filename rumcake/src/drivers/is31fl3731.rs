//! Rumcake driver implementations for [gleich's IS31FL3731 driver](`is31fl3731`).
//!
//! This driver provides implementations for
//! [`SimpleBacklightDriver`](`crate::backlight::drivers::SimpleBacklightDriver`),
//! [`SimpleBacklightMatrixDriver`](`crate::backlight::drivers::SimpleBacklightMatrixDriver`), and
//! [`RGBBacklightMatrixDriver`](`crate::backlight::drivers::RGBBacklightMatrixDriver`)
//!
//! To use this driver for backlighting, keyboards must implement [`IS31FL3731BacklightDriver`](backlight::IS31FL3731BacklightDriver).

pub use is31fl3731 as driver;

#[repr(u8)]
#[allow(missing_docs)]
/// Possible positions on an IS31FL3731-backed charlieplexed matrix. Consult the datasheet for more details.
pub enum Position {
    C1_1 = 0x00,
    C1_2,
    C1_3,
    C1_4,
    C1_5,
    C1_6,
    C1_7,
    C1_8,
    C1_9,
    C1_10,
    C1_11,
    C1_12,
    C1_13,
    C1_14,
    C1_15,
    C1_16,

    C2_1,
    C2_2,
    C2_3,
    C2_4,
    C2_5,
    C2_6,
    C2_7,
    C2_8,
    C2_9,
    C2_10,
    C2_11,
    C2_12,
    C2_13,
    C2_14,
    C2_15,
    C2_16,

    C3_1,
    C3_2,
    C3_3,
    C3_4,
    C3_5,
    C3_6,
    C3_7,
    C3_8,
    C3_9,
    C3_10,
    C3_11,
    C3_12,
    C3_13,
    C3_14,
    C3_15,
    C3_16,

    C4_1,
    C4_2,
    C4_3,
    C4_4,
    C4_5,
    C4_6,
    C4_7,
    C4_8,
    C4_9,
    C4_10,
    C4_11,
    C4_12,
    C4_13,
    C4_14,
    C4_15,
    C4_16,

    C5_1,
    C5_2,
    C5_3,
    C5_4,
    C5_5,
    C5_6,
    C5_7,
    C5_8,
    C5_9,
    C5_10,
    C5_11,
    C5_12,
    C5_13,
    C5_14,
    C5_15,
    C5_16,

    C6_1,
    C6_2,
    C6_3,
    C6_4,
    C6_5,
    C6_6,
    C6_7,
    C6_8,
    C6_9,
    C6_10,
    C6_11,
    C6_12,
    C6_13,
    C6_14,
    C6_15,
    C6_16,

    C7_1,
    C7_2,
    C7_3,
    C7_4,
    C7_5,
    C7_6,
    C7_7,
    C7_8,
    C7_9,
    C7_10,
    C7_11,
    C7_12,
    C7_13,
    C7_14,
    C7_15,
    C7_16,

    C8_1,
    C8_2,
    C8_3,
    C8_4,
    C8_5,
    C8_6,
    C8_7,
    C8_8,
    C8_9,
    C8_10,
    C8_11,
    C8_12,
    C8_13,
    C8_14,
    C8_15,
    C8_16,

    C9_1,
    C9_2,
    C9_3,
    C9_4,
    C9_5,
    C9_6,
    C9_7,
    C9_8,
    C9_9,
    C9_10,
    C9_11,
    C9_12,
    C9_13,
    C9_14,
    C9_15,
    C9_16,
}

#[cfg(feature = "_backlight")]
/// IS31FL3731 backlight driver implementations
pub mod backlight {
    use is31fl3731::{gamma, Error, IS31FL3731};

    use core::fmt::Debug;
    use embassy_time::Delay;
    use embedded_hal_async::i2c::I2c;
    use smart_leds::RGB8;

    use crate::backlight::drivers::SimpleBacklightDriver;
    use crate::backlight::drivers::{RGBBacklightMatrixDriver, SimpleBacklightMatrixDriver};
    use crate::backlight::BacklightMatrixDevice;

    /// A trait that keyboards must implement to use the IS31FL3731 driver for backlighting.
    pub trait IS31FL3731BacklightDriver: BacklightMatrixDevice {
        /// I2C Address for the IS31FL3731. Consult the datasheet for more information.
        const LED_DRIVER_ADDR: u32;

        /// Convert matrix coordinates in the form of (col, row) to an IS31FL3731 [`Position`](super::Position).
        ///
        /// It is recommended to use [`is31fl3731_get_led_from_matrix_coordinates`] to implement this function.
        fn get_led_from_matrix_coordinates(x: u8, y: u8) -> u8;

        /// Setup the I2C peripheral to communicate with the IS31FL3731 chip.
        ///
        /// It is recommended to use [`crate::hw::mcu::setup_i2c`] to implement this function.
        fn setup_i2c() -> impl I2c<Error = impl Debug>;
    }

    #[macro_export]
    macro_rules! is31fl3731_get_led_from_rgb_matrix_coordinates {
        ([] -> [$($body:tt)*]) => {
            [$($body)*]
        };
        ([No $($rest:tt)*] -> [$($body:tt)*]) => {
            is31fl3731_get_led_from_rgb_matrix_coordinates!([$($rest)*] -> [$($body)* 255,])
        };
        ([$pos:ident $($rest:tt)*] -> [$($body:tt)*]) => {
            is31fl3731_get_led_from_rgb_matrix_coordinates!([$($rest)*] -> [$($body)* $crate::drivers::is31fl3731::Position::$pos as u8,])
        };
        ({$([$($r_pos:ident)*])*} {$([$($g_pos:ident)*])*} {$([$($b_pos:ident)*])*}) => {
            fn get_led_from_matrix_coordinates(x: u8, y: u8) -> u8 {
                let lookup: [[u8; { Self::LIGHTING_COLS * 3 }]; Self::LIGHTING_ROWS] = [
                    $(
                        is31fl3731_get_led_from_rgb_matrix_coordinates!([$($r_pos)* $($g_pos)* $($b_pos)*] -> [])
                    ),*
                ];

                lookup[y as usize][x as usize] as u8
            }
        };
    }

    #[macro_export]
    macro_rules! is31fl3731_get_led_from_matrix_coordinates {
        ([] -> [$($body:tt)*]) => {
            [$($body)*]
        };
        ([No $($rest:tt)*] -> [$($body:tt)*]) => {
            is31fl3731_get_led_from_matrix_coordinates!([$($rest)*] -> [$($body)* 255,])
        };
        ([$pos:ident $($rest:tt)*] -> [$($body:tt)*]) => {
            is31fl3731_get_led_from_matrix_coordinates!([$($rest)*] -> [$($body)* $crate::drivers::is31fl3731::Position::$pos as u8,])
        };
        ($([$($pos:ident)*])*) => {
            fn get_led_from_matrix_coordinates(x: u8, y: u8) -> u8 {
                let lookup: [[u8; Self::LIGHTING_COLS]; Self::LIGHTING_ROWS] = [
                    $(
                        is31fl3731_get_led_from_matrix_coordinates!([$($pos)*] -> [])
                    ),*
                ];

                lookup[y as usize][x as usize] as u8
            }
        };
    }

    /// Create an instance of the IS31FL3731 driver based on the implementation of [`IS31FL3731BacklightDriver`].
    pub async fn setup_backlight_driver<K: IS31FL3731BacklightDriver>(
    ) -> IS31FL3731<impl I2c<Error = impl Debug>> {
        let mut driver = IS31FL3731::new(
            K::setup_i2c(),
            K::LED_DRIVER_ADDR as u8,
            K::LIGHTING_COLS as u8,
            K::LIGHTING_ROWS as u8,
            K::get_led_from_matrix_coordinates,
        );

        driver.setup(&mut Delay).await.unwrap();

        driver
    }

    impl<I2CError: Debug, I2C: I2c<Error = I2CError>, K: IS31FL3731BacklightDriver>
        SimpleBacklightDriver<K> for IS31FL3731<I2C>
    {
        type DriverWriteError = Error<I2CError>;

        async fn write(&mut self, brightness: u8) -> Result<(), Self::DriverWriteError> {
            let payload = [gamma(brightness); 144];

            self.all_pixels(&payload).await?;

            Ok(())
        }

        type DriverEnableError = Error<I2CError>;

        async fn turn_on(&mut self) -> Result<(), Self::DriverEnableError> {
            self.sleep(false).await?;

            Ok(())
        }

        type DriverDisableError = Error<I2CError>;

        async fn turn_off(&mut self) -> Result<(), Self::DriverDisableError> {
            self.sleep(true).await?;

            Ok(())
        }
    }

    impl<I2CError: Debug, I2C: I2c<Error = I2CError>, K: IS31FL3731BacklightDriver>
        SimpleBacklightMatrixDriver<K> for IS31FL3731<I2C>
    {
        type DriverWriteError = Error<I2CError>;

        async fn write(
            &mut self,
            buf: &[[u8; K::LIGHTING_COLS]; K::LIGHTING_ROWS],
        ) -> Result<(), Self::DriverWriteError> {
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

        type DriverEnableError = Error<I2CError>;

        async fn turn_on(&mut self) -> Result<(), Self::DriverEnableError> {
            self.sleep(false).await?;

            Ok(())
        }

        type DriverDisableError = Error<I2CError>;

        async fn turn_off(&mut self) -> Result<(), Self::DriverDisableError> {
            self.sleep(true).await?;

            Ok(())
        }
    }

    impl<I2CError: Debug, I2C: I2c<Error = I2CError>, K: IS31FL3731BacklightDriver>
        RGBBacklightMatrixDriver<K> for IS31FL3731<I2C>
    {
        type DriverWriteError = Error<I2CError>;

        async fn write(
            &mut self,
            buf: &[[RGB8; K::LIGHTING_COLS]; K::LIGHTING_ROWS],
        ) -> Result<(), Self::DriverWriteError> {
            let mut payload = [0; 144];

            // Map the frame data to LED offsets and set the brightness of the LED in the payload
            for (row_num, row) in buf.iter().enumerate() {
                for (col_num, color) in row.iter().enumerate() {
                    for (component, val) in color.iter().enumerate() {
                        let offset = K::get_led_from_matrix_coordinates(
                            col_num as u8 + (component * K::LIGHTING_COLS) as u8,
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

        type DriverEnableError = Error<I2CError>;

        async fn turn_on(&mut self) -> Result<(), Self::DriverEnableError> {
            self.sleep(false).await?;

            Ok(())
        }

        type DriverDisableError = Error<I2CError>;

        async fn turn_off(&mut self) -> Result<(), Self::DriverDisableError> {
            self.sleep(true).await?;

            Ok(())
        }
    }
}
