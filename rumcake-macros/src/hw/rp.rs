use proc_macro2::{Ident, TokenStream};
use proc_macro_error::abort;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::{LitInt, Token};

use crate::common::{
    AnalogPinType, DirectPinArgs, DirectPinDefinition, MultiplexerArgs, MultiplexerDefinition,
    OptionalItem,
};

pub const HAL_CRATE: &str = "embassy_rp";

pub fn input_pin(ident: Ident) -> TokenStream {
    quote! {
        unsafe {
            ::rumcake::hw::platform::embassy_rp::gpio::Input::new(
                ::rumcake::hw::platform::embassy_rp::gpio::Pin::degrade(
                    ::rumcake::hw::platform::embassy_rp::peripherals::#ident::steal(),
                ),
                ::rumcake::hw::platform::embassy_rp::gpio::Pull::Up,
            )
        }
    }
}

pub fn output_pin(ident: Ident) -> TokenStream {
    quote! {
        unsafe {
            ::rumcake::hw::platform::embassy_rp::gpio::Output::new(
                ::rumcake::hw::platform::embassy_rp::gpio::Pin::degrade(
                    ::rumcake::hw::platform::embassy_rp::peripherals::#ident::steal(),
                ),
                ::rumcake::hw::platform::embassy_rp::gpio::Level::High,
            )
        }
    }
}

crate::parse_as_custom_fields! {
    pub struct I2cArgsBuilder for I2cArgs {
        interrupt: Ident,
        i2c: Ident,
        scl: Ident,
        sda: Ident,
    }
}

pub fn setup_i2c(
    I2cArgs {
        interrupt,
        i2c,
        scl,
        sda,
    }: I2cArgs,
) -> TokenStream {
    quote! {
        unsafe {
            ::rumcake::hw::platform::embassy_rp::bind_interrupts! {
                struct Irqs {
                    #interrupt => ::rumcake::hw::platform::embassy_rp::i2c::InterruptHandler<::rumcake::hw::platform::embassy_rp::peripherals::#i2c>;
                }
            };
            let i2c = ::rumcake::hw::platform::embassy_rp::peripherals::#i2c::steal();
            let scl = ::rumcake::hw::platform::embassy_rp::peripherals::#scl::steal();
            let sda = ::rumcake::hw::platform::embassy_rp::peripherals::#sda::steal();
            ::rumcake::hw::platform::embassy_rp::i2c::I2c::new_async(i2c, scl, sda, Irqs, Default::default())
        }
    }
}

crate::parse_as_custom_fields! {
    pub struct BufferedUartArgsBuilder for BufferedUartArgs {
        interrupt: Ident,
        uart: Ident,
        rx_pin: Ident,
        tx_pin: Ident,
        buffer_size: Option<LitInt>
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
            ::rumcake::hw::platform::embassy_rp::bind_interrupts! {
                struct Irqs {
                    #interrupt => ::rumcake::hw::platform::embassy_rp::uart::BufferedInterruptHandler<::rumcake::hw::platform::embassy_rp::peripherals::#uart>;
                }
            };
            let uart = ::rumcake::hw::platform::embassy_rp::peripherals::#uart::steal();
            let rx = ::rumcake::hw::platform::embassy_rp::peripherals::#rx_pin::steal();
            let tx = ::rumcake::hw::platform::embassy_rp::peripherals::#tx_pin::steal();
            ::rumcake::hw::platform::embassy_rp::uart::BufferedUart::new(
                uart,
                Irqs,
                rx,
                tx,
                &mut TBUF,
                &mut RBUF,
                Default::default(),
            )
        }
    }
}

pub fn setup_adc_sampler(channels: Punctuated<AnalogPinType, Token![,]>) -> TokenStream {
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
                        ::rumcake::hw::platform::embassy_rp::adc::Channel::new_pin(
                            unsafe { ::rumcake::hw::platform::embassy_rp::peripherals::#pin::steal() },
                            ::rumcake::hw::platform::embassy_rp::gpio::Pull::None
                        )
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
                        ::rumcake::hw::platform::embassy_rp::adc::Channel::new_pin(
                            unsafe { ::rumcake::hw::platform::embassy_rp::peripherals::#pin::steal() },
                            ::rumcake::hw::platform::embassy_rp::gpio::Pull::None
                        )
                    },
                )
            },
        })
        .unzip();

    quote! {
        type AdcSamplerType = ::rumcake::hw::platform::AdcSampler<'static, #select_pin_count, #channel_count>;

        static SAMPLER: ::rumcake::once_cell::sync::OnceCell<AdcSamplerType> = ::rumcake::once_cell::sync::OnceCell::new();

        fn setup_adc_sampler() -> &'static AdcSamplerType {
            SAMPLER.get_or_init(||
                ::rumcake::hw::platform::AdcSampler::new(
                    [ #(#pins),* ],
                    [ #(#channels),* ]
                )
            )
        }
    }
}
