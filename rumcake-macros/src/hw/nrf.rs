use proc_macro2::{Ident, TokenStream};
use proc_macro_error::{abort, OptionExt};
use quote::quote;
use syn::punctuated::Punctuated;
use syn::Token;

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
