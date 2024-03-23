use proc_macro2::{Ident, TokenStream};
use proc_macro_error::{abort, OptionExt};
use quote::quote;
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::{braced, parenthesized, Token};

use crate::common::{AnalogPinType, DirectPinDefinition, MultiplexerDefinition};

pub const HAL_CRATE: &'static str = "embassy_nrf";

pub fn input_pin(ident: Ident) -> TokenStream {
    quote! {
        unsafe {
            ::rumcake::hw::mcu::embassy_nrf::gpio::Input::new(
                ::rumcake::hw::mcu::embassy_nrf::gpio::Pin::degrade(
                    ::rumcake::hw::mcu::embassy_nrf::peripherals::#ident::steal(),
                ),
                ::rumcake::hw::mcu::embassy_nrf::gpio::Pull::Up,
            )
        }
    }
}

pub fn output_pin(ident: Ident) -> TokenStream {
    quote! {
        unsafe {
            ::rumcake::hw::mcu::embassy_nrf::gpio::Output::new(
                ::rumcake::hw::mcu::embassy_nrf::gpio::Pin::degrade(
                    ::rumcake::hw::mcu::embassy_nrf::peripherals::#ident::steal(),
                ),
                ::rumcake::hw::mcu::embassy_nrf::gpio::Level::High,
                ::rumcake::hw::mcu::embassy_nrf::gpio::OutputDrive::Standard,
            )
        }
    }
}

