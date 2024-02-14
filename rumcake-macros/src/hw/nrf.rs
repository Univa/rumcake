use proc_macro2::{Ident, TokenStream};
use proc_macro_error::{abort, OptionExt};
use quote::quote;
use syn::punctuated::Punctuated;
use syn::Token;

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
