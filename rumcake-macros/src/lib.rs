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
#[proc_macro_error]
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
#[proc_macro_error]
pub fn build_standard_matrix(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let matrix = parse_macro_input!(input as keyboard::StandardMatrixDefinition);
    keyboard::build_standard_matrix(matrix).into()
}

#[proc_macro]
#[proc_macro_error]
pub fn build_direct_pin_matrix(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let matrix = parse_macro_input!(input as common::MatrixLike<common::OptionalItem<Ident>>);
    keyboard::build_direct_pin_matrix(matrix).into()
}

#[proc_macro]
#[proc_macro_error]
pub fn build_analog_matrix(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let matrix = parse_macro_input!(input as keyboard::AnalogMatrixDefinition);
    keyboard::build_analog_matrix(matrix).into()
}

#[proc_macro]
#[proc_macro_error]
pub fn build_layout(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let raw = input.clone();
    let layers = parse_macro_input!(input as common::LayoutLike<TokenTree>);
    keyboard::build_layout(raw.into(), layers).into()
}

#[proc_macro]
#[proc_macro_error]
pub fn setup_encoders(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args = parse_macro_input!(input with Punctuated<keyboard::EncoderDefinition, Token![,]>::parse_terminated);
    keyboard::setup_encoders(args).into()
}

#[proc_macro]
#[proc_macro_error]
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
pub fn led_layout(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let matrix = parse_macro_input!(input as common::MatrixLike<common::OptionalItem<TuplePair>>);
    backlight::led_layout(matrix).into()
}

#[proc_macro]
pub fn led_flags(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let matrix =
        parse_macro_input!(input as common::MatrixLike<common::OptionalItem<backlight::LEDFlags>>);
    backlight::led_flags(matrix).into()
}

mod drivers;

#[proc_macro]
#[proc_macro_error]
pub fn setup_ws2812_bitbang(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let matrix = parse_macro_input!(input as drivers::ws2812::bitbang::WS2812BitbangArgs);
    drivers::ws2812::bitbang::setup_ws2812_bitbang(matrix).into()
}

#[proc_macro]
pub fn ws2812_get_led_from_matrix_coordinates(
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let matrix = parse_macro_input!(input as common::MatrixLike<common::OptionalItem<Literal>>);
    drivers::ws2812::get_led_from_matrix_coordinates(matrix).into()
}

#[proc_macro]
#[proc_macro_error]
pub fn setup_is31fl3731(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args = parse_macro_input!(input as drivers::is31fl3731::IS31FL3731Args);
    drivers::is31fl3731::setup_is31fl3731(args).into()
}

#[proc_macro]
pub fn is31fl3731_get_led_from_matrix_coordinates(
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let layout = parse_macro_input!(input as common::MatrixLike<common::OptionalItem<Ident>>);
    drivers::is31fl3731::get_led_from_matrix_coordinates(layout).into()
}

#[proc_macro]
#[proc_macro_error]
pub fn is31fl3731_get_led_from_rgb_matrix_coordinates(
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let layout = parse_macro_input!(input as drivers::is31fl3731::IS31FL3731RgbMatrixLedArgs);
    drivers::is31fl3731::get_led_from_rgb_matrix_coordinates(layout).into()
}

#[proc_macro]
#[proc_macro_error]
pub fn setup_ssd1306(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args = parse_macro_input!(input as drivers::ssd1306::Ssd1306Args);
    drivers::ssd1306::setup_ssd1306(args).into()
}

#[proc_macro]
#[proc_macro_error]
pub fn setup_nrf_ble_split_central(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args = parse_macro_input!(input as drivers::nrf_ble::NrfBleCentralArgs);
    drivers::nrf_ble::setup_nrf_ble_split_central(args).into()
}

#[proc_macro]
#[proc_macro_error]
pub fn setup_nrf_ble_split_peripheral(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args = parse_macro_input!(input as drivers::nrf_ble::NrfBlePeripheralArgs);
    drivers::nrf_ble::setup_nrf_ble_split_peripheral(args).into()
}

