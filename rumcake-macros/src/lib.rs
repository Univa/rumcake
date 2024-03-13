use darling::FromMeta;
use heck::{ToShoutySnakeCase, ToSnakeCase};
use proc_macro2::{Ident, Literal, TokenStream, TokenTree};
use proc_macro_error::proc_macro_error;
use quote::{format_ident, quote, quote_spanned};
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
    parse_macro_input, parse_quote, parse_str, DeriveInput, ExprTuple, ItemEnum, ItemFn,
    ItemStruct, LitStr, Meta, Pat, Token,
};

struct Templates(Punctuated<LitStr, Token![,]>);

impl Parse for Templates {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Templates(
            Punctuated::<LitStr, Token![,]>::parse_separated_nonempty(input)?,
        ))
    }
}

fn process_template(template: &str, name: &str) -> String {
    template
        .replace("{variant}", name)
        .replace("{variant_snake_case}", &name.to_snake_case())
        .replace("{variant_shouty_snake_case}", &name.to_shouty_snake_case())
}

#[proc_macro_attribute]
pub fn generate_items_from_enum_variants(
    a: proc_macro::TokenStream,
    e: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args = parse_macro_input!(a as Templates).0;
    let mut item = parse_macro_input!(e as ItemEnum);
    let enum_name = item.ident.clone().to_string().to_snake_case();
    let macro_name = format_ident!("{}_items", enum_name);

    let members: Vec<TokenStream> = args
        .iter()
        .flat_map(|t| {
            item.variants
                .iter_mut()
                .flat_map(|variant| -> Vec<TokenStream> {
                    let mut streams: Vec<TokenStream> = Vec::new();
                    let variant_name = variant.ident.to_string();

                    let rendered = process_template(&t.value(), &variant_name);

                    // Generate variant-specific items
                    if let Some(idx) = variant
                        .attrs
                        .iter()
                        .position(|v| v.path().is_ident("generate_items"))
                    {
                        if let Meta::List(list) = variant.attrs.remove(idx).meta.clone() {
                            let tokens: proc_macro::TokenStream = list.tokens.clone().into();
                            match syn::parse::<Templates>(tokens) {
                                Ok(data) => {
                                    data.0.iter().for_each(|t| {
                                        streams.push(
                                            parse_str(&process_template(
                                                &t.value(),
                                                &variant_name.clone(),
                                            ))
                                            .unwrap(),
                                        );
                                    });
                                }
                                Err(_err) => streams.push(quote_spanned! {
                                    list.span() => compile_error!("Could not parse item.")
                                }),
                            };
                        };
                    };

                    streams.push(parse_str(&rendered).unwrap());

                    streams
                })
                .collect::<Vec<TokenStream>>()
        })
        .collect();

    quote! {
        #item

        macro_rules! #macro_name {
            () => {
                #(#members;)*
            }
        }

        pub(crate) use #macro_name;
    }
    .into()
}

mod derive;

#[proc_macro_derive(LEDEffect, attributes(animated, reactive))]
pub fn derive_ledeffect(e: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let item = parse_macro_input!(e as DeriveInput);
    derive::ledeffect(item)
}

#[proc_macro_derive(Cycle)]
pub fn derive_cycle(e: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let item = parse_macro_input!(e as DeriveInput);
    derive::cycle(item)
}

mod keyboard;

#[proc_macro_attribute]
pub fn keyboard_main(
    args: proc_macro::TokenStream,
    str: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let str = parse_macro_input!(str as ItemStruct);
    let kb_name = str.ident.clone();
    let args = darling::ast::NestedMeta::parse_meta_list(args.into()).unwrap();

    keyboard::keyboard_main(
        str,
        kb_name,
        keyboard::KeyboardSettings::from_list(&args).unwrap(),
    )
    .into()
}

#[proc_macro]
pub fn build_matrix(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let matrix = parse_macro_input!(input as keyboard::MatrixDefinition<Ident>);
    keyboard::build_matrix(matrix).into()
}

#[proc_macro]
pub fn build_layout(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let raw = input.clone();
    let layers = parse_macro_input!(input as keyboard::LayoutLike<TokenTree>);
    keyboard::build_layout(raw.into(), layers).into()
}

#[proc_macro]
pub fn remap_matrix(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let remap = parse_macro_input!(input as keyboard::RemapMacroInput);
    keyboard::remap_matrix(remap).into()
}

mod backlight;

#[proc_macro]
#[proc_macro_error]
pub fn setup_backlight_matrix(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let matrix = parse_macro_input!(input as backlight::BacklightMatrixMacroInput);
    backlight::setup_backlight_matrix(matrix).into()
}

#[proc_macro]
#[proc_macro_error]
pub fn led_layout(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let matrix =
        parse_macro_input!(input as keyboard::MatrixLike<keyboard::OptionalItem<ExprTuple>>);
    backlight::led_layout(matrix).into()
}

