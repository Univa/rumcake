use darling::FromMeta;
use proc_macro2::{Ident, Span, TokenStream};
use proc_macro_error::{abort, OptionExt};
use quote::quote;
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::{braced, parenthesized, LitInt, Token};

use crate::common::{
    AnalogPinType, DirectPinArgs, DirectPinDefinition, MultiplexerArgs, MultiplexerDefinition,
    OptionalItem,
};

pub const HAL_CRATE: &str = "embassy_nrf";

pub fn input_pin(ident: Ident) -> TokenStream {
    quote! {
        unsafe {
            ::rumcake::hw::platform::embassy_nrf::gpio::Input::new(
                ::rumcake::hw::platform::embassy_nrf::gpio::Pin::degrade(
                    ::rumcake::hw::platform::embassy_nrf::peripherals::#ident::steal(),
                ),
                ::rumcake::hw::platform::embassy_nrf::gpio::Pull::Up,
            )
        }
    }
}

pub fn output_pin(ident: Ident) -> TokenStream {
    quote! {
        unsafe {
            ::rumcake::hw::platform::embassy_nrf::gpio::Output::new(
                ::rumcake::hw::platform::embassy_nrf::gpio::Pin::degrade(
                    ::rumcake::hw::platform::embassy_nrf::peripherals::#ident::steal(),
                ),
                ::rumcake::hw::platform::embassy_nrf::gpio::Level::High,
                ::rumcake::hw::platform::embassy_nrf::gpio::OutputDrive::Standard,
            )
        }
    }
}

crate::parse_as_custom_fields! {
    pub struct I2cArgsBuilder for I2cArgs {
        interrupt: Ident,
        i2c: Ident,
        sda: Ident,
        scl: Ident,
    }
}

pub fn setup_i2c(
    I2cArgs {
        interrupt,
        i2c,
        sda,
        scl,
    }: I2cArgs,
) -> TokenStream {
    quote! {
        unsafe {
            use ::rumcake::hw::platform::embassy_nrf::interrupt::InterruptExt;
            ::rumcake::hw::platform::embassy_nrf::bind_interrupts! {
                struct Irqs {
                    #interrupt => ::rumcake::hw::platform::embassy_nrf::twim::InterruptHandler<::rumcake::hw::platform::embassy_nrf::peripherals::#i2c>;
                }
            };
            ::rumcake::hw::platform::embassy_nrf::interrupt::#interrupt.set_priority(::rumcake::hw::platform::embassy_nrf::interrupt::Priority::P2);
            let i2c = ::rumcake::hw::platform::embassy_nrf::peripherals::#i2c::steal();
            let sda = ::rumcake::hw::platform::embassy_nrf::peripherals::#sda::steal();
            let scl = ::rumcake::hw::platform::embassy_nrf::peripherals::#scl::steal();
            ::rumcake::hw::platform::embassy_nrf::twim::Twim::new(i2c, Irqs, sda, scl, Default::default())
        }
    }
}

crate::parse_as_custom_fields! {
    pub struct BufferedUarteArgsBuilder for BufferedUarteArgs {
        interrupt: Ident,
        uarte: Ident,
        timer: Ident,
        ppi_ch0: Ident,
        ppi_ch1: Ident,
        ppi_group: Ident,
        rx_pin: Ident,
        tx_pin: Ident,
        buffer_size: Option<LitInt>,
    }
}

