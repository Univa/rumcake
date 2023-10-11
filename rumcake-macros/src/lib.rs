use darling::FromMeta;
use heck::{ToShoutySnakeCase, ToSnakeCase};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
    parse_macro_input, parse_quote, parse_quote_spanned, parse_str, DeriveInput, ItemEnum, ItemFn,
    ItemStruct, LitStr, Meta, Pat, Token,
};

struct Templates(Punctuated<LitStr, Token![,]>);

impl Parse for Templates {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Templates(
            Punctuated::<LitStr, Token![,]>::parse_separated_nonempty(input)?,
        ))
    }
}

fn process_template(template: &str, name: &str) -> String {
    template
        .replace("{variant}", name)
        .replace("{variant_snake_case}", &name.to_snake_case())
        .replace("{variant_shouty_snake_case}", &name.to_shouty_snake_case())
}

#[proc_macro_attribute]
pub fn generate_items_from_enum_variants(
    a: proc_macro::TokenStream,
    e: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args = parse_macro_input!(a as Templates).0;
    let mut item = parse_macro_input!(e as ItemEnum);
    let enum_name = item.ident.clone().to_string().to_snake_case();
    let macro_name = format_ident!("{}_items", enum_name);

    let members: Vec<TokenStream> = args
        .iter()
        .flat_map(|t| {
            item.variants
                .iter_mut()
                .flat_map(|variant| -> Vec<TokenStream> {
                    let mut streams: Vec<TokenStream> = Vec::new();
                    let variant_name = variant.ident.to_string();

                    let rendered = process_template(&t.value(), &variant_name);

                    // Generate variant-specific items
                    if let Some(idx) = variant
                        .attrs
                        .iter()
                        .position(|v| v.path().is_ident("generate_items"))
                    {
                        if let Meta::List(list) = variant.attrs.remove(idx).meta.clone() {
                            let tokens: proc_macro::TokenStream = list.tokens.clone().into();
                            match syn::parse::<Templates>(tokens) {
                                Ok(data) => {
                                    data.0.iter().for_each(|t| {
                                        streams.push(
                                            parse_str(&process_template(
                                                &t.value(),
                                                &variant_name.clone(),
                                            ))
                                            .unwrap(),
                                        );
                                    });
                                }
                                Err(_err) => streams.push(quote_spanned! {
                                    list.span() => compile_error!("Could not parse item.")
                                }),
                            };
                        };
                    };

                    streams.push(parse_str(&rendered).unwrap());

                    streams
                })
                .collect::<Vec<TokenStream>>()
        })
        .collect();

    quote! {
        #item

        macro_rules! #macro_name {
            () => {
                #(#members;)*
            }
        }

        pub(crate) use #macro_name;
    }
    .into()
}

#[proc_macro_derive(LEDEffect, attributes(animated))]
pub fn derive_ledeffect(e: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let item = parse_macro_input!(e as DeriveInput);
    let enum_name = item.ident.clone();
    let variant_results: Vec<TokenStream> = if let syn::Data::Enum(e) = item.data {
        e.variants
            .clone()
            .iter()
            .map(|variant| {
                let variant_name = variant.ident.clone();
                let result = variant
                    .attrs
                    .iter()
                    .any(|cur| cur.path().is_ident("animated"));

                quote! {
                    #enum_name::#variant_name => #result
                }
            })
            .collect()
    } else {
        vec![quote_spanned! {
            item.span() => compile_error!("LEDEffect can only be derived on enums.")
        }]
    };

    quote! {
        impl LEDEffect for #enum_name {
            fn is_animated(&self) -> bool {
                match self {
                    #(#variant_results),*
                }
            }
        }
    }
    .into()
}

#[proc_macro_derive(Cycle)]
pub fn derive_cycle(e: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let item = parse_macro_input!(e as DeriveInput);
    let enum_name = item.ident.clone();
    let idents = if let syn::Data::Enum(e) = item.data {
        e.variants
            .clone()
            .iter()
            .map(|v| v.ident.clone())
            .collect::<Vec<Ident>>()
    } else {
        vec![parse_quote_spanned! {
            item.span() => compile_error!("Cycle can only be derived on enums.")
        }]
    };

    let mut incremented = idents.clone();
    incremented.rotate_left(1);
    let mut decremented = idents.clone();
    decremented.rotate_right(1);

    quote! {
        impl Cycle for #enum_name {
            fn increment(&mut self) {
                *self = match self {
                    #(#enum_name::#idents => #enum_name::#incremented),*
                }
            }

            fn decrement(&mut self) {
                *self = match self {
                    #(#enum_name::#idents => #enum_name::#decremented),*
                }
            }
        }
    }
    .into()
}