#[cfg_attr(feature = "stm32", path = "hw/stm32.rs")]
#[cfg_attr(feature = "nrf", path = "hw/nrf.rs")]
#[cfg_attr(feature = "rp", path = "hw/rp.rs")]
mod hw;

#[cfg(feature = "stm32")]
#[proc_macro]
#[proc_macro_error]
pub fn stm32_input_pin(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args = parse_macro_input!(input with Punctuated<Ident, Token![,]>::parse_terminated);
    hw::input_pin(args).into()
}

#[cfg(feature = "stm32")]
#[proc_macro]
pub fn stm32_output_pin(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ident = parse_macro_input!(input as Ident);
    hw::output_pin(ident).into()
}

#[cfg(feature = "stm32")]
#[proc_macro]
#[proc_macro_error]
pub fn stm32_setup_buffered_uart(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ident = parse_macro_input!(input as hw::BufferedUartArgs);
    hw::setup_buffered_uart(ident).into()
}

#[cfg(feature = "stm32")]
#[proc_macro]
#[proc_macro_error]
pub fn stm32_setup_adc_sampler(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let channels = parse_macro_input!(input with Punctuated<hw::STM32AdcSamplerDefinition, Token![,]>::parse_terminated);
    hw::setup_adc_sampler(channels).into()
}

#[cfg(feature = "stm32")]
#[proc_macro]
#[proc_macro_error]
pub fn stm32_setup_i2c(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args = parse_macro_input!(input as hw::I2cArgs);
    hw::setup_i2c(args).into()
}

#[cfg(feature = "rp")]
#[proc_macro]
pub fn rp_input_pin(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ident = parse_macro_input!(input as Ident);
    hw::input_pin(ident).into()
}

#[cfg(feature = "rp")]
#[proc_macro]
pub fn rp_output_pin(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ident = parse_macro_input!(input as Ident);
    hw::output_pin(ident).into()
}

#[cfg(feature = "rp")]
#[proc_macro]
#[proc_macro_error]
pub fn rp_setup_buffered_uart(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ident = parse_macro_input!(input as hw::BufferedUartArgs);
    hw::setup_buffered_uart(ident).into()
}

#[cfg(feature = "rp")]
#[proc_macro]
#[proc_macro_error]
pub fn rp_setup_adc_sampler(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let channels = parse_macro_input!(input with Punctuated<common::AnalogPinType, Token![,]>::parse_terminated);
    hw::setup_adc_sampler(channels).into()
}

#[cfg(feature = "rp")]
#[proc_macro]
#[proc_macro_error]
pub fn rp_setup_i2c(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args = parse_macro_input!(input as hw::I2cArgs);
    hw::setup_i2c(args).into()
}

#[cfg(feature = "nrf")]
#[proc_macro]
pub fn nrf_input_pin(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ident = parse_macro_input!(input as Ident);
    hw::input_pin(ident).into()
}

#[cfg(feature = "nrf")]
#[proc_macro]
pub fn nrf_output_pin(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ident = parse_macro_input!(input as Ident);
    hw::output_pin(ident).into()
}

#[cfg(feature = "nrf")]
#[proc_macro]
#[proc_macro_error]
pub fn nrf_setup_adc_sampler(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let channels = parse_macro_input!(input as hw::NrfAdcSamplerDefinition);
    hw::setup_adc_sampler(channels).into()
}

#[cfg(feature = "nrf")]
#[proc_macro]
#[proc_macro_error]
pub fn nrf_setup_buffered_uarte(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ident = parse_macro_input!(input as hw::BufferedUarteArgs);
    hw::setup_buffered_uarte(ident).into()
}

#[cfg(feature = "nrf")]
#[proc_macro]
#[proc_macro_error]
pub fn nrf_setup_i2c(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args = parse_macro_input!(input as hw::I2cArgs);
    hw::setup_i2c(args).into()
}