#[proc_macro]
pub fn led_flags(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let matrix = parse_macro_input!(
        input as keyboard::MatrixLike<keyboard::OptionalItem<backlight::LEDFlags>>
    );
    backlight::led_flags(matrix).into()
}

mod drivers;

#[proc_macro]
pub fn ws2812_bitbang_pin(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let pin = parse_macro_input!(input as Ident);
    drivers::ws2812::bitbang::pin(pin).into()
}

#[proc_macro]
pub fn ws2812_get_led_from_matrix_coordinates(
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let matrix = parse_macro_input!(input as keyboard::MatrixLike<keyboard::OptionalItem<Literal>>);
    drivers::ws2812::get_led_from_matrix_coordinates(matrix).into()
}

#[proc_macro]
pub fn is31fl3731_get_led_from_matrix_coordinates(
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let matrix = parse_macro_input!(input as keyboard::MatrixLike<keyboard::OptionalItem<Ident>>);
    drivers::is31fl3731::get_led_from_matrix_coordinates(matrix).into()
}

#[proc_macro]
#[proc_macro_error]
pub fn is31fl3731_get_led_from_rgb_matrix_coordinates(
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let layout = parse_macro_input!(input as keyboard::LayoutLike<keyboard::OptionalItem<Ident>>);
    drivers::is31fl3731::get_led_from_rgb_matrix_coordinates(layout).into()
}

#[cfg_attr(feature = "stm32", path = "hw/stm32.rs")]
#[cfg_attr(feature = "nrf", path = "hw/nrf.rs")]
#[cfg_attr(feature = "rp", path = "hw/rp.rs")]
mod hw;

#[proc_macro]
pub fn input_pin(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ident = parse_macro_input!(input as Ident);
    hw::input_pin(ident).into()
}

#[proc_macro]
pub fn output_pin(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ident = parse_macro_input!(input as Ident);
    hw::output_pin(ident).into()
}

#[proc_macro]
pub fn setup_i2c(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args = parse_macro_input!(input with Punctuated<Ident, Token![,]>::parse_terminated);
    hw::setup_i2c(args).into()
}

#[cfg(feature = "nrf")]
#[proc_macro]
pub fn setup_i2c_blocking(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ident = parse_macro_input!(input with Punctuated<Ident, Token![,]>::parse_terminated);
    hw::setup_i2c_blocking(ident).into()
}

#[cfg(feature = "nrf")]
#[proc_macro]
pub fn setup_buffered_uarte(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ident = parse_macro_input!(input with Punctuated<Ident, Token![,]>::parse_terminated);
    hw::setup_buffered_uarte(ident).into()
}

#[cfg(any(feature = "stm32", feature = "rp"))]
#[proc_macro]
pub fn setup_buffered_uart(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ident = parse_macro_input!(input with Punctuated<Ident, Token![,]>::parse_terminated);
    hw::setup_buffered_uart(ident).into()
}

#[cfg(feature = "rp")]
#[proc_macro]
pub fn setup_dma_channel(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args = parse_macro_input!(input as Ident);
    hw::setup_dma_channel(args).into()
}

mod via;

#[proc_macro]
pub fn setup_macro_buffer(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args = parse_macro_input!(input with Punctuated<Literal, Token![,]>::parse_terminated);
    via::setup_macro_buffer(args).into()
}

mod vial;

#[proc_macro]
pub fn enable_vial_rgb(_input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    vial::enable_vial_rgb().into()
}

#[proc_macro_attribute]
pub fn task(
    _args: proc_macro::TokenStream,
    fun: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let fun = parse_macro_input!(fun as ItemFn);

    // for the outer macro
    let task_ident = fun.sig.ident.clone();

    // Copy the function and change the identifier
    let mut inner = fun.clone();
    let task_name = inner.sig.ident;
    inner.sig.ident = format_ident!("__{}", task_name);

    let task_name_string = task_name.to_string();
    inner.block.stmts.insert(
        0,
        parse_quote! {
            defmt::info!("{} has spawned.", #task_name_string);
        },
    );
    let inner_ident = inner.sig.ident.clone();

    // Arguments to pass to the inner task
    let arg_names: Vec<Ident> = fun
        .sig
        .inputs
        .clone()
        .iter_mut()
        .filter_map(|a| match a {
            syn::FnArg::Typed(t) => match t.pat.as_mut() {
                Pat::Ident(i) => Some(i.ident.clone()),
                _ => None,
            },
            _ => None,
        })
        .collect();

    quote! {
        #inner

        #[macro_export]
        macro_rules! #task_ident {
            (#($#arg_names:expr),*) => {
                {
                    type Fut = impl ::core::future::Future + 'static;
                    static POOL: ::embassy_executor::raw::TaskPool<Fut, 1> = ::embassy_executor::raw::TaskPool::new();
                    unsafe { POOL._spawn_async_fn(move || $crate::tasks::#inner_ident(#($#arg_names,)*)) }
                }
            };
        }
    }
    .into()
}
