use proc_macro2::{Ident, TokenStream};
use quote::quote;

pub fn enable_vial_rgb(ident: Ident) -> TokenStream {
    quote! {
        const VIALRGB_ENABLE: bool = true;
        type RGBBacklightMatrixDevice = #ident;
        fn get_backlight_matrix() -> Option<
            ::rumcake::lighting::BacklightMatrix<
                { <Self::RGBBacklightMatrixDevice as ::rumcake::lighting::BacklightMatrixDevice>::LIGHTING_COLS },
                { <Self::RGBBacklightMatrixDevice as ::rumcake::lighting::BacklightMatrixDevice>::LIGHTING_ROWS },
            >,
        > {
            Some(<Self::RGBBacklightMatrixDevice as ::rumcake::lighting::BacklightMatrixDevice>::get_backlight_matrix())
        }
    }
}
