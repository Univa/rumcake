use proc_macro2::TokenStream;
use proc_macro_error::OptionExt;
use quote::{quote, ToTokens};
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::{Ident, Token};

use crate::common::{Layer, MatrixLike, OptionalItem};
use crate::TuplePair;

pub fn led_layout(input: MatrixLike<OptionalItem<TuplePair>>) -> TokenStream {
    let coordinates = input.rows.iter().map(|row| {
        let items = &row.items;

        quote! { #(#items),* }
    });

    quote! {
        [
            #([ #coordinates ]),*
        ]
    }
}

#[derive(Debug)]
pub struct LEDFlags {
    pub flags: Punctuated<Ident, Token![|]>,
}

impl Parse for LEDFlags {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(LEDFlags {
            flags: input.call(Punctuated::parse_separated_nonempty)?,
        })
    }
}

impl ToTokens for LEDFlags {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let flags = self.flags.iter();

        quote! { #(::rumcake::lighting::LEDFlags::#flags)|* }.to_tokens(tokens)
    }
}

pub fn led_flags(input: MatrixLike<OptionalItem<LEDFlags>>) -> TokenStream {
    let flags = input.rows.iter().map(|row| {
        let items = row.items.iter().map(|col| match col {
            OptionalItem::None => quote! {
                ::rumcake::lighting::LEDFlags::NONE
            },
            OptionalItem::Some(ident) => quote! {
                #ident
            },
        });

        quote! { #(#items),* }
    });

    quote! {
        [
            #([ #flags ]),*
        ]
    }
}

crate::parse_as_custom_fields! {
    pub struct BacklightMatrixMacroInputBuilder for BacklightMatrixMacroInput {
        pub led_layout: Layer<OptionalItem<TuplePair>>,
        pub led_flags: Layer<OptionalItem<LEDFlags>>,
    }
}

pub fn setup_backlight_matrix(
    BacklightMatrixMacroInput {
        led_layout,
        led_flags,
    }: BacklightMatrixMacroInput,
) -> TokenStream {
    let first_row = led_layout
        .layer
        .rows
        .first()
        .expect_or_abort("Expected at least one row to be defined.");

    let row_count = led_layout.layer.rows.len();
    let col_count = first_row.items.len();

    let led_layout = self::led_layout(led_layout.layer);
    let led_flags = self::led_flags(led_flags.layer);

    quote! {
        const LIGHTING_COLS: usize = #col_count;
        const LIGHTING_ROWS: usize = #row_count;

        fn get_backlight_matrix(
        ) -> ::rumcake::lighting::BacklightMatrix<{ Self::LIGHTING_COLS }, { Self::LIGHTING_ROWS }>
        {
            const BACKLIGHT_MATRIX: ::rumcake::lighting::BacklightMatrix<#col_count, #row_count> =
                ::rumcake::lighting::BacklightMatrix::new(#led_layout, #led_flags);
            BACKLIGHT_MATRIX
        }
    }
}