#[derive(Debug, FromMeta)]
struct KeyboardArgs {
    #[darling(default)]
    bluetooth: bool,
    #[darling(default)]
    usb: bool,
    #[darling(default)]
    backlight: Option<String>,
    #[darling(default)]
    underglow: Option<String>,
    #[darling(default)]
    display: Option<String>,
    #[darling(default)]
    split_peripheral: Option<String>,
    #[darling(default)]
    split_central: Option<String>,
}

enum SplitRole {
    Central,
    Peripheral,
}

fn setup_split_driver(kb_name: &Ident, driver: &str, role: SplitRole) -> Option<TokenStream> {
    match driver {
        #[cfg(feature = "nrf")]
        "ble" => match role {
            SplitRole::Central => Some(quote! {
                spawner.spawn(rumcake_drivers::nrf_ble_central_task!(#kb_name, sd)).unwrap();
                let split_central_driver = rumcake_drivers::nrf_ble::central::setup_split_central_driver(#kb_name);
            }),
            SplitRole::Peripheral => Some(quote! {
                let peripheral_server = rumcake_drivers::nrf_ble::peripheral::PeripheralDeviceServer::new(sd).unwrap();
                spawner.spawn(rumcake_drivers::nrf_ble_peripheral_task!((#kb_name), (sd, peripheral_server))).unwrap();
                let split_peripheral_driver = rumcake_drivers::nrf_ble::peripheral::setup_split_peripheral_driver::<#kb_name>();
            }),
        },
        _ => None,
    }
}

fn setup_underglow_driver(kb_name: &Ident, driver: &str) -> Option<TokenStream> {
    match driver {
        "ws2812_bitbang" => Some(quote! {
            let underglow_driver = rumcake_drivers::ws2812_bitbang::setup_underglow_driver::<#kb_name>().await;
        }),
        _ => None,
    }
}

fn setup_backlight_driver(kb_name: &Ident, driver: &str) -> Option<TokenStream> {
    match driver {
        "is31fl3731" => Some(quote! {
            let backlight_driver = rumcake_drivers::is31fl3731::setup_backlight_driver::<#kb_name>().await;
        }),
        "ws2812_bitbang" => Some(quote! {
            let backlight_driver = rumcake_drivers::ws2812_bitbang::setup_backlight_driver::<#kb_name>().await;
        }),
        _ => None,
    }
}

fn setup_display_driver(kb_name: &Ident, driver: &str) -> Option<TokenStream> {
    match driver {
        "ssd1306" => Some(quote! {
            let display_driver = rumcake_drivers::ssd1306::setup_display_driver(#kb_name).await;
        }),
        _ => None,
    }
}

#[proc_macro_attribute]
pub fn main(
    args: proc_macro::TokenStream,
    str: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let str = parse_macro_input!(str as ItemStruct);
    let kb_name = str.ident.clone();

    let args = darling::ast::NestedMeta::parse_meta_list(args.into()).unwrap();
    let keyboard = KeyboardArgs::from_list(&args).unwrap();

    let mut initialization = TokenStream::new();
    let mut spawning = TokenStream::new();

    // Setup microcontroller
    initialization.extend(quote! {
        rumcake::hw::mcu::initialize_rcc();
    });

    // Keyboard setup, and matrix polling task
    initialization.extend(quote! {
        let (matrix, debouncer) = rumcake::setup_keyboard_matrix!(#kb_name);
    });
    spawning.extend(quote! {
        spawner
            .spawn(rumcake::matrix_poll!((#kb_name), (matrix, debouncer)))
            .unwrap();
    });

    #[cfg(feature = "nrf")]
    {
        spawning.extend(quote! {
            spawner.spawn(rumcake::adc_task!()).unwrap();
        });

        if keyboard.bluetooth
            || keyboard
                .split_peripheral
                .as_ref()
                .is_some_and(|driver| driver == "ble")
            || keyboard
                .split_central
                .as_ref()
                .is_some_and(|driver| driver == "ble")
        {
            initialization.extend(quote! {
                let sd = rumcake::hw::mcu::setup_softdevice::<#kb_name>();
            });
            spawning.extend(quote! {
                spawner.spawn(rumcake::softdevice_task!(sd)).unwrap();
            });
        }
    }

    if keyboard.bluetooth || keyboard.usb {
        initialization.extend(quote! {
            let layout = rumcake::setup_keyboard_layout!(#kb_name);
        });
        spawning.extend(quote! {
            spawner.spawn(rumcake::layout_collect!((#kb_name), (layout))).unwrap();
            spawner
                .spawn(rumcake::layout_register!((#kb_name), (layout)))
                .unwrap();
        })
    }

    // Split keyboard setup
    if let Some(ref driver) = keyboard.split_peripheral {
        match setup_split_driver(&kb_name, driver.as_str(), SplitRole::Peripheral) {
            Some(driver_setup) => {
                initialization.extend(driver_setup);
                spawning.extend(quote! {
                    spawner.spawn(rumcake::peripheral_task!((#kb_name), (split_peripheral_driver))).unwrap();
                });
            }
            None => {
                initialization.extend(quote_spanned! {
                    keyboard.split_peripheral.span() => compile_error!("Unknown split peripheral device driver.");
                });
            }
        }
    }

    if let Some(ref driver) = keyboard.split_central {
        match setup_split_driver(&kb_name, driver.as_str(), SplitRole::Central) {
            Some(driver_setup) => {
                initialization.extend(driver_setup);
                spawning.extend(quote! {
                    spawner.spawn(rumcake::central_task!((#kb_name), (split_central_driver, layout))).unwrap();
                });
            }
            None => {
                initialization.extend(quote_spanned! {
                    keyboard.split_central.span() => compile_error!("Unknown split central device driver.");
                });
            }
        }
    }

    #[cfg(feature = "nrf")]
    if keyboard.bluetooth {
        initialization.extend(quote! {
            let hid_server = rumcake::nrf_ble::Server::new(sd).unwrap();
        });
        spawning.extend(quote! {
            spawner.spawn(rumcake::nrf_ble_task!((#kb_name), (sd, hid_server))).unwrap();
        });
    }

    // USB Configuration
    if keyboard.usb {
        initialization.extend(quote! {
            let mut builder = rumcake::hw::mcu::setup_usb_driver::<#kb_name>();

            // HID Class setup
            let kb_class = rumcake::usb::setup_usb_hid_nkro_writer(&mut builder);
        });
        spawning.extend(quote! {
            let usb = builder.build();

            // Task spawning
            // Initialize USB device
            spawner.spawn(rumcake::start_usb!(usb)).unwrap();

            // HID Keyboard Report sending
            spawner.spawn(rumcake::usb_hid_kb_write_task!(kb_class)).unwrap();
        });
    }

    #[cfg(feature = "eeprom")]
    initialization.extend(quote! {
        // Flash setup
        let raw_hid_flash = rumcake::hw::mcu::setup_flash();
    });

    // The appropriate via/vial request handler built by `setup_raw_hid_request_handler` is chosen based on the feature flags set on `rumcake`.
    #[cfg(feature = "via")]
    {
        initialization.extend(quote! {
            // Via HID setup
            let (via_reader, via_writer) =
                rumcake::via::setup_usb_via_hid_reader_writer(&mut builder).split();
        });
        spawning.extend(quote! {
            // HID raw report (for VIA) reading and writing
            spawner
                .spawn(rumcake::usb_hid_via_read_task!(via_reader))
                .unwrap();
        })
    }

    #[cfg(all(feature = "via", not(feature = "vial")))]
    spawning.extend(quote! {
        spawner.spawn(rumcake::usb_hid_via_write_task!((#kb_name), (debouncer, raw_hid_flash, via_writer))).unwrap();
    });

    #[cfg(feature = "vial")]
    spawning.extend(quote! {
        spawner
            .spawn(rumcake::usb_hid_vial_write_task!(
                (
                    { #kb_name::KEYBOARD_DEFINITION.len() },
                    { #kb_name::VIAL_UNLOCK_COMBO.len() },
                    #kb_name
                ),
                (debouncer, raw_hid_flash, via_writer)
            ))
            .unwrap();
    });

    // Underglow setup
    if let Some(ref driver) = keyboard.underglow {
        match setup_underglow_driver(&kb_name, driver.as_str()) {
            Some(driver_setup) => {
                initialization.extend(driver_setup);
                spawning.extend(quote! {
                    spawner.spawn(rumcake::underglow_task!((#kb_name), (underglow_driver))).unwrap();
                });
            }
            None => {
                initialization.extend(quote_spanned! {
                    keyboard.underglow.span() => compile_error!("Unknown underglow driver.");
                });
            }
        }
    }

    // Backlight setup
    if let Some(ref driver) = keyboard.backlight {
        match setup_backlight_driver(&kb_name, driver.as_str()) {
            Some(driver_setup) => {
                initialization.extend(driver_setup);
                spawning.extend(quote! {
                    spawner.spawn(rumcake::backlight_task!((#kb_name), (backlight_driver))).unwrap();
                });
            }
            None => {
                initialization.extend(quote_spanned! {
                    keyboard.backlight.span() => compile_error!("Unknown backlight driver.");
                });
            }
        }
    }

    // Display setup
    if let Some(ref driver) = keyboard.display {
        match setup_display_driver(&kb_name, driver.as_str()) {
            Some(driver_setup) => {
                initialization.extend(driver_setup);
                spawning.extend(quote! {
                    spawner.spawn(rumcake::display_task!((#kb_name), (display_driver))).unwrap();
                });
            }
            None => {
                initialization.extend(quote_spanned! {
                    keyboard.display.span() => compile_error!("Unknown display driver.");
                });
            }
        }
    }

    quote! {
        #[rumcake::embassy_executor::main]
        async fn main(spawner: rumcake::embassy_executor::Spawner) {
            #initialization
            #spawning
        }

        #str
    }
    .into()
}

#[proc_macro_attribute]
pub fn task(
    _args: proc_macro::TokenStream,
    fun: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let fun = parse_macro_input!(fun as ItemFn);

    // for the outer macro
    let task_ident = fun.sig.ident.clone();
    // let generics = fun.sig.generics.clone();
    let gen_names: Vec<Ident> = fun
        .sig
        .generics
        .clone()
        .params
        .iter_mut()
        .filter_map(|p| match p {
            syn::GenericParam::Type(t) => Some(t.ident.clone()),
            syn::GenericParam::Const(c) => Some(c.ident.clone()),
            _ => None,
        })
        .collect();
    // let args = fun.sig.inputs.clone();
    // let wc = fun.sig.generics.where_clause.clone();

    // Copy the function and change the identifier
    let mut inner = fun.clone();
    let task_name = inner.sig.ident;
    inner.sig.ident = format_ident!("__{}_task", task_name);

    let task_name_string = task_name.to_string();
    inner.block.stmts.insert(
        0,
        parse_quote! {
            defmt::debug!("{} has spawned.", #task_name_string);
        },
    );
    let inner_ident = inner.sig.ident.clone();

    // Arguments to pass to the inner task
    let arg_names: Vec<Ident> = fun
        .sig
        .inputs
        .clone()
        .iter_mut()
        .filter_map(|a| match a {
            syn::FnArg::Typed(t) => match t.pat.as_mut() {
                Pat::Ident(i) => Some(i.ident.clone()),
                _ => None,
            },
            _ => None,
        })
        .collect();

    quote! {
        #inner

        #[macro_export]
        macro_rules! #task_ident {
            ((#($#gen_names:tt),*), (#($#arg_names:tt),*)) => {
                {
                    type Fut = impl ::core::future::Future + 'static;
                    static POOL: $crate::embassy_executor::raw::TaskPool<Fut, 1> = $crate::embassy_executor::raw::TaskPool::new();
                    unsafe { POOL._spawn_async_fn(|| $crate::tasks::#inner_ident::<#($#gen_names),*>(#($#arg_names,)*)) }
                }
            };
            (#($#arg_names:tt),*) => {
                {
                    type Fut = impl ::core::future::Future + 'static;
                    static POOL: $crate::embassy_executor::raw::TaskPool<Fut, 1> = $crate::embassy_executor::raw::TaskPool::new();
                    unsafe { POOL._spawn_async_fn(|| $crate::tasks::#inner_ident(#($#arg_names,)*)) }
                }
            };
        }
    }
    .into()
}
