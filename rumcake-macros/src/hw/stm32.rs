use proc_macro2::{Ident, TokenStream};
use proc_macro_error::{abort, OptionExt};
use quote::quote;
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::{braced, parenthesized, Token};

use crate::common::{AnalogPinType, DirectPinDefinition, MultiplexerDefinition};

pub const HAL_CRATE: &'static str = "embassy_stm32";

pub fn input_pin(ident: Ident) -> TokenStream {
    quote! {
        unsafe {
            ::rumcake::hw::mcu::embassy_stm32::gpio::Input::new(
                ::rumcake::hw::mcu::embassy_stm32::gpio::Pin::degrade(
                    ::rumcake::hw::mcu::embassy_stm32::peripherals::#ident::steal(),
                ),
                ::rumcake::hw::mcu::embassy_stm32::gpio::Pull::Up,
            )
        }
    }
}

pub fn output_pin(ident: Ident) -> TokenStream {
    quote! {
        unsafe {
            ::rumcake::hw::mcu::embassy_stm32::gpio::Output::new(
                ::rumcake::hw::mcu::embassy_stm32::gpio::Pin::degrade(
                    ::rumcake::hw::mcu::embassy_stm32::peripherals::#ident::steal(),
                ),
                ::rumcake::hw::mcu::embassy_stm32::gpio::Level::High,
                ::rumcake::hw::mcu::embassy_stm32::gpio::Speed::Low,
            )
        }
    }
}

fn setup_i2c_inner(args: Punctuated<Ident, Token![,]>) -> TokenStream {
    let mut args = args.iter();

    let event_interrupt = args
        .next()
        .expect_or_abort("Missing event interrupt argument.");
    let error_interrupt = args
        .next()
        .expect_or_abort("Missing error interrupt argument.");
    let i2c = args
        .next()
        .expect_or_abort("Missing I2C peripheral argument.");
    let scl = args
        .next()
        .expect_or_abort("Missing SCL peripheral argument.");
    let sda = args
        .next()
        .expect_or_abort("Missing SDA peripheral argument.");
    let rxdma = args
        .next()
        .expect_or_abort("Missing RX DMA peripheral argument.");
    let txdma = args
        .next()
        .expect_or_abort("Missing TX DMA peripheral argument.");

    if let Some(literal) = args.next() {
        abort!(literal.span(), "Unexpected extra arguments.")
    }

    let interrupt_setup = if event_interrupt == error_interrupt {
        quote! {
            #event_interrupt => ::rumcake::hw::mcu::embassy_stm32::i2c::EventInterruptHandler<::rumcake::hw::mcu::embassy_stm32::peripherals::#i2c>, ::rumcake::hw::mcu::embassy_stm32::i2c::ErrorInterruptHandler<::rumcake::hw::mcu::embassy_stm32::peripherals::#i2c>;
        }
    } else {
        quote! {
            #event_interrupt => ::rumcake::hw::mcu::embassy_stm32::i2c::EventInterruptHandler<::rumcake::hw::mcu::embassy_stm32::peripherals::#i2c>;
            #error_interrupt => ::rumcake::hw::mcu::embassy_stm32::i2c::ErrorInterruptHandler<::rumcake::hw::mcu::embassy_stm32::peripherals::#i2c>;
        }
    };

    quote! {
        unsafe {
            ::rumcake::hw::mcu::embassy_stm32::bind_interrupts! {
                struct Irqs {
                    #interrupt_setup
                }
            };
            let i2c = ::rumcake::hw::mcu::embassy_stm32::peripherals::#i2c::steal();
            let scl = ::rumcake::hw::mcu::embassy_stm32::peripherals::#scl::steal();
            let sda = ::rumcake::hw::mcu::embassy_stm32::peripherals::#sda::steal();
            let rx_dma = ::rumcake::hw::mcu::embassy_stm32::peripherals::#rxdma::steal();
            let tx_dma = ::rumcake::hw::mcu::embassy_stm32::peripherals::#txdma::steal();
            let time = ::rumcake::hw::mcu::embassy_stm32::time::Hertz(100_000);
            ::rumcake::hw::mcu::embassy_stm32::i2c::I2c::new(i2c, scl, sda, Irqs, tx_dma, rx_dma, time, Default::default())
        }
    }
}

pub fn setup_i2c(args: Punctuated<Ident, Token![,]>) -> TokenStream {
    let inner = setup_i2c_inner(args);
    quote! {
        fn setup_i2c() -> impl ::rumcake::embedded_hal_async::i2c::I2c<Error = impl core::fmt::Debug> {
            #inner
        }
    }
}

