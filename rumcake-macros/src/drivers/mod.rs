use proc_macro2::TokenStream;
use quote::quote;

pub mod is31fl3731;
pub mod nrf_ble;
pub mod ssd1306;
pub mod ws2812;

pub fn serial_driver_trait() -> TokenStream {
    quote! {
        /// A trait that must be implemented to set up the IS31FL3731 driver.
        pub(crate) trait SerialDriverSettings {
            /// Setup a serial driver that is capable of both reading and writing.
            ///
            /// It is recommended to use a macro to implement this function.
            fn setup_serial() -> impl ::rumcake::embedded_io_async::Write + ::rumcake::embedded_io_async::Read;
        }
    }
}
