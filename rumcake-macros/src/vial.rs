use proc_macro2::TokenStream;
use quote::quote;

pub fn enable_vial_rgb() -> TokenStream {
    quote! {
        const VIALRGB_ENABLE: bool = true;
        type BacklightMatrixDevice = Self;
        fn get_backlight_matrix() -> Option<
            ::rumcake::backlight::BacklightMatrix<
                { <Self::BacklightMatrixDevice as ::rumcake::backlight::BacklightMatrixDevice>::LIGHTING_COLS },
                { <Self::BacklightMatrixDevice as ::rumcake::backlight::BacklightMatrixDevice>::LIGHTING_ROWS },
            >,
        > {
            Some(<Self::BacklightMatrixDevice as ::rumcake::backlight::BacklightMatrixDevice>::get_backlight_matrix())
        }
    }
}
