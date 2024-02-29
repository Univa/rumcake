use proc_macro2::TokenStream;
use quote::quote;

pub fn peripheral_driver_trait() -> TokenStream {
    quote! {
        /// A trait that nRF-based keyboards must implement to use bluetooth to drive peripheral devices in a split keyboard setup.
        pub(crate) trait NRFBLEPeripheralDriverSettings {
            /// A "Random Static" bluetooth address of the central device that this peripheral will connect to.
            const CENTRAL_ADDRESS: [u8; 6];
        }
    }
}

pub fn central_driver_trait() -> TokenStream {
    quote! {
        /// A trait that nRF-based keyboards must implement to use bluetooth to drive central devices in a split keyboard setup.
        pub(crate) trait NRFBLECentralDriverSettings {
            /// A list of "Random Static" bluetooth addresses that this central device can connect to.
            const PERIPHERAL_ADDRESSES: &'static [[u8; 6]];
        }
    }
}
