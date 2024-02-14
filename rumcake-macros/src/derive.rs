use proc_macro2::{Ident, TokenStream};
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{parse_quote_spanned, DeriveInput};

pub fn ledeffect(item: DeriveInput) -> proc_macro::TokenStream {
    let enum_name = item.ident.clone();
    let (animated_results, reactive_results): (TokenStream, TokenStream) =
        if let syn::Data::Enum(e) = item.data {
            let mut animated_tokens = TokenStream::new();
            let mut reactive_tokens = TokenStream::new();

            for variant in e.variants.clone().iter() {
                let variant_name = variant.ident.clone();
                let (is_animated, is_reactive) =
                    variant.attrs.iter().fold((false, false), |mut acc, attr| {
                        if attr.path().is_ident("animated") {
                            acc.0 = true;
                        }
                        if attr.path().is_ident("reactive") {
                            acc.1 = true;
                        }
                        acc
                    });

                animated_tokens.extend(quote! {
                    #enum_name::#variant_name => #is_animated,
                });
                reactive_tokens.extend(quote! {
                    #enum_name::#variant_name => #is_reactive,
                })
            }

            (animated_tokens, reactive_tokens)
        } else {
            (
                quote_spanned! {
                    item.span() => _ => compile_error!("LEDEffect can only be derived on enums.")
                },
                TokenStream::new(),
            )
        };

    quote! {
        impl LEDEffect for #enum_name {
            fn is_animated(&self) -> bool {
                match self {
                    #animated_results
                }
            }

            fn is_reactive(&self) -> bool {
                match self {
                    #reactive_results
                }
            }
        }
    }
    .into()
}

pub fn cycle(item: DeriveInput) -> proc_macro::TokenStream {
    let enum_name = item.ident.clone();
    let idents = if let syn::Data::Enum(e) = item.data {
        e.variants
            .clone()
            .iter()
            .map(|v| v.ident.clone())
            .collect::<Vec<Ident>>()
    } else {
        vec![parse_quote_spanned! {
            item.span() => compile_error!("Cycle can only be derived on enums.")
        }]
    };

    let mut incremented = idents.clone();
    incremented.rotate_left(1);
    let mut decremented = idents.clone();
    decremented.rotate_right(1);

    quote! {
        impl Cycle for #enum_name {
            fn increment(&mut self) {
                *self = match self {
                    #(#enum_name::#idents => #enum_name::#incremented),*
                }
            }

            fn decrement(&mut self) {
                *self = match self {
                    #(#enum_name::#idents => #enum_name::#decremented),*
                }
            }
        }
    }
    .into()
}
