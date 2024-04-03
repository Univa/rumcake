use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::LitInt;

crate::parse_as_custom_fields! {
    pub struct MacroBufferArgsBuilder for MacroBufferArgs {
        buffer_size: LitInt,
        macro_count: LitInt,
    }
}

pub fn setup_macro_buffer(
    MacroBufferArgs {
        buffer_size,
        macro_count,
    }: MacroBufferArgs,
) -> TokenStream {
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

pub fn connect_storage_service(ident: Ident) -> TokenStream {
    quote! {
        type StorageType = #ident;
        fn get_storage_service() -> Option<
            &'static ::rumcake::storage::StorageService<
                'static,
                <Self::StorageType as ::rumcake::storage::StorageDevice>::FlashStorageType,
                Self::StorageType,
            >,
        >
        where
            [(); <<Self::StorageType as ::rumcake::storage::StorageDevice>::FlashStorageType as ::rumcake::storage::FlashStorage>::ERASE_SIZE]:,
        {
            Some(<Self::StorageType as ::rumcake::storage::StorageDevice>::get_storage_service())
        }
    }
}
