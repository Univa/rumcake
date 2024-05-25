use proc_macro2::{Ident, TokenStream};
use proc_macro_error::{abort, OptionExt};
use quote::quote;
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::{braced, parenthesized, LitInt, Token};

use crate::common::{
    AnalogPinType, DirectPinArgs, DirectPinDefinition, MultiplexerArgs, MultiplexerDefinition,
    OptionalItem,
};

pub const HAL_CRATE: &str = "embassy_stm32";

pub fn input_pin(args: Punctuated<Ident, Token![,]>) -> TokenStream {
    let mut args = args.iter();
    let ident = args.next().expect_or_abort("missing pin identifier");
    let exti_channel = args.next();

    if let Some(arg) = args.next() {
        abort!(arg, "unexpected extra args provided");
    }

    if let Some(exti) = exti_channel {
        quote! {
            unsafe {
                ::rumcake::hw::platform::embassy_stm32::exti::ExtiInput::new(
                    ::rumcake::hw::platform::embassy_stm32::peripherals::#ident::steal(),
                    ::rumcake::hw::platform::embassy_stm32::peripherals::#exti::steal(),
                    ::rumcake::hw::platform::embassy_stm32::gpio::Pull::Up,
                )
            }
        }
    } else {
        quote! {
            unsafe {
                ::rumcake::hw::platform::embassy_stm32::gpio::Input::new(
                    ::rumcake::hw::platform::embassy_stm32::gpio::Pin::degrade(
                        ::rumcake::hw::platform::embassy_stm32::peripherals::#ident::steal(),
                    ),
                    ::rumcake::hw::platform::embassy_stm32::gpio::Pull::Up,
                )
            }
        }
    }
}

pub fn output_pin(ident: Ident) -> TokenStream {
    quote! {
        unsafe {
            ::rumcake::hw::platform::embassy_stm32::gpio::Output::new(
                ::rumcake::hw::platform::embassy_stm32::gpio::Pin::degrade(
                    ::rumcake::hw::platform::embassy_stm32::peripherals::#ident::steal(),
                ),
                ::rumcake::hw::platform::embassy_stm32::gpio::Level::High,
                ::rumcake::hw::platform::embassy_stm32::gpio::Speed::Low,
            )
        }
    }
}

crate::parse_as_custom_fields! {
    pub struct I2cArgsBuilder for I2cArgs {
        event_interrupt: Ident,
        error_interrupt: Ident,
        i2c: Ident,
        scl: Ident,
        sda: Ident,
        rx_dma: Ident,
        tx_dma: Ident
    }
}

pub fn setup_i2c(
    I2cArgs {
        event_interrupt,
        error_interrupt,
        i2c,
        scl,
        sda,
        rx_dma,
        tx_dma,
    }: I2cArgs,
) -> TokenStream {
    let interrupt_setup = if event_interrupt == error_interrupt {
        quote! {
            #event_interrupt => ::rumcake::hw::platform::embassy_stm32::i2c::EventInterruptHandler<::rumcake::hw::platform::embassy_stm32::peripherals::#i2c>, ::rumcake::hw::platform::embassy_stm32::i2c::ErrorInterruptHandler<::rumcake::hw::platform::embassy_stm32::peripherals::#i2c>;
        }
    } else {
        quote! {
            #event_interrupt => ::rumcake::hw::platform::embassy_stm32::i2c::EventInterruptHandler<::rumcake::hw::platform::embassy_stm32::peripherals::#i2c>;
            #error_interrupt => ::rumcake::hw::platform::embassy_stm32::i2c::ErrorInterruptHandler<::rumcake::hw::platform::embassy_stm32::peripherals::#i2c>;
        }
    };

    quote! {
        unsafe {
            ::rumcake::hw::platform::embassy_stm32::bind_interrupts! {
                struct Irqs {
                    #interrupt_setup
                }
            };
            let i2c = ::rumcake::hw::platform::embassy_stm32::peripherals::#i2c::steal();
            let scl = ::rumcake::hw::platform::embassy_stm32::peripherals::#scl::steal();
            let sda = ::rumcake::hw::platform::embassy_stm32::peripherals::#sda::steal();
            let rx_dma = ::rumcake::hw::platform::embassy_stm32::peripherals::#rx_dma::steal();
            let tx_dma = ::rumcake::hw::platform::embassy_stm32::peripherals::#tx_dma::steal();
            let time = ::rumcake::hw::platform::embassy_stm32::time::Hertz(100_000);
            ::rumcake::hw::platform::embassy_stm32::i2c::I2c::new(i2c, scl, sda, Irqs, tx_dma, rx_dma, time, Default::default())
        }
    }
}

crate::parse_as_custom_fields! {
    pub struct BufferedUartArgsBuilder for BufferedUartArgs {
        interrupt: Ident,
        uart: Ident,
        rx_pin: Ident,
        tx_pin: Ident,
        buffer_size: Option<LitInt>,
    }
}