pub fn setup_buffered_uarte(
    BufferedUarteArgs {
        interrupt,
        uarte,
        timer,
        ppi_ch0,
        ppi_ch1,
        ppi_group,
        rx_pin,
        tx_pin,
        buffer_size,
    }: BufferedUarteArgs,
) -> TokenStream {
    let buf_size = buffer_size.map_or(1024, |lit| {
        lit.base10_parse::<usize>().unwrap_or_else(|_| {
            abort!(
                lit,
                "The provided buffer size could not be parsed as a usize value."
            )
        })
    });

    quote! {
        unsafe {
            static mut RBUF: [u8; #buf_size] = [0; #buf_size];
            static mut TBUF: [u8; #buf_size] = [0; #buf_size];
            ::rumcake::hw::platform::embassy_nrf::bind_interrupts! {
                struct Irqs {
                    #interrupt => ::rumcake::hw::platform::embassy_nrf::buffered_uarte::InterruptHandler<::rumcake::hw::platform::embassy_nrf::peripherals::#uarte>;
                }
            };
            let uarte = ::rumcake::hw::platform::embassy_nrf::peripherals::#uarte::steal();
            let timer = ::rumcake::hw::platform::embassy_nrf::peripherals::#timer::steal();
            let ppi_ch0 = ::rumcake::hw::platform::embassy_nrf::peripherals::#ppi_ch0::steal();
            let ppi_ch1 = ::rumcake::hw::platform::embassy_nrf::peripherals::#ppi_ch1::steal();
            let ppi_group = ::rumcake::hw::platform::embassy_nrf::peripherals::#ppi_group::steal();
            let rx_pin = ::rumcake::hw::platform::embassy_nrf::peripherals::#rx_pin::steal();
            let tx_pin = ::rumcake::hw::platform::embassy_nrf::peripherals::#tx_pin::steal();
            ::rumcake::hw::platform::embassy_nrf::buffered_uarte::BufferedUarte::new(
                uarte,
                timer,
                ppi_ch0,
                ppi_ch1,
                ppi_group,
                Irqs,
                rx_pin,
                tx_pin,
                Default::default(),
                &mut RBUF,
                &mut TBUF,
            )
        }
    }
}

crate::parse_as_custom_fields! {
    pub struct AdcArgsBuilder for AdcArgs {
        timer: Ident,
        ppi_ch0: Ident,
        ppi_ch1: Ident,
    }
}

pub struct NrfAdcSamplerDefinition {
    parenthesis_token: syn::token::Paren,
    adc_instance_args: AdcArgs,
    colon_token: Token![=>],
    brace_token: syn::token::Brace,
    channels: Punctuated<AnalogPinType, Token![,]>,
}

impl Parse for NrfAdcSamplerDefinition {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let adc_type_content;
        let channels_content;
        Ok(Self {
            parenthesis_token: parenthesized!(adc_type_content in input),
            adc_instance_args: adc_type_content.parse()?,
            colon_token: input.parse()?,
            brace_token: braced!(channels_content in input),
            channels: Punctuated::parse_terminated(&channels_content)?,
        })
    }
}

pub fn setup_adc_sampler(
    NrfAdcSamplerDefinition {
        adc_instance_args,
        channels,
        ..
    }: NrfAdcSamplerDefinition,
) -> TokenStream {
    let AdcArgs {
        timer,
        ppi_ch0,
        ppi_ch1,
    } = adc_instance_args;

    let channel_count = channels.len();
    let select_pin_count = channels.iter().fold(0, |acc, ch| {
        if let AnalogPinType::Multiplexed(MultiplexerDefinition {
            multiplexer_args, ..
        }) = ch
        {
            acc.max(multiplexer_args.select_pins.items.len())
        } else {
            acc
        }
    });
    let buf_size = 2usize.pow(select_pin_count as u32);

    let (pins, channels): (Vec<TokenStream>, Vec<TokenStream>) = channels
        .iter()
        .map(|ch| match ch {
            AnalogPinType::Multiplexed(MultiplexerDefinition {
                multiplexer_args, ..
            }) => {
                let MultiplexerArgs { pin, select_pins } = multiplexer_args;
                let select_pins = select_pins.items.iter().map(|select_pin| match select_pin {
                    OptionalItem::None => quote! { None },
                    OptionalItem::Some(pin_ident) => {
                        quote! { Some(::rumcake::hw::platform::output_pin!(#pin_ident)) }
                    }
                });

                (
                    quote! {
                        ::rumcake::hw::platform::AnalogPinType::Multiplexed(
                            [0; #buf_size],
                            ::rumcake::hw::Multiplexer::new(
                                [ #(#select_pins),* ],
                                None
                            )
                        )
                    },
                    quote! {
                        ::rumcake::hw::platform::embassy_nrf::saadc::ChannelConfig::single_ended(
                            unsafe { ::rumcake::hw::platform::embassy_nrf::peripherals::#pin::steal() }
                        )
                    },
                )
            }
            AnalogPinType::Direct(DirectPinDefinition { direct_pin_args, .. }) => {
                let DirectPinArgs { pin } = direct_pin_args;
                (
                    quote! {
                        ::rumcake::hw::platform::AnalogPinType::Direct([0])
                    },
                    quote! {
                        ::rumcake::hw::platform::embassy_nrf::saadc::ChannelConfig::single_ended(
                            unsafe { ::rumcake::hw::platform::embassy_nrf::peripherals::#pin::steal() }
                        )
                    },
                )
            },
        })
        .unzip();

    quote! {
        type AdcSamplerType = ::rumcake::hw::platform::AdcSampler<
            'static,
            ::rumcake::hw::platform::embassy_nrf::peripherals::#timer,
            ::rumcake::hw::platform::embassy_nrf::peripherals::#ppi_ch0,
            ::rumcake::hw::platform::embassy_nrf::peripherals::#ppi_ch1,
            #select_pin_count,
            #channel_count,
        >;

        fn setup_adc_sampler() -> &'static AdcSamplerType {
            static SAMPLER: ::rumcake::once_cell::sync::OnceCell<AdcSamplerType> = ::rumcake::once_cell::sync::OnceCell::new();

            SAMPLER.get_or_init(|| unsafe {
                ::rumcake::hw::platform::AdcSampler::new(
                    [ #(#pins),* ],
                    [ #(#channels),* ],
                    ::rumcake::hw::platform::embassy_nrf::peripherals::#timer::steal(),
                    ::rumcake::hw::platform::embassy_nrf::peripherals::#ppi_ch0::steal(),
                    ::rumcake::hw::platform::embassy_nrf::peripherals::#ppi_ch1::steal(),
                )
            })
        }
    }
}