mod via;

#[proc_macro]
#[proc_macro_error]
pub fn setup_macro_buffer(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args = parse_macro_input!(input as via::MacroBufferArgs);
    via::setup_macro_buffer(args).into()
}

#[proc_macro]
pub fn connect_storage_service(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ident = parse_macro_input!(input as Ident);
    via::connect_storage_service(ident).into()
}

macro_rules! parse_as_custom_fields {
    ($str_vis:vis struct $builder_name:ident for $str_name:ident { $($all:tt)* }) => {
        $crate::parse_as_custom_fields!($str_vis struct $builder_name for $str_name [$($all)*] -> []);
    };
    ($str_vis:vis struct $builder_name:ident for $str_name:ident [$vis:vis $field_name:ident: Option<$type:ty> $(, $($rest:tt)*)? ] -> [$($processed:tt)*]) => {
        $crate::parse_as_custom_fields!($str_vis struct $builder_name for $str_name [$($($rest)*)?] -> [$($processed)* $vis $field_name: (Some(None), Option<$type>),]);
    };
    ($str_vis:vis struct $builder_name:ident for $str_name:ident [$vis:vis $field_name:ident: $type:ty $(, $($rest:tt)*)? ] -> [$($processed:tt)*]) => {
        $crate::parse_as_custom_fields!($str_vis struct $builder_name for $str_name [$($($rest)*)?] -> [$($processed)* $vis $field_name: (None, $type),]);
    };
    ($str_vis:vis struct $builder_name:ident for $str_name:ident [] -> [$($vis:vis $field_name:ident: ($default:expr, $($type:tt)*)),*,]) => {
        $str_vis struct $str_name {
            $($vis $field_name: $($type)*),*
        }

        struct $builder_name {
            $($field_name: Option<$($type)*>),*
        }

        impl $builder_name {
            fn new() -> Self {
                Self {
                    $($field_name: $default),*
                }
            }

            fn build(self) -> $str_name {
                $(
                    let $field_name = proc_macro_error::OptionExt::expect_or_abort(self.$field_name, stringify!($field_name field is missing));
                )*

                $str_name {
                    $($field_name),*
                }
            }
        }

        impl syn::parse::Parse for $str_name {
            fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
                let mut args: $builder_name = $builder_name::new();

                loop {
                    if input.is_empty() {
                        break;
                    }
                    let ident: Ident = input.parse()?;
                    let _colon: syn::Token![:] = input.parse()?;
                    match ident.to_string().as_str() {
                        $(stringify!($field_name) => args.$field_name = Some(input.parse()?)),*,
                        _ => return Err(syn::Error::new(input.span(), "unknown field encountered."))
                    }
                    if input.is_empty() {
                        break;
                    }
                    let _comma: syn::Token![,] = input.parse()?;
                }

                Ok(args.build())
            }
        }
    }
}

pub(crate) use parse_as_custom_fields;
pub(crate) mod common {
    use proc_macro2::{Ident, TokenStream};
    use proc_macro_error::OptionExt;
    use quote::{quote, ToTokens};
    use syn::parse::Parse;
    use syn::{braced, bracketed, custom_keyword, Token};

    custom_keyword!(Multiplexer);
    custom_keyword!(Direct);
    custom_keyword!(pin);
    custom_keyword!(select_pins);

    crate::parse_as_custom_fields! {
        pub struct MultiplexerArgsBuilder for MultiplexerArgs {
            pub pin: Ident,
            pub select_pins: Row<OptionalItem<Ident>>
        }
    }

    #[allow(dead_code)]
    pub struct MultiplexerDefinition {
        pub multiplexer_field_name: Multiplexer,
        pub pin_brace_token: syn::token::Brace,
        pub multiplexer_args: MultiplexerArgs,
    }

