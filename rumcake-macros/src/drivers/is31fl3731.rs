use crate::common::{Layer, MatrixLike, OptionalItem};
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{Expr, LitInt};

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
        let items = row.items.iter().map(render_optional_item_to_led);
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

crate::parse_as_custom_fields! {
    pub struct IS31FL3731RgbMatrixLedArgsBuilder for IS31FL3731RgbMatrixLedArgs {
        red: Layer<OptionalItem<Ident>>,
        green: Layer<OptionalItem<Ident>>,
        blue: Layer<OptionalItem<Ident>>,
    }
}

pub fn get_led_from_rgb_matrix_coordinates(
    IS31FL3731RgbMatrixLedArgs { red, green, blue }: IS31FL3731RgbMatrixLedArgs,
) -> TokenStream {
    let red_values = red.layer.rows.iter();
    let green_values = green.layer.rows.iter();
    let blue_values = blue.layer.rows.iter();

    let rows =
        red_values
            .zip(green_values)
            .zip(blue_values)
            .map(|((red_row, green_row), blue_row)| {
                let red_leds = red_row.items.iter().map(render_optional_item_to_led);
                let green_leds = green_row.items.iter().map(render_optional_item_to_led);
                let blue_leds = blue_row.items.iter().map(render_optional_item_to_led);
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

crate::parse_as_custom_fields! {
    pub struct IS31FL3731ArgsBuilder for IS31FL3731Args {
        device: Ident,
        i2c: Expr,
        address: LitInt,
    }
}

pub fn setup_is31fl3731(
    IS31FL3731Args {
        device,
        i2c,
        address,
    }: IS31FL3731Args,
) -> TokenStream {
    quote! {
        ::rumcake::drivers::is31fl3731::setup_driver(
            #i2c,
            #address,
            <#device as ::rumcake::lighting::BacklightMatrixDevice>::LIGHTING_COLS as u8,
            <#device as ::rumcake::lighting::BacklightMatrixDevice>::LIGHTING_ROWS as u8,
            <#device as ::rumcake::drivers::is31fl3731::IS31FL3731BacklightDriver>::get_led_from_matrix_coordinates
        ).await
    }
}