fn setup_i2c_inner(args: Punctuated<Ident, Token![,]>) -> TokenStream {
    let mut args = args.iter();

    let interrupt = args.next().expect_or_abort("Missing interrupt argument.");
    let i2c = args
        .next()
        .expect_or_abort("Missing I2C peripheral argument.");
    let sda = args
        .next()
        .expect_or_abort("Missing SDA peripheral argument.");
    let scl = args
        .next()
        .expect_or_abort("Missing SCL peripheral argument.");

    if let Some(literal) = args.next() {
        abort!(literal.span(), "Unexpected extra arguments.")
    }

    quote! {
        use ::rumcake::hw::mcu::embassy_nrf::interrupt::InterruptExt;
        unsafe {
            ::rumcake::hw::mcu::embassy_nrf::bind_interrupts! {
                struct Irqs {
                    #interrupt => ::rumcake::hw::mcu::embassy_nrf::twim::InterruptHandler<::rumcake::hw::mcu::embassy_nrf::peripherals::#i2c>;
                }
            };
            ::rumcake::hw::mcu::embassy_nrf::interrupt::#interrupt.set_priority(::rumcake::hw::mcu::embassy_nrf::interrupt::Priority::P2);
            let i2c = ::rumcake::hw::mcu::embassy_nrf::peripherals::#i2c::steal();
            let sda = ::rumcake::hw::mcu::embassy_nrf::peripherals::#sda::steal();
            let scl = ::rumcake::hw::mcu::embassy_nrf::peripherals::#scl::steal();
            ::rumcake::hw::mcu::embassy_nrf::twim::Twim::new(i2c, Irqs, sda, scl, Default::default())
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

pub fn setup_i2c_blocking(args: Punctuated<Ident, Token![,]>) -> TokenStream {
    let inner = setup_i2c_inner(args);
    quote! {
        fn setup_i2c(
        ) -> impl ::rumcake::embedded_hal::blocking::i2c::Write<Error = impl core::fmt::Debug> {
            #inner
        }
    }
}

fn setup_buffered_uarte_inner(args: Punctuated<Ident, Token![,]>) -> TokenStream {
    let mut args = args.iter();

    let interrupt = args.next().expect_or_abort("Missing interrupt argument.");
    let uarte = args
        .next()
        .expect_or_abort("Missing UARTE peripheral argument.");
    let timer = args
        .next()
        .expect_or_abort("Missing timer peripheral argument.");
    let ppi_ch1 = args
        .next()
        .expect_or_abort("Missing PPI CH1 peripheral argument.");
    let ppi_ch2 = args
        .next()
        .expect_or_abort("Missing PPI CH2 peripheral argument.");
    let ppi_group = args
        .next()
        .expect_or_abort("Missing PPI Group peripheral argument.");
    let rx_pin = args.next().expect_or_abort("Missing RX pin argument.");
    let tx_pin = args.next().expect_or_abort("Missing TX pin argument.");

    quote! {
        unsafe {
            static mut RBUF: [u8; 4096] = [0; 4096];
            static mut TBUF: [u8; 4096] = [0; 4096];
            ::rumcake::hw::mcu::embassy_nrf::bind_interrupts! {
                struct Irqs {
                    #interrupt => ::rumcake::hw::mcu::embassy_nrf::buffered_uarte::InterruptHandler<::rumcake::hw::mcu::embassy_nrf::peripherals::#uarte>;
                }
            };
            let uarte = ::rumcake::hw::mcu::embassy_nrf::peripherals::#uarte::steal();
            let timer = ::rumcake::hw::mcu::embassy_nrf::peripherals::#timer::steal();
            let ppi_ch1 = ::rumcake::hw::mcu::embassy_nrf::peripherals::#ppi_ch1::steal();
            let ppi_ch2 = ::rumcake::hw::mcu::embassy_nrf::peripherals::#ppi_ch2::steal();
            let ppi_group = ::rumcake::hw::mcu::embassy_nrf::peripherals::#ppi_group::steal();
            let rx_pin = ::rumcake::hw::mcu::embassy_nrf::peripherals::#rx_pin::steal();
            let tx_pin = ::rumcake::hw::mcu::embassy_nrf::peripherals::#tx_pin::steal();
            ::rumcake::hw::mcu::embassy_nrf::buffered_uarte::BufferedUarte::new(
                uarte,
                timer,
                ppi_ch1,
                ppi_ch2,
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

pub fn setup_buffered_uarte(args: Punctuated<Ident, Token![,]>) -> TokenStream {
    let inner = setup_buffered_uarte_inner(args);

    quote! {
        fn setup_serial(
        ) -> impl ::rumcake::embedded_io_async::Write + ::rumcake::embedded_io_async::Read {
            #inner
        }
    }
}
pub struct NrfAdcSamplerDefinition {
    parenthesis_token: syn::token::Paren,
    adc_instance_args: Punctuated<Ident, Token![,]>,
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
            adc_instance_args: Punctuated::parse_terminated(&adc_type_content)?,
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
    let mut args = adc_instance_args.iter();
    let timer = args.next().expect_or_abort("Missing timer argument.");
    let ppi_ch0 = args.next().expect_or_abort("Missing PPI CH0 argument.");
    let ppi_ch1 = args.next().expect_or_abort("Missing PPI CH1 argument.");

    let channel_count = channels.len();
    let select_pin_count = channels.iter().fold(0, |acc, ch| {
        if let AnalogPinType::Multiplexed(MultiplexerDefinition { select_pins, .. }) = ch {
            acc.max(select_pins.len())
        } else {
            acc
        }
    });
    let buf_size = 2usize.pow(select_pin_count as u32);

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
                            [0; #buf_size],
                            ::rumcake::hw::Multiplexer::new(
                                [ #(#select_pins),* ],
                                None
                            )
                        )
                    },
                    quote! {
                        ::rumcake::hw::mcu::embassy_nrf::saadc::ChannelConfig::single_ended(
                            unsafe { ::rumcake::hw::mcu::embassy_nrf::peripherals::#pin::steal() }
                        )
                    },
                )
            }
            AnalogPinType::Direct(DirectPinDefinition { pin, .. }) => (
                quote! {
                    ::rumcake::hw::mcu::AnalogPinType::Direct([0])
                },
                quote! {
                    ::rumcake::hw::mcu::embassy_nrf::saadc::ChannelConfig::single_ended(
                        unsafe { ::rumcake::hw::mcu::embassy_nrf::peripherals::#pin::steal() }
                    )
                },
            ),
        })
        .unzip();

    quote! {
        type AdcSamplerType = ::rumcake::hw::mcu::AdcSampler<
            'static,
            ::rumcake::hw::mcu::embassy_nrf::peripherals::#timer,
            ::rumcake::hw::mcu::embassy_nrf::peripherals::#ppi_ch0,
            ::rumcake::hw::mcu::embassy_nrf::peripherals::#ppi_ch1,
            #select_pin_count,
            #channel_count,
        >;

        fn setup_adc_sampler() -> &'static AdcSamplerType {
            static SAMPLER: ::rumcake::once_cell::sync::OnceCell<AdcSamplerType> = ::rumcake::once_cell::sync::OnceCell::new();

            SAMPLER.get_or_init(|| unsafe {
                ::rumcake::hw::mcu::AdcSampler::new(
                    [ #(#pins),* ],
                    [ #(#channels),* ],
                    ::rumcake::hw::mcu::embassy_nrf::peripherals::#timer::steal(),
                    ::rumcake::hw::mcu::embassy_nrf::peripherals::#ppi_ch0::steal(),
                    ::rumcake::hw::mcu::embassy_nrf::peripherals::#ppi_ch1::steal(),
                )
            })
        }
    }
}
