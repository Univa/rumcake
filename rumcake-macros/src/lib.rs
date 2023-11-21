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

#[proc_macro_derive(LEDEffect, attributes(animated, reactive))]
pub fn derive_ledeffect(e: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let item = parse_macro_input!(e as DeriveInput);
    let enum_name = item.ident.clone();
    let (animated_results, reactive_results): (TokenStream, TokenStream) =
        if let syn::Data::Enum(e) = item.data {
            let mut animated_tokens = TokenStream::new();
            let mut reactive_tokens = TokenStream::new();

            for variant in e.variants.clone().iter() {
                let variant_name = variant.ident.clone();
                let (is_animated, is_reactive) =
                    variant.attrs.iter().fold((false, false), |mut acc, attr| {
                        if attr.path().is_ident("animated") {
                            acc.0 = true;
                        }
                        if attr.path().is_ident("reactive") {
                            acc.1 = true;
                        }
                        acc
                    });

                animated_tokens.extend(quote! {
                    #enum_name::#variant_name => #is_animated,
                });
                reactive_tokens.extend(quote! {
                    #enum_name::#variant_name => #is_reactive,
                })
            }

            (animated_tokens, reactive_tokens)
        } else {
            (
                quote_spanned! {
                    item.span() => _ => compile_error!("LEDEffect can only be derived on enums.")
                },
                TokenStream::new(),
            )
        };

    quote! {
        impl LEDEffect for #enum_name {
            fn is_animated(&self) -> bool {
                match self {
                    #animated_results
                }
            }

            fn is_reactive(&self) -> bool {
                match self {
                    #reactive_results
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
    no_matrix: bool,
    #[darling(default)]
    no_storage: bool,
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
    storage: Option<String>,
    #[darling(default)]
    split_peripheral: Option<String>,
    #[darling(default)]
    split_central: Option<String>,
    #[darling(default)]
    via: bool,
    #[darling(default)]
    vial: bool,
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
                spawner.spawn(rumcake::nrf_ble_central_task!(#kb_name, sd)).unwrap();
                let split_central_driver = rumcake::drivers::nrf_ble::central::setup_split_central_driver(#kb_name);
            }),
            SplitRole::Peripheral => Some(quote! {
                let peripheral_server = rumcake::drivers::nrf_ble::peripheral::PeripheralDeviceServer::new(sd).unwrap();
                spawner.spawn(rumcake::nrf_ble_peripheral_task!(#kb_name, sd, peripheral_server)).unwrap();
                let split_peripheral_driver = rumcake::drivers::nrf_ble::peripheral::setup_split_peripheral_driver::<#kb_name>();
            }),
        },
        _ => None,
    }
}

fn setup_underglow_driver(kb_name: &Ident, driver: &str) -> Option<TokenStream> {
    match driver {
        "ws2812_bitbang" => Some(quote! {
            let underglow_driver = rumcake::drivers::ws2812_bitbang::underglow::setup_underglow_driver::<#kb_name>().await;
        }),
        _ => None,
    }
}

fn setup_backlight_driver(kb_name: &Ident, driver: &str) -> Option<TokenStream> {
    match driver {
        "is31fl3731" => Some(quote! {
            let backlight_driver = rumcake::drivers::is31fl3731::backlight::setup_backlight_driver::<#kb_name>().await;
        }),
        "ws2812_bitbang" => Some(quote! {
            let backlight_driver = rumcake::drivers::ws2812_bitbang::backlight::setup_backlight_driver::<#kb_name>().await;
        }),
        _ => None,
    }
}

fn setup_display_driver(kb_name: &Ident, driver: &str) -> Option<TokenStream> {
    match driver {
        "ssd1306" => Some(quote! {
            let display_driver = rumcake::drivers::ssd1306::display::setup_display_driver(#kb_name).await;
        }),
        _ => None,
    }
}

fn setup_storage_driver(driver: &str, uses_bluetooth: bool) -> Option<TokenStream> {
    match driver {
        "internal" => {
            if cfg!(feature = "nrf") && uses_bluetooth {
                Some(quote! {
                    use rumcake::embedded_storage_async::nor_flash::NorFlash;
                    let flash = rumcake::hw::mcu::setup_internal_softdevice_flash(sd);
                    let config_start = unsafe { &rumcake::hw::__config_start as *const u32 as usize };
                    let config_end = unsafe { &rumcake::hw::__config_end as *const u32 as usize };
                    static mut READ_BUF: [u8; rumcake::hw::mcu::nrf_softdevice::ERASE_SIZE] = [0; rumcake::hw::mcu::nrf_softdevice::ERASE_SIZE];
                    static DATABASE: rumcake::storage::Database<'static, rumcake::hw::nrf_softdevice::Flash> = rumcake::storage::Database::new();
                    DATABASE.setup(flash, config_start, config_end, unsafe { &mut READ_BUF }).await;
                })
            } else {
                Some(quote! {
                    use rumcake::embedded_storage_async::nor_flash::NorFlash;
                    let flash = rumcake::hw::mcu::setup_internal_flash();
                    let config_start = unsafe { &rumcake::hw::__config_start as *const u32 as usize };
                    let config_end = unsafe { &rumcake::hw::__config_end as *const u32 as usize };
                    static mut READ_BUF: [u8; rumcake::hw::mcu::Flash::ERASE_SIZE] = [0; rumcake::hw::mcu::Flash::ERASE_SIZE];
                    static DATABASE: rumcake::storage::Database<'static, rumcake::hw::mcu::Flash> = rumcake::storage::Database::new();
                    DATABASE.setup(flash, config_start, config_end, unsafe { &mut READ_BUF }).await;
                })
            }
        }
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
    let mut keyboard = KeyboardArgs::from_list(&args).unwrap();

    #[cfg(not(feature = "storage"))]
    {
        keyboard.no_storage = true;
    }

    let mut initialization = TokenStream::new();
    let mut spawning = TokenStream::new();

    let uses_bluetooth = keyboard.bluetooth
        || keyboard
            .split_peripheral
            .as_ref()
            .is_some_and(|driver| driver == "ble")
        || keyboard
            .split_central
            .as_ref()
            .is_some_and(|driver| driver == "ble");

    // Setup microcontroller
    initialization.extend(quote! {
        rumcake::hw::mcu::initialize_rcc();
    });

    // Keyboard setup, and matrix polling task
    if !keyboard.no_matrix {
        initialization.extend(quote! {
            let (matrix, debouncer) = rumcake::keyboard::setup_keyboard_matrix(#kb_name);
        });
        spawning.extend(quote! {
            spawner
                .spawn(rumcake::matrix_poll!(#kb_name, matrix, debouncer))
                .unwrap();
        });
    }

    #[cfg(feature = "nrf")]
    {
        spawning.extend(quote! {
            spawner.spawn(rumcake::adc_task!()).unwrap();
        });

        if uses_bluetooth {
            initialization.extend(quote! {
                let sd = rumcake::hw::mcu::setup_softdevice::<#kb_name>();
            });
            spawning.extend(quote! {
                spawner.spawn(rumcake::softdevice_task!(sd)).unwrap();
            });
        }
    }

    // Flash setup
    if !keyboard.no_storage {
        // Default to internal flash if a driver is not specified
        let driver = if let Some(driver) = keyboard.storage {
            driver
        } else {
            "internal".to_string()
        };
        let driver_setup = setup_storage_driver(driver.as_str(), uses_bluetooth);

        initialization.extend(driver_setup);
    }

    if keyboard.bluetooth || keyboard.usb {
        spawning.extend(quote! {
            spawner.spawn(rumcake::layout_collect!(#kb_name)).unwrap();
        });
    }

    #[cfg(feature = "nrf")]
    if keyboard.bluetooth {
        initialization.extend(quote! {
            let hid_server = rumcake::bluetooth::nrf_ble::Server::new(sd).unwrap();
        });
        spawning.extend(quote! {
            spawner.spawn(rumcake::nrf_ble_task!(#kb_name, sd, hid_server)).unwrap();
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

    if keyboard.via || keyboard.vial {
        initialization.extend(quote! {
            // Via HID setup
            let (via_reader, via_writer) =
                rumcake::via::setup_usb_via_hid_reader_writer(&mut builder).split();
        });

        if !keyboard.no_storage {
            spawning.extend(quote! {
                spawner
                    .spawn(rumcake::via_storage_task!(#kb_name, &DATABASE))
                    .unwrap();
            });
        }

        spawning.extend(quote! {
            // HID raw report (for VIA) reading and writing
            spawner
                .spawn(rumcake::usb_hid_via_read_task!(via_reader))
                .unwrap();
        });
    }

    if keyboard.via && !keyboard.vial {
        spawning.extend(quote! {
            spawner.spawn(rumcake::usb_hid_via_write_task!(#kb_name, via_writer)).unwrap();
        });
    }

    if keyboard.vial {
        if !keyboard.no_storage {
            spawning.extend(quote! {
                spawner
                    .spawn(rumcake::vial_storage_task!(#kb_name, &DATABASE))
                    .unwrap();
            });
        }

        spawning.extend(quote! {
            spawner
                .spawn(rumcake::usb_hid_vial_write_task!(#kb_name, via_writer))
                .unwrap();
        });
    }

    // Split keyboard setup
    if let Some(ref driver) = keyboard.split_peripheral {
        match setup_split_driver(&kb_name, driver.as_str(), SplitRole::Peripheral) {
            Some(driver_setup) => {
                initialization.extend(driver_setup);
                spawning.extend(quote! {
                    spawner.spawn(rumcake::peripheral_task!(#kb_name, split_peripheral_driver)).unwrap();
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
                    spawner.spawn(rumcake::central_task!(#kb_name, split_central_driver)).unwrap();
                });
            }
            None => {
                initialization.extend(quote_spanned! {
                    keyboard.split_central.span() => compile_error!("Unknown split central device driver.");
                });
            }
        }
    }

    // Underglow setup
    if let Some(ref driver) = keyboard.underglow {
        match setup_underglow_driver(&kb_name, driver.as_str()) {
            Some(driver_setup) => {
                initialization.extend(driver_setup);

                if !keyboard.no_storage {
                    spawning.extend(quote! {
                        spawner.spawn(rumcake::underglow_storage_task!(&DATABASE)).unwrap();
                    });
                }

                spawning.extend(quote! {
                    spawner.spawn(rumcake::underglow_task!(#kb_name, underglow_driver)).unwrap();
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

                if !keyboard.no_storage {
                    spawning.extend(quote! {
                        spawner.spawn(rumcake::backlight_storage_task!(&DATABASE)).unwrap();
                    });
                }

                spawning.extend(quote! {
                    spawner.spawn(rumcake::backlight_task!(#kb_name, backlight_driver)).unwrap();
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
                    spawner.spawn(rumcake::display_task!(#kb_name, display_driver)).unwrap();
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

    // Copy the function and change the identifier
    let mut inner = fun.clone();
    let task_name = inner.sig.ident;
    inner.sig.ident = format_ident!("__{}_task", task_name);

    let task_name_string = task_name.to_string();
    inner.block.stmts.insert(
        0,
        parse_quote! {
            defmt::info!("{} has spawned.", #task_name_string);
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
            (#($#arg_names:expr),*) => {
                {
                    type Fut = impl ::core::future::Future + 'static;
                    static POOL: $crate::embassy_executor::raw::TaskPool<Fut, 1> = $crate::embassy_executor::raw::TaskPool::new();
                    unsafe { POOL._spawn_async_fn(move || $crate::tasks::#inner_ident(#($#arg_names,)*)) }
                }
            };
        }
    }
    .into()
}