    impl Parse for MultiplexerDefinition {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            let content;
            Ok(Self {
                multiplexer_field_name: input.parse()?,
                pin_brace_token: braced!(content in input),
                multiplexer_args: content.parse()?,
            })
        }
    }

    crate::parse_as_custom_fields! {
        pub struct DirectPinArgsBuilder for DirectPinArgs {
            pub pin: Ident
        }
    }

    #[allow(dead_code)]
    pub struct DirectPinDefinition {
        pub direct_field_name: Direct,
        pub brace_token: syn::token::Brace,
        pub direct_pin_args: DirectPinArgs,
    }

    impl Parse for DirectPinDefinition {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            let content;
            Ok(Self {
                direct_field_name: input.parse()?,
                brace_token: braced!(content in input),
                direct_pin_args: content.parse()?,
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

    #[derive(Debug)]
    pub struct LayoutLike<T> {
        pub layers: Vec<Layer<T>>,
    }

    impl<T: Parse> syn::parse::Parse for LayoutLike<T> {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            let mut layers = Vec::new();
            while let Ok(t) = input.parse() {
                layers.push(t)
            }
            if !input.is_empty() {
                return Err(syn::Error::new(
                    input.span(),
                    "Encountered tokens that don't look like a layer definition.",
                ));
            }

            Ok(Self { layers })
        }
    }

    #[derive(Debug)]
    pub struct Layer<T> {
        pub layer_brace: syn::token::Brace,
        pub layer: MatrixLike<T>,
    }

    impl<T: Parse> syn::parse::Parse for Layer<T> {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            let content;
            let layer_brace = braced!(content in input);

            Ok(Self {
                layer_brace,
                layer: content.parse()?,
            })
        }
    }

    #[derive(Debug)]
    pub struct MatrixLike<T> {
        pub rows: Vec<Row<T>>,
    }

    impl<T: Parse> syn::parse::Parse for MatrixLike<T> {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            let mut rows = Vec::new();
            while match input.parse() {
                Ok(t) => {
                    rows.push(t);
                    true
                }
                Err(e) if !input.is_empty() => {
                    let mut err = syn::Error::new(
                        input.span(),
                        "Encountered tokens that don't look like a row definition.",
                    );
                    err.combine(e);
                    return Err(err);
                }
                Err(_) => false,
            } {}

            Ok(Self { rows })
        }
    }

    #[derive(Debug)]
    pub struct Row<T> {
        pub row_bracket: syn::token::Bracket,
        pub items: Vec<T>,
    }

    impl<T: Parse> syn::parse::Parse for Row<T> {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            let content;
            let row_bracket = bracketed!(content in input);
            let mut items = Vec::new();
            while match content.parse() {
                Ok(t) => {
                    items.push(t);
                    true
                }
                Err(e) if !content.is_empty() => {
                    let mut err = syn::Error::new(input.span(), "Encountered an invalid token.");
                    err.combine(e);
                    return Err(err);
                }
                Err(_) => false,
            } {}

            Ok(Self { row_bracket, items })
        }
    }

    #[derive(Debug)]
    /// This is the exact same as [`Option<T>`], but has a different [`syn::parse::Parse`] implementation,
    /// where "No" parses to `None`, and anything else that parses as `T` corresponds `Some(T)`
    pub(crate) enum OptionalItem<T> {
        None,
        Some(T),
    }

    custom_keyword!(No);

    impl<T: Parse> Parse for OptionalItem<T> {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            let lookahead = input.lookahead1();
            if lookahead.peek(No) {
                input.parse::<No>().map(|_| OptionalItem::None)
            } else {
                input.parse().map(OptionalItem::Some)
            }
        }
    }

    impl<T: ToTokens> ToTokens for OptionalItem<T> {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            match self {
                OptionalItem::None => quote! { None }.to_tokens(tokens),
                OptionalItem::Some(item) => quote! { Some(#item) }.to_tokens(tokens),
            }
        }
    }
}
