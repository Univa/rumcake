use crate::keyboard::{LayoutLike, MatrixLike, OptionalItem};
use proc_macro2::{Ident, TokenStream};
use proc_macro_error::OptionExt;
use quote::quote;

fn render_optional_item_to_led(item: &OptionalItem<Ident>) -> TokenStream {
    match item {
        OptionalItem::None => quote! { 255 },
        OptionalItem::Some(ident) => {
            quote! { ::rumcake::drivers::is31fl3731::Position::#ident as u8 }
        }
    }
}

pub fn get_led_from_matrix_coordinates(input: MatrixLike<OptionalItem<Ident>>) -> TokenStream {
    let values = input.rows.iter().map(|row| {
        let items = row.cols.iter().map(render_optional_item_to_led);
        quote! { #(#items),* }
    });

    quote! {
        fn get_led_from_matrix_coordinates(x: u8, y: u8) -> u8 {
            let lookup: [[u8; Self::LIGHTING_COLS]; Self::LIGHTING_ROWS] = [
                #([ #values ]),*
            ];

            lookup[y as usize][x as usize]
        }
    }
}

pub fn get_led_from_rgb_matrix_coordinates(input: LayoutLike<OptionalItem<Ident>>) -> TokenStream {
    // let convert_to_
    let red_values = input
        .layers
        .first()
        .expect_or_abort("Red LED positions not specified.")
        .layer
        .rows
        .iter();

    let green_values = input
        .layers
        .get(1)
        .expect_or_abort("Green LEDs positions not specified.")
        .layer
        .rows
        .iter();

    let blue_values = input
        .layers
        .get(2)
        .expect_or_abort("Blue LEDs positions not specified.")
        .layer
        .rows
        .iter();

    let rows =
        red_values
            .zip(green_values)
            .zip(blue_values)
            .map(|((red_row, green_row), blue_row)| {
                let red_leds = red_row.cols.iter().map(render_optional_item_to_led);
                let green_leds = green_row.cols.iter().map(render_optional_item_to_led);
                let blue_leds = blue_row.cols.iter().map(render_optional_item_to_led);
                quote! { #(#red_leds),*, #(#green_leds),*, #(#blue_leds),* }
            });

    quote! {
        fn get_led_from_matrix_coordinates(x: u8, y: u8) -> u8 {
            let lookup: [[u8; { Self::LIGHTING_COLS * 3 }]; Self::LIGHTING_ROWS] = [
                #([ #rows ]),*
            ];

            lookup[y as usize][x as usize]
        }
    }
}
