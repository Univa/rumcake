use crate::keyboard::{MatrixLike, OptionalItem};
use proc_macro2::{Literal, TokenStream};
use quote::quote;

pub fn get_led_from_matrix_coordinates(input: MatrixLike<OptionalItem<Literal>>) -> TokenStream {
    let values = input.rows.iter().map(|row| {
        let items = &row.cols;
        quote! {#(#items),*}
    });

    quote! {
        fn get_led_from_matrix_coordinates(x: u8, y: u8) -> Option<u8> {
            let lookup: [[Option<u8>; Self::LIGHTING_COLS]; Self::LIGHTING_ROWS] = [
                #([ #values ]),*
            ];

            lookup[y as usize][x as usize]
        }
    }
}

pub mod bitbang {
    use proc_macro2::{Ident, TokenStream};
    use quote::quote;

    pub fn driver_trait() -> TokenStream {
        quote! {
            /// A trait that must be implemented to set up the WS2812 driver.
            pub(crate) trait WS2812BitbangDriverSettings {
                /// Setup the GPIO pin used to send data to the WS2812 LEDs.
                ///
                /// It is recommended to use
                /// [`rumcake::drivers::ws2812_bitbang::ws2812_bitbang_pin`] to implement this
                /// function.
                fn ws2812_pin() -> impl ::rumcake::embedded_hal::digital::v2::OutputPin;
            }
        }
    }

    pub fn pin(input: Ident) -> TokenStream {
        quote! {
            fn ws2812_pin() -> impl ::rumcake::embedded_hal::digital::v2::OutputPin {
                ::rumcake::hw::mcu::output_pin!(#input)
            }
        }
    }
}
