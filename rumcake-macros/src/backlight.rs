use proc_macro2::TokenStream;
use proc_macro_error::{abort, OptionExt};
use quote::{quote, ToTokens};
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{braced, ExprTuple, Ident, Token};

use crate::keyboard::{MatrixLike, OptionalItem};

pub fn led_layout(input: MatrixLike<OptionalItem<ExprTuple>>) -> TokenStream {
    let coordinates = input.rows.iter().map(|row| {
        let items = &row.cols;

        if let Some(item) = items.iter().find(|item| match item {
            OptionalItem::None => false,
            OptionalItem::Some(tuple) => tuple.elems.len() != 2,
        }) {
            abort!(item.span(), "Item is not a coordinate.")
        };

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
            flags: input.parse_terminated(Ident::parse, Token![|])?,
        })
    }
}

impl ToTokens for LEDFlags {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        for flag in self.flags.iter() {
            quote! { ::rumcake::backlight::LEDFlags::#flag }.to_tokens(tokens)
        }
    }
}

pub fn led_flags(input: MatrixLike<OptionalItem<LEDFlags>>) -> TokenStream {
    let flags = input.rows.iter().map(|row| {
        let items = row.cols.iter().map(|col| match col {
            OptionalItem::None => quote! {
                ::rumcake::backlight::LEDFlags::NONE
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

#[derive(Debug)]
pub struct BacklightMatrixMacroInput {
    pub led_layout_brace: syn::token::Brace,
    pub led_layout: MatrixLike<OptionalItem<ExprTuple>>,
    pub led_flags_brace: syn::token::Brace,
    pub led_flags: MatrixLike<OptionalItem<LEDFlags>>,
}

impl Parse for BacklightMatrixMacroInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let led_layout_content;
        let led_layout_brace = braced!(led_layout_content in input);
        let led_flags_content;
        let led_flags_brace = braced!(led_flags_content in input);
        Ok(BacklightMatrixMacroInput {
            led_layout_brace,
            led_layout: led_layout_content.parse()?,
            led_flags_brace,
            led_flags: led_flags_content.parse()?,
        })
    }
}

pub fn setup_backlight_matrix(input: BacklightMatrixMacroInput) -> TokenStream {
    let BacklightMatrixMacroInput {
        led_layout,
        led_flags,
        ..
    } = input;

    let first_row = led_layout
        .rows
        .first()
        .expect_or_abort("Expected at least one row to be defined.");

    let row_count = led_layout.rows.len();
    let col_count = first_row.cols.len();

    let led_layout = self::led_layout(led_layout);
    let led_flags = self::led_flags(led_flags);

    quote! {
        const LIGHTING_COLS: usize = #col_count;
        const LIGHTING_ROWS: usize = #row_count;

        fn get_backlight_matrix(
        ) -> ::rumcake::backlight::BacklightMatrix<{ Self::LIGHTING_COLS }, { Self::LIGHTING_ROWS }>
        {
            const BACKLIGHT_MATRIX: ::rumcake::backlight::BacklightMatrix<#col_count, #row_count> =
                ::rumcake::backlight::BacklightMatrix::new(#led_layout, #led_flags);
            BACKLIGHT_MATRIX
        }
    }
}
