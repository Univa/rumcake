use proc_macro2::TokenStream;
use quote::quote;

pub fn driver_trait() -> TokenStream {
    quote! {
        /// A trait that keyboards must implement to set up the SSD1306 driver.
        pub(crate) trait Ssd1306I2cDriverSettings {
            /// Size of the display. Must be an implementor of [`DisplaySize`].
            type SIZE_TYPE: ::rumcake::drivers::ssd1306::driver::size::DisplaySize;

            /// Size of the display. Must be an implementor of [`DisplaySize`].
            const SIZE: Self::SIZE_TYPE;

            /// Rotation of the SSD1306 display. See [`DisplayRotation`].
            const ROTATION: ::rumcake::drivers::ssd1306::driver::rotation::DisplayRotation =
                ::rumcake::drivers::ssd1306::driver::rotation::DisplayRotation::Rotate90;

            /// Setup the I2C peripheral to communicate with the SSD1306 display.
            ///
            /// It is recommended to use [`rumcake::hw::mcu::setup_i2c`] to implement this function.
            fn setup_i2c() -> impl ::rumcake::embedded_hal::blocking::i2c::Write<Error = impl core::fmt::Debug>;
        }
    }
}
