use darling::FromMeta;
use heck::{ToShoutySnakeCase, ToSnakeCase};
use proc_macro2::{Ident, Literal, TokenStream, TokenTree};
use proc_macro_error::proc_macro_error;
use quote::{format_ident, quote, quote_spanned, ToTokens};
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
    parenthesized, parse_macro_input, parse_quote, parse_str, DeriveInput, ItemEnum, ItemFn,
    ItemStruct, LitInt, LitStr, Meta, Pat, Token,
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

#[derive(Debug)]
struct TuplePair {
    parenthesis_token: syn::token::Paren,
    left: LitInt,
    comma_token: Token![,],
    right: LitInt,
}

impl Parse for TuplePair {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        let parenthesis_token = parenthesized!(content in input);
        Ok(Self {
            parenthesis_token,
            left: content.parse()?,
            comma_token: content.parse()?,
            right: content.parse()?,
        })
    }
}

impl ToTokens for TuplePair {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let left = &self.left;
        let right = &self.right;
        quote! { (#left,#right) }.to_tokens(tokens)
    }
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
pub fn build_standard_matrix(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let matrix = parse_macro_input!(input as keyboard::StandardMatrixDefinition);
    keyboard::build_standard_matrix(matrix).into()
}

#[proc_macro]
pub fn build_direct_pin_matrix(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let matrix = parse_macro_input!(input as keyboard::MatrixLike<keyboard::OptionalItem<Ident>>);
    keyboard::build_direct_pin_matrix(matrix).into()
}

#[proc_macro]
pub fn build_analog_matrix(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let matrix = parse_macro_input!(input as keyboard::AnalogMatrixDefinition);
    keyboard::build_analog_matrix(matrix).into()
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
        parse_macro_input!(input as keyboard::MatrixLike<keyboard::OptionalItem<TuplePair>>);
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

pub(crate) mod common {
    use proc_macro2::Ident;
    use syn::parse::Parse;
    use syn::{braced, custom_keyword, Token};

    custom_keyword!(Multiplexer);
    custom_keyword!(Direct);
    custom_keyword!(pin);
    custom_keyword!(select_pins);

    #[allow(dead_code)]
    pub struct MultiplexerDefinition {
        pub multiplexer_field_name: Multiplexer,
        pub pin_brace_token: syn::token::Brace,
        pub pin_field_name: pin,
        pub pin_field_colon_token: Token![:],
        pub pin: Ident,
        pub select_pins_field_name: select_pins,
        pub select_pins_field_colon_token: Token![:],
        pub select_pins_brace_token: syn::token::Brace,
        pub select_pins: Vec<crate::keyboard::OptionalItem<Ident>>,
    }

    impl Parse for MultiplexerDefinition {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            let content;
            let select_pins_content;
            Ok(Self {
                multiplexer_field_name: input.parse()?,
                pin_brace_token: braced!(content in input),
                pin_field_name: content.parse()?,
                pin_field_colon_token: content.parse()?,
                pin: content.parse()?,
                select_pins_field_name: content.parse()?,
                select_pins_field_colon_token: content.parse()?,
                select_pins_brace_token: braced!(select_pins_content in content),
                select_pins: {
                    let mut pins = Vec::new();
                    while let Ok(t) = select_pins_content.parse() {
                        pins.push(t)
                    }
                    if !select_pins_content.is_empty() {
                        return Err(syn::Error::new(
                            select_pins_content.span(),
                            "Encountered an invalid token.",
                        ));
                    }
                    pins
                },
            })
        }
    }

    #[allow(dead_code)]
    pub struct DirectPinDefinition {
        pub direct_field_name: Direct,
        pub brace_token: syn::token::Brace,
        pub pin_field_name: pin,
        pub colon_token: Token![:],
        pub pin: Ident,
    }

    impl Parse for DirectPinDefinition {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            let content;
            Ok(Self {
                direct_field_name: input.parse()?,
                brace_token: braced!(content in input),
                pin_field_name: content.parse()?,
                colon_token: content.parse()?,
                pin: content.parse()?,
            })
        }
    }

    pub enum AnalogPinType {
        Multiplexed(MultiplexerDefinition),
        Direct(DirectPinDefinition),
    }

    impl Parse for AnalogPinType {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            let lookahead = input.lookahead1();
            if lookahead.peek(Direct) {
                input.parse().map(AnalogPinType::Direct)
            } else if lookahead.peek(Multiplexer) {
                input.parse().map(AnalogPinType::Multiplexed)
            } else {
                Err(lookahead.error())
            }
        }
    }
}

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

#[proc_macro]
#[proc_macro_error]
pub fn setup_adc_sampler(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    #[cfg(feature = "stm32")]
    let channels = parse_macro_input!(input with Punctuated<hw::STM32AdcSamplerDefinition, Token![,]>::parse_terminated);

    #[cfg(feature = "nrf")]
    let channels = parse_macro_input!(input as hw::NrfAdcSamplerDefinition);

    #[cfg(feature = "rp")]
    let channels = parse_macro_input!(input with Punctuated<common::AnalogPinType, Token![,]>::parse_terminated);

    hw::setup_adc_sampler(channels).into()
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
