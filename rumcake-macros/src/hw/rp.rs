use proc_macro2::{Ident, TokenStream};
use proc_macro_error::OptionExt;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::Token;

use crate::common::{AnalogPinType, DirectPinDefinition, MultiplexerDefinition};

pub const HAL_CRATE: &'static str = "embassy_rp";

pub fn input_pin(ident: Ident) -> TokenStream {
    quote! {
        unsafe {
            ::rumcake::hw::mcu::embassy_rp::gpio::Input::new(
                ::rumcake::hw::mcu::embassy_rp::gpio::Pin::degrade(
                    ::rumcake::hw::mcu::embassy_rp::peripherals::#ident::steal(),
                ),
                ::rumcake::hw::mcu::embassy_rp::gpio::Pull::Up,
            )
        }
    }
}

pub fn output_pin(ident: Ident) -> TokenStream {
    quote! {
        unsafe {
            ::rumcake::hw::mcu::embassy_rp::gpio::Output::new(
                ::rumcake::hw::mcu::embassy_rp::gpio::Pin::degrade(
                    ::rumcake::hw::mcu::embassy_rp::peripherals::#ident::steal(),
                ),
                ::rumcake::hw::mcu::embassy_rp::gpio::Level::High,
            )
        }
    }
}

pub fn internal_storage_trait() -> TokenStream {
    quote! {
        /// A trait that must be implemented to use the flash chip connected to your RP2040 for storage..
        pub(crate) trait RP2040FlashSettings {
            /// Get the DMA channel used for flash operations.
            fn setup_dma_channel(
            ) -> impl ::rumcake::hw::mcu::embassy_rp::Peripheral<P = impl ::rumcake::hw::mcu::embassy_rp::dma::Channel>;
        }
    }
}

pub fn setup_dma_channel(dma: Ident) -> TokenStream {
    quote! {
        fn setup_dma_channel(
        ) -> impl ::rumcake::hw::mcu::embassy_rp::Peripheral<P = impl ::rumcake::hw::mcu::embassy_rp::dma::Channel> {
            unsafe {
                ::rumcake::hw::mcu::embassy_rp::peripherals::#dma::steal()
            }
        }
    }
}

fn setup_i2c_inner(args: Punctuated<Ident, Token![,]>) -> TokenStream {
    let mut args = args.iter();

    let interrupt = args.next().expect_or_abort("Missing interrupt argument.");
    let i2c = args
        .next()
        .expect_or_abort("Missing I2C peripheral argument.");
    let scl = args
        .next()
        .expect_or_abort("Missing SCL peripheral argument.");
    let sda = args
        .next()
        .expect_or_abort("Missing SDA peripheral argument.");

    quote! {
        unsafe {
            ::rumcake::hw::mcu::embassy_rp::bind_interrupts! {
                struct Irqs {
                    #interrupt => ::rumcake::hw::mcu::embassy_rp::i2c::InterruptHandler<::rumcake::hw::mcu::embassy_rp::peripherals::#i2c>;
                }
            };
            let i2c = ::rumcake::hw::mcu::embassy_rp::peripherals::#i2c::steal();
            let scl = ::rumcake::hw::mcu::embassy_rp::peripherals::#scl::steal();
            let sda = ::rumcake::hw::mcu::embassy_rp::peripherals::#sda::steal();
            ::rumcake::hw::mcu::embassy_rp::i2c::I2c::new_async(i2c, scl, sda, Irqs, Default::default())
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
            ::rumcake::hw::mcu::embassy_rp::bind_interrupts! {
                struct Irqs {
                    #interrupt => ::rumcake::hw::mcu::embassy_rp::uart::BufferedInterruptHandler<::rumcake::hw::mcu::embassy_rp::peripherals::#uart>;
                }
            };
            let uart = ::rumcake::hw::mcu::embassy_rp::peripherals::#uart::steal();
            let rx = ::rumcake::hw::mcu::embassy_rp::peripherals::#rx_pin::steal();
            let tx = ::rumcake::hw::mcu::embassy_rp::peripherals::#tx_pin::steal();
            ::rumcake::hw::mcu::embassy_rp::uart::BufferedUart::new(
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

pub fn setup_buffered_uart(args: Punctuated<Ident, Token![,]>) -> TokenStream {
    let inner = setup_buffered_uart_inner(args);

    quote! {
        fn setup_serial(
        ) -> impl ::rumcake::embedded_io_async::Write + ::rumcake::embedded_io_async::Read {
            #inner
        }
    }
}

pub fn setup_adc_sampler(channels: Punctuated<AnalogPinType, Token![,]>) -> TokenStream {
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
                        ::rumcake::hw::mcu::embassy_rp::adc::Channel::new_pin(
                            unsafe { ::rumcake::hw::mcu::embassy_rp::peripherals::#pin::steal() },
                            ::rumcake::hw::mcu::embassy_rp::gpio::Pull::None
                        )
                    },
                )
            }
            AnalogPinType::Direct(DirectPinDefinition { pin, .. }) => (
                quote! {
                    ::rumcake::hw::mcu::AnalogPinType::Direct
                },
                quote! {
                    ::rumcake::hw::mcu::embassy_rp::adc::Channel::new_pin(
                        unsafe { ::rumcake::hw::mcu::embassy_rp::peripherals::#pin::steal() },
                        ::rumcake::hw::mcu::embassy_rp::gpio::Pull::None
                    )
                },
            ),
        })
        .unzip();

    quote! {
        type AdcSamplerType = ::rumcake::hw::mcu::AdcSampler<'static, #select_pin_count, #channel_count>;

        static SAMPLER: ::rumcake::once_cell::sync::OnceCell<AdcSamplerType> = ::rumcake::once_cell::sync::OnceCell::new();

        fn setup_adc_sampler() -> &'static AdcSamplerType {
            SAMPLER.get_or_init(||
                ::rumcake::hw::mcu::AdcSampler::new(
                    [ #(#pins),* ],
                    [ #(#channels),* ]
                )
            )
        }
    }
}
