use proc_macro2::{Literal, TokenStream};
use proc_macro_error::{abort, OptionExt};
use quote::quote;
use syn::punctuated::Punctuated;
use syn::Token;

pub fn setup_macro_buffer(args: Punctuated<Literal, Token![,]>) -> TokenStream {
    let mut args = args.iter();

    let buffer_size = args.next().expect_or_abort("Missing buffer size argument.");
    let macro_count = args.next().expect_or_abort("Missing macro count argument.");

    if let Some(literal) = args.next() {
        abort!(literal.span(), "Unexpected extra arguments.")
    }

    quote! {
        const DYNAMIC_KEYMAP_MACRO_BUFFER_SIZE: u16 = #buffer_size;
        const DYNAMIC_KEYMAP_MACRO_COUNT: u8 = #macro_count;

        fn get_macro_buffer() -> Option<
            &'static mut ::rumcake::via::MacroBuffer<
                'static,
                { Self::DYNAMIC_KEYMAP_MACRO_BUFFER_SIZE as usize },
                { Self::DYNAMIC_KEYMAP_MACRO_COUNT as usize },
            >,
        > {
            static mut MACRO_BUFFER: ::rumcake::via::MacroBuffer<'static, #buffer_size, #macro_count> =
                ::rumcake::via::MacroBuffer::new();
            Some(unsafe { &mut MACRO_BUFFER })
        }
    }
}
