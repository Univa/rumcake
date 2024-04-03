use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::Expr;

crate::parse_as_custom_fields! {
    pub struct Ssd1306ArgsBuilder for Ssd1306Args {
        i2c: Expr,
        size: Ident,
        rotation: Ident,
    }
}

pub fn setup_ssd1306(
    Ssd1306Args {
        i2c,
        size,
        rotation,
    }: Ssd1306Args,
) -> TokenStream {
    quote! {
        ::rumcake::drivers::ssd1306::setup_driver(
            #i2c,
            ::rumcake::drivers::ssd1306::driver::size::#size,
            ::rumcake::drivers::ssd1306::driver::rotation::DisplayRotation::#rotation
        )
    }
}