pub fn setup_buffered_uart(
    BufferedUartArgs {
        interrupt,
        uart,
        rx_pin,
        tx_pin,
        buffer_size,
    }: BufferedUartArgs,
) -> TokenStream {
    let buf_size = buffer_size.map_or(64, |lit| {
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
            ::rumcake::hw::platform::embassy_stm32::bind_interrupts! {
                struct Irqs {
                    #interrupt => ::rumcake::hw::platform::embassy_stm32::usart::BufferedInterruptHandler<::rumcake::hw::platform::embassy_stm32::peripherals::#uart>;
                }
            };
            let uart = ::rumcake::hw::platform::embassy_stm32::peripherals::#uart::steal();
            let rx = ::rumcake::hw::platform::embassy_stm32::peripherals::#rx_pin::steal();
            let tx = ::rumcake::hw::platform::embassy_stm32::peripherals::#tx_pin::steal();
            ::rumcake::hw::platform::embassy_stm32::usart::BufferedUart::new(
                uart,
                Irqs,
                rx,
                tx,
                &mut TBUF,
                &mut RBUF,
                Default::default(),
            ).unwrap()
        }
    }
}

crate::parse_as_custom_fields! {
    pub struct AdcArgsBuilder for AdcArgs {
        interrupt: Ident,
        adc: Ident
    }
}

pub struct STM32AdcSamplerDefinition {
    parenthesis_token: syn::token::Paren,
    adc_instance_args: AdcArgs,
    colon_token: Token![=>],
    brace_token: syn::token::Brace,
    channels: Punctuated<AnalogPinType, Token![,]>,
}

impl Parse for STM32AdcSamplerDefinition {
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

fn setup_adc_inner(adc_definition: &STM32AdcSamplerDefinition) -> (TokenStream, TokenStream) {
    let STM32AdcSamplerDefinition {
        adc_instance_args,
        channels,
        ..
    } = adc_definition;

    let AdcArgs { interrupt, adc } = adc_instance_args;

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

    let (pins, channels): (Vec<TokenStream>, Vec<TokenStream>) = channels
        .iter()
        .map(|ch| match ch {
            AnalogPinType::Multiplexed(MultiplexerDefinition {
                multiplexer_args, ..
            }) => {
                let MultiplexerArgs {pin, select_pins} = multiplexer_args;
                let select_pins = select_pins.items.iter().map(|select_pin| match select_pin {
                    OptionalItem::None => quote! { None },
                    OptionalItem::Some(pin_ident) => {
                        quote! { Some(::rumcake::hw::platform::output_pin!(#pin_ident)) }
                    }
                });

                (
                    quote! {
                        ::rumcake::hw::platform::AnalogPinType::Multiplexed(
                            ::rumcake::hw::Multiplexer::new(
                                [ #(#select_pins),* ],
                                None
                            )
                        )
                    },
                    quote! {
                        unsafe {
                            use ::rumcake::hw::platform::embassy_stm32::adc::AdcChannel;
                            ::rumcake::hw::platform::embassy_stm32::peripherals::#pin::steal().degrade_adc()
                        }
                    },
                )
            }
            AnalogPinType::Direct(DirectPinDefinition { direct_pin_args, .. }) => {
                let DirectPinArgs { pin } = direct_pin_args;
                (
                    quote! {
                        ::rumcake::hw::platform::AnalogPinType::Direct
                    },
                    quote! {
                        unsafe {
                            use ::rumcake::hw::platform::embassy_stm32::adc::AdcChannel;
                            ::rumcake::hw::platform::embassy_stm32::peripherals::#pin::steal().degrade_adc()
                        }
                    },
                )
            },
        })
        .unzip();

    (
        quote! {
            ::rumcake::hw::platform::AdcSampler<'static, ::rumcake::hw::platform::embassy_stm32::peripherals::#adc, #select_pin_count, #channel_count>
        },
        quote! {
            ::rumcake::hw::platform::AdcSampler::new(
                unsafe { ::rumcake::hw::platform::embassy_stm32::peripherals::#adc::steal() },
                {
                    ::rumcake::hw::platform::embassy_stm32::bind_interrupts! {
                        struct Irqs {
                            #interrupt => ::rumcake::hw::platform::embassy_stm32::adc::InterruptHandler<::rumcake::hw::platform::embassy_stm32::peripherals::#adc>;
                        }
                    };
                    Irqs
                },
                [ #(#pins),* ],
                [ #(#channels),* ]
            )
        },
    )
}

pub fn setup_adc_sampler(
    samplers: Punctuated<STM32AdcSamplerDefinition, Token![,]>,
) -> TokenStream {
    let (types, instances): (Vec<TokenStream>, Vec<TokenStream>) =
        samplers.iter().map(setup_adc_inner).unzip();

    let final_type = if types.len() == 1 {
        quote! { #(#types)* }
    } else {
        quote! { (#(#types),*) }
    };

    let final_instance = if instances.len() == 1 {
        quote! { #(#instances)* }
    } else {
        quote! { (#(#instances),*) }
    };

    quote! {
        type AdcSamplerType = #final_type;

        static SAMPLER: ::rumcake::once_cell::sync::OnceCell<
            AdcSamplerType,
        > = ::rumcake::once_cell::sync::OnceCell::new();

        fn setup_adc_sampler() -> &'static AdcSamplerType {
            SAMPLER.get_or_init(||
                #final_instance
            )
        }
    }
}
