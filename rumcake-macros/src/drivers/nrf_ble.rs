use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::ExprArray;

use crate::common::Row;

crate::parse_as_custom_fields! {
    pub struct NrfBlePeripheralArgsBuilder for NrfBlePeripheralArgs {
        central_address: ExprArray,
    }
}

pub fn setup_nrf_ble_split_peripheral(
    NrfBlePeripheralArgs { central_address }: NrfBlePeripheralArgs,
) -> TokenStream {
    quote! {
        (::rumcake::drivers::nrf_ble::peripheral::setup_driver(), #central_address)
    }
}

crate::parse_as_custom_fields! {
    pub struct NrfBleCentralArgsBuilder for NrfBleCentralArgs {
        peripheral_addresses: Row<ExprArray>,
    }
}

pub fn setup_nrf_ble_split_central(
    NrfBleCentralArgs {
        peripheral_addresses,
    }: NrfBleCentralArgs,
) -> TokenStream {
    let items = peripheral_addresses
        .items
        .iter()
        .map(|item| quote! { #item });

    quote! {
        (::rumcake::drivers::nrf_ble::central::setup_driver(), &[ #(#items),* ] )
    }
}
