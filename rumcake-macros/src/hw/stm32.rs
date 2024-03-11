use proc_macro2::{Ident, TokenStream};
use proc_macro_error::{abort, OptionExt};
use quote::quote;
use syn::punctuated::Punctuated;
use syn::Token;

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
