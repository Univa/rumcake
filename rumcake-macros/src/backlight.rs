use proc_macro2::TokenStream;
use proc_macro_error::{abort, OptionExt};
use quote::{quote, ToTokens};
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{braced, Ident, Token};

use crate::keyboard::{MatrixLike, OptionalItem};
use crate::TuplePair;

pub fn led_layout(input: MatrixLike<OptionalItem<TuplePair>>) -> TokenStream {
    let coordinates = input.rows.iter().map(|row| {
        let items = &row.cols;

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

        quote! { #(::rumcake::backlight::LEDFlags::#flags)|* }.to_tokens(tokens)
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
    pub led_layout: MatrixLike<OptionalItem<TuplePair>>,
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
