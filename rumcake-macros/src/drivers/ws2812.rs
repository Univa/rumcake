use crate::common::{MatrixLike, OptionalItem};
use proc_macro2::{Literal, TokenStream};
use quote::quote;

pub fn get_led_from_matrix_coordinates(input: MatrixLike<OptionalItem<Literal>>) -> TokenStream {
    let values = input.rows.iter().map(|row| {
        let items = &row.items;
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
    use proc_macro_error::abort;
    use quote::quote;
    use syn::LitInt;

    crate::parse_as_custom_fields! {
        pub struct WS2812BitbangArgsBuilder for WS2812BitbangArgs {
            pin: Ident,
            fudge: Option<LitInt>
        }
    }

    pub fn setup_ws2812_bitbang(
        WS2812BitbangArgs { pin, fudge }: WS2812BitbangArgs,
    ) -> TokenStream {
        let fudge = if let Some(lit) = fudge {
            lit.base10_parse::<u8>().unwrap_or_else(|_| {
                abort!(
                    lit,
                    "The provided fudge value could not be parsed as a u8 value."
                )
            })
        } else {
            60
        };

        quote! {
            ::rumcake::drivers::ws2812_bitbang::setup_driver::<{ ::rumcake::hw::platform::SYSCLK }, #fudge>(::rumcake::hw::platform::output_pin!(#pin))
        }
    }
}