fn setup_buffered_uart_inner(args: Punctuated<Ident, Token![,]>) -> TokenStream {
    let mut args = args.iter();

    let interrupt = args.next().expect_or_abort("Missing interrupt argument.");
    let uart = args
        .next()
        .expect_or_abort("Missing UART peripheral argument.");
    let rx_pin = args.next().expect_or_abort("Missing RX pin argument.");
    let tx_pin = args.next().expect_or_abort("Missing TX pin argument.");

    quote! {
        unsafe {
            static mut RBUF: [u8; 64] = [0; 64];
            static mut TBUF: [u8; 64] = [0; 64];
            ::rumcake::hw::mcu::embassy_stm32::bind_interrupts! {
                struct Irqs {
                    #interrupt => ::rumcake::hw::mcu::embassy_stm32::usart::BufferedInterruptHandler<::rumcake::hw::mcu::embassy_stm32::peripherals::#uart>;
                }
            };
            let uart = ::rumcake::hw::mcu::embassy_stm32::peripherals::#uart::steal();
            let rx = ::rumcake::hw::mcu::embassy_stm32::peripherals::#rx_pin::steal();
            let tx = ::rumcake::hw::mcu::embassy_stm32::peripherals::#tx_pin::steal();
            ::rumcake::hw::mcu::embassy_stm32::usart::BufferedUart::new(
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

pub fn setup_buffered_uart(args: Punctuated<Ident, Token![,]>) -> TokenStream {
    let inner = setup_buffered_uart_inner(args);

    quote! {
        fn setup_serial(
        ) -> impl ::rumcake::embedded_io_async::Write + ::rumcake::embedded_io_async::Read {
            #inner
        }
    }
}

pub struct STM32AdcSamplerDefinition {
    parenthesis_token: syn::token::Paren,
    adc_instance_args: Punctuated<Ident, Token![,]>,
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
            adc_instance_args: Punctuated::parse_terminated(&adc_type_content)?,
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

    let mut args = adc_instance_args.iter();
    let interrupt = args.next().expect_or_abort("Missing interrupt argument.");
    let adc = args
        .next()
        .expect_or_abort("Missing ADC peripheral argument.");

    let channel_count = channels.len();
    let select_pin_count = channels.iter().fold(0, |acc, ch| {
        if let AnalogPinType::Multiplexed(MultiplexerDefinition { select_pins, .. }) = ch {
            acc.max(select_pins.len())
        } else {
            acc
        }
    });

    let (pins, channels): (Vec<TokenStream>, Vec<TokenStream>) = channels
        .iter()
        .map(|ch| match ch {
            AnalogPinType::Multiplexed(MultiplexerDefinition {
                pin, select_pins, ..
            }) => {
                let select_pins = select_pins.iter().map(|select_pin| match select_pin {
                    crate::keyboard::OptionalItem::None => quote! { None },
                    crate::keyboard::OptionalItem::Some(pin_ident) => {
                        quote! { Some(::rumcake::hw::mcu::output_pin!(#pin_ident)) }
                    }
                });

                (
                    quote! {
                        ::rumcake::hw::mcu::AnalogPinType::Multiplexed(
                            ::rumcake::hw::Multiplexer::new(
                                [ #(#select_pins),* ],
                                None
                            )
                        )
                    },
                    quote! {
                        ::rumcake::hw::mcu::Channel::new(
                            unsafe { ::rumcake::hw::mcu::embassy_stm32::peripherals::#pin::steal() }
                        )
                    },
                )
            }
            AnalogPinType::Direct(DirectPinDefinition { pin, .. }) => (
                quote! {
                    ::rumcake::hw::mcu::AnalogPinType::Direct
                },
                quote! {
                    ::rumcake::hw::mcu::Channel::new(
                        unsafe { ::rumcake::hw::mcu::embassy_stm32::peripherals::#pin::steal() }
                    )
                },
            ),
        })
        .unzip();

    (
        quote! {
            ::rumcake::hw::mcu::AdcSampler<'static, ::rumcake::hw::mcu::embassy_stm32::peripherals::#adc, #select_pin_count, #channel_count>
        },
        quote! {
            ::rumcake::hw::mcu::AdcSampler::new(
                unsafe { ::rumcake::hw::mcu::embassy_stm32::peripherals::#adc::steal() },
                {
                    ::rumcake::hw::mcu::embassy_stm32::bind_interrupts! {
                        struct Irqs {
                            #interrupt => ::rumcake::hw::mcu::embassy_stm32::adc::InterruptHandler<::rumcake::hw::mcu::embassy_stm32::peripherals::#adc>;
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
