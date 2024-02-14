use darling::util::Override;
use darling::FromMeta;
use proc_macro2::{Ident, TokenStream, TokenTree};
use proc_macro_error::OptionExt;
use quote::{quote, quote_spanned, ToTokens};
use syn::parse::{Parse, Parser};
use syn::spanned::Spanned;
use syn::{braced, bracketed, ItemStruct};

#[derive(Debug, FromMeta, Default)]
#[darling(default)]
pub(crate) struct KeyboardSettings {
    no_matrix: bool,
    bluetooth: bool,
    usb: bool,
    storage: Option<String>,
    simple_backlight: Option<LightingSettings>,
    simple_backlight_matrix: Option<LightingSettings>,
    rgb_backlight_matrix: Option<LightingSettings>,
    underglow: Option<LightingSettings>,
    display: Option<DisplaySettings>,
    split_peripheral: Option<SplitPeripheralSettings>,
    split_central: Option<SplitCentralSettings>,
    via: Option<Override<ViaSettings>>,
    vial: Option<Override<ViaSettings>>,
}

#[derive(Debug, FromMeta, Default)]
#[darling(default)]
pub(crate) struct LightingSettings {
    driver: String,
    use_storage: bool,
}

#[derive(Debug, FromMeta, Default)]
#[darling(default)]
pub(crate) struct DisplaySettings {
    driver: String,
}

#[derive(Debug, FromMeta, Default)]
#[darling(default)]
pub(crate) struct SplitCentralSettings {
    driver: String,
}

#[derive(Debug, FromMeta, Default)]
#[darling(default)]
pub(crate) struct SplitPeripheralSettings {
    driver: String,
}

#[derive(Debug, FromMeta, Default)]
#[darling(default)]
pub(crate) struct ViaSettings {
    use_storage: bool,
}

enum SplitRole {
    Central,
    Peripheral,
}

fn setup_split_driver(
    kb_name: &Ident,
    driver: &str,
    role: SplitRole,
) -> Option<(TokenStream, TokenStream)> {
    match driver {
        #[cfg(feature = "nrf")]
        "ble" => match role {
            SplitRole::Central => Some((
                quote! {
                    let split_central_driver = ::rumcake::drivers::nrf_ble::central::setup_split_central_driver(#kb_name);
                },
                quote! {
                    spawner.spawn(::rumcake::nrf_ble_central_task!(#kb_name, sd)).unwrap();
                },
            )),
            SplitRole::Peripheral => Some((
                quote! {
                    let peripheral_server = ::rumcake::drivers::nrf_ble::peripheral::PeripheralDeviceServer::new(sd).unwrap();
                    let split_peripheral_driver = ::rumcake::drivers::nrf_ble::peripheral::setup_split_peripheral_driver::<#kb_name>();
                },
                quote! {
                    spawner.spawn(::rumcake::nrf_ble_peripheral_task!(#kb_name, sd, peripheral_server)).unwrap();
                },
            )),
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

enum BacklightType {
    SimpleBacklight,
    SimpleBacklightMatrix,
    RGBBacklightMatrix,
}

fn setup_backlight_driver(
    kb_name: &Ident,
    backlight_type: BacklightType,
    driver: &str,
) -> Option<TokenStream> {
    match driver {
        "is31fl3731" => Some(quote! {
            let backlight_driver = ::rumcake::drivers::is31fl3731::backlight::setup_backlight_driver::<#kb_name>().await;
        }),
        "ws2812_bitbang" => Some(quote! {
            let backlight_driver = ::rumcake::drivers::ws2812_bitbang::backlight::setup_backlight_driver::<#kb_name>().await;
        }),
        _ => None,
    }
}

fn setup_display_driver(kb_name: &Ident, driver: &str) -> Option<TokenStream> {
    match driver {
        "ssd1306" => Some(quote! {
            let display_driver = ::rumcake::drivers::ssd1306::display::setup_display_driver(#kb_name).await;
        }),
        _ => None,
    }
}

fn setup_storage_driver(driver: &str, uses_bluetooth: bool) -> Option<TokenStream> {
    match driver {
        "internal" => {
            if cfg!(feature = "nrf") && uses_bluetooth {
                Some(quote! {
                    use ::rumcake::embedded_storage_async::nor_flash::NorFlash;
                    let flash = ::rumcake::hw::mcu::setup_internal_softdevice_flash(sd);
                    let config_start = unsafe { &::rumcake::hw::__config_start as *const u32 as usize };
                    let config_end = unsafe { &::rumcake::hw::__config_end as *const u32 as usize };
                    static mut READ_BUF: [u8; ::rumcake::hw::mcu::nrf_softdevice::ERASE_SIZE] = [0; ::rumcake::hw::mcu::nrf_softdevice::ERASE_SIZE];
                    static mut OP_BUF: [u8; ::rumcake::hw::mcu::nrf_softdevice::ERASE_SIZE] = [0; ::rumcake::hw::mcu::nrf_softdevice::ERASE_SIZE];
                    static DATABASE: ::rumcake::storage::StorageService<'static, ::rumcake::hw::nrf_softdevice::Flash> = ::rumcake::storage::Database::new();
                    unsafe { DATABASE.setup(flash, config_start, config_end, &mut read_buf, &mut op_buf).await; }
                })
            } else {
                Some(quote! {
                    use ::rumcake::embedded_storage_async::nor_flash::NorFlash;
                    let flash = ::rumcake::hw::mcu::setup_internal_flash();
                    let config_start = unsafe { &::rumcake::hw::__config_start as *const u32 as usize };
                    let config_end = unsafe { &::rumcake::hw::__config_end as *const u32 as usize };
                    static mut READ_BUF: [u8; ::rumcake::hw::mcu::Flash::ERASE_SIZE] = [0; ::rumcake::hw::mcu::Flash::ERASE_SIZE];
                    static mut OP_BUF: [u8; ::rumcake::hw::mcu::Flash::ERASE_SIZE] = [0; ::rumcake::hw::mcu::Flash::ERASE_SIZE];
                    static DATABASE: ::rumcake::storage::StorageService<'static, ::rumcake::hw::mcu::Flash> = ::rumcake::storage::StorageService::new();
                    unsafe { DATABASE.setup(flash, config_start, config_end, &mut READ_BUF, &mut OP_BUF).await; }
                })
            }
        }
        _ => None,
    }
}

pub(crate) fn keyboard_main(
    str: ItemStruct,
    kb_name: Ident,
    keyboard: KeyboardSettings,
) -> TokenStream {
    let mut initialization = TokenStream::new();
    let mut spawning = TokenStream::new();

    let uses_bluetooth = keyboard.bluetooth
        || keyboard
            .split_peripheral
            .as_ref()
            .is_some_and(|args| args.driver == "ble")
        || keyboard
            .split_central
            .as_ref()
            .is_some_and(|args| args.driver == "ble");

    // Setup microcontroller
    initialization.extend(quote! {
        ::rumcake::hw::mcu::initialize_rcc();
    });

    #[cfg(feature = "nrf")]
    {
        spawning.extend(quote! {
            spawner.spawn(::rumcake::adc_task!()).unwrap();
        });

        if uses_bluetooth {
            initialization.extend(quote! {
                let sd = ::rumcake::hw::mcu::setup_softdevice::<#kb_name>();
            });
            spawning.extend(quote! {
                let sd = &*sd;
                spawner.spawn(::rumcake::softdevice_task!(sd)).unwrap();
            });
        }
    }

    // Keyboard setup, and matrix polling task
    if !keyboard.no_matrix {
        initialization.extend(quote! {
            let (matrix, debouncer) = ::rumcake::keyboard::setup_keyboard_matrix(#kb_name);
        });
        spawning.extend(quote! {
            spawner
                .spawn(::rumcake::matrix_poll!(#kb_name, matrix, debouncer))
                .unwrap();
        });
    }

    // Flash setup
    if let Some(ref driver) = keyboard.storage {
        if !cfg!(feature = "storage") {
            initialization.extend(quote_spanned! {
                keyboard.storage.span() => compile_error!("Storage driver was specified, but rumcake's `storage` feature flag is not enabled. Please enable the feature.");
            });
        } else {
            match setup_storage_driver(driver.as_str(), uses_bluetooth) {
                Some(driver_setup) => {
                    initialization.extend(driver_setup);
                }
                None => {
                    initialization.extend(quote_spanned! {
                        driver.span() => compile_error!("Unknown storage driver.");
                    });
                }
            };
        }
    };

    if keyboard.bluetooth || keyboard.usb {
        spawning.extend(quote! {
            spawner.spawn(::rumcake::layout_collect!(#kb_name)).unwrap();
        });
    }

    spawning.extend(quote! {
        spawner.spawn(::rumcake::output_switcher!()).unwrap();
    });

    #[cfg(feature = "nrf")]
    if keyboard.bluetooth {
        initialization.extend(quote! {
            let hid_server = ::rumcake::bluetooth::nrf_ble::Server::new(sd).unwrap();
        });
        spawning.extend(quote! {
            spawner.spawn(::rumcake::nrf_ble_task!(#kb_name, sd, hid_server)).unwrap();
        });
    }

    // USB Configuration
    if keyboard.usb {
        initialization.extend(quote! {
            let mut builder = ::rumcake::hw::mcu::setup_usb_driver::<#kb_name>();

            // HID Class setup
            let kb_class = ::rumcake::usb::setup_usb_hid_nkro_writer(&mut builder);
        });
        spawning.extend(quote! {
            let usb = builder.build();

            // Task spawning
            // Initialize USB device
            spawner.spawn(::rumcake::start_usb!(usb)).unwrap();

            // HID Keyboard Report sending
            spawner.spawn(::rumcake::usb_hid_kb_write_task!(kb_class)).unwrap();
        });

        if cfg!(feature = "media-keycodes") {
            initialization.extend(quote! {
                // HID consumer
                let consumer_class = ::rumcake::usb::setup_usb_hid_consumer_writer(&mut builder);
            });
            spawning.extend(quote! {
                // HID Consumer Report sending
                spawner.spawn(::rumcake::usb_hid_consumer_write_task!(consumer_class)).unwrap();
            });
        }
    }

    if keyboard.usb && (keyboard.via.is_some() || keyboard.vial.is_some()) {
        initialization.extend(quote! {
            // Via HID setup
            let (via_reader, via_writer) =
                ::rumcake::usb::setup_usb_via_hid_reader_writer(&mut builder).split();
        });
        spawning.extend(quote! {
            // HID raw report (for VIA) reading and writing
            spawner
                .spawn(::rumcake::usb_hid_via_read_task!(via_reader))
                .unwrap();
        });
        spawning.extend(quote! {
            spawner.spawn(::rumcake::usb_hid_via_write_task!(via_writer)).unwrap();
        });
    }

    if keyboard.via.is_some() && keyboard.vial.is_some() {
        initialization.extend(quote_spanned! {
            str.span() => compile_error!("Via and Vial are both specified. Please only choose one.");
        });
    } else if let Some(args) = keyboard.via {
        let args = args.unwrap_or_default();

        if args.use_storage && keyboard.storage.is_none() {
            initialization.extend(quote_spanned! {
                args.use_storage.span() => compile_error!("Via uses storage but no `storage` driver was specified. Either specify a `storage` driver, or remove `use_storage` from your Via settings.");
            });
        } else if args.use_storage {
            spawning.extend(quote! {
                spawner
                    .spawn(::rumcake::via_storage_task!(#kb_name, &DATABASE))
                    .unwrap();
            });
        }

        spawning.extend(quote! {
            spawner
                .spawn(::rumcake::via_process_task!(#kb_name))
                .unwrap();
        });
    } else if let Some(args) = keyboard.vial {
        let args = args.unwrap_or_default();

        if args.use_storage && keyboard.storage.is_none() {
            initialization.extend(quote_spanned! {
                args.use_storage.span() => compile_error!("Vial uses storage but no `storage` driver was specified. Either specify a `storage` driver, or remove `use_storage` from your Vial settings.");
            });
        } else if args.use_storage {
            spawning.extend(quote! {
                spawner
                    .spawn(::rumcake::vial_storage_task!(#kb_name, &DATABASE))
                    .unwrap();
            });
        }

        spawning.extend(quote! {
            spawner
                .spawn(::rumcake::vial_process_task!(#kb_name))
                .unwrap();
        });
    }

    // Split keyboard setup
    if keyboard.split_peripheral.is_some() && keyboard.split_central.is_some() {
        initialization.extend(quote_spanned! {
            str.span() => compile_error!("A device can not be a central device and a peripheral at the same time. Please only choose one.");
        });
    } else if keyboard.split_peripheral.is_some() && keyboard.no_matrix {
        initialization.extend(quote_spanned! {
            str.span() => compile_error!("A split peripheral must have a matrix. Please remove `no_matrix` or `split_peripheral`.");
        });
    } else if let Some(ref args) = keyboard.split_peripheral {
        if args.driver.is_empty() {
            initialization.extend(quote_spanned! {
                args.driver.span() => compile_error!("You must specify a peripheral device driver.");
            })
        } else {
            match setup_split_driver(&kb_name, args.driver.as_str(), SplitRole::Peripheral) {
                Some((driver_setup, driver_spawns)) => {
                    initialization.extend(driver_setup);
                    spawning.extend(driver_spawns);
                    spawning.extend(quote! {
                    spawner.spawn(::rumcake::peripheral_task!(#kb_name, split_peripheral_driver)).unwrap();
                });
                }
                None => {
                    initialization.extend(quote_spanned! {
                    args.driver.span() => compile_error!("Unknown split peripheral device driver.");
                });
                }
            }
        }
    }

    if let Some(ref args) = keyboard.split_central {
        if args.driver.is_empty() {
            initialization.extend(quote_spanned! {
                args.driver.span() => compile_error!("You must specify a central device driver.");
            })
        } else {
            match setup_split_driver(&kb_name, args.driver.as_str(), SplitRole::Central) {
                Some((driver_setup, driver_spawns)) => {
                    initialization.extend(driver_setup);
                    spawning.extend(driver_spawns);
                    spawning.extend(quote! {
                    spawner.spawn(::rumcake::central_task!(#kb_name, split_central_driver)).unwrap();
                });
                }
                None => {
                    initialization.extend(quote_spanned! {
                    args.driver.span() => compile_error!("Unknown split central device driver.");
                });
                }
            }
        }
    }

    // Underglow setup
    if let Some(args) = keyboard.underglow {
        if args.driver.is_empty() {
            initialization.extend(quote_spanned! {
                args.driver.span() => compile_error!("You must specify an underglow driver.");
            })
        } else if args.use_storage && keyboard.storage.is_none() {
            initialization.extend(quote_spanned! {
                args.driver.span() => compile_error!("Underglow uses storage but no `storage` driver was specified. Either specify a `storage` driver, or remove `use_storage` from your underglow settings.");
            });
        } else {
            match setup_underglow_driver(&kb_name, args.driver.as_str()) {
                Some(driver_setup) => {
                    initialization.extend(driver_setup);

                    if args.use_storage {
                        spawning.extend(quote! {
                            spawner.spawn(::rumcake::underglow_storage_task!(#kb_name, &DATABASE)).unwrap();
                        });
                    }

                    spawning.extend(quote! {
                        spawner.spawn(::rumcake::underglow_task!(#kb_name, underglow_driver)).unwrap();
                    });
                }
                None => {
                    initialization.extend(quote_spanned! {
                        args.driver.span() => compile_error!("Unknown underglow driver.");
                    });
                }
            }
        }
    }

    // Backlight setup
    if let Some(args) = keyboard.simple_backlight {
        if args.driver.is_empty() {
            initialization.extend(quote_spanned! {
                args.driver.span() => compile_error!("You must specify a simple backlight driver.");
            })
        } else if args.use_storage && keyboard.storage.is_none() {
            initialization.extend(quote_spanned! {
                args.driver.span() => compile_error!("Simple backlighting uses storage but no `storage` driver was specified. Either specify a `storage` driver, or remove `use_storage` from your simple backlight settings.");
            });
        } else {
            match setup_backlight_driver(
                &kb_name,
                BacklightType::SimpleBacklight,
                args.driver.as_str(),
            ) {
                Some(driver_setup) => {
                    initialization.extend(driver_setup);

                    if args.use_storage {
                        spawning.extend(quote! {
                            spawner.spawn(::rumcake::simple_backlight_storage_task!(#kb_name, &DATABASE)).unwrap();
                        });
                    }

                    spawning.extend(quote! {
                        spawner.spawn(::rumcake::simple_backlight_task!(#kb_name, backlight_driver)).unwrap();
                    });
                }
                None => {
                    initialization.extend(quote_spanned! {
                        args.driver.span() => compile_error!("Unknown simple backlight driver.");
                    });
                }
            }
        }
    }

    if let Some(args) = keyboard.simple_backlight_matrix {
        if args.driver.is_empty() {
            initialization.extend(quote_spanned! {
                args.driver.span() => compile_error!("You must specify a simple backlight matrix driver.");
            })
        } else if args.use_storage && keyboard.storage.is_none() {
            initialization.extend(quote_spanned! {
                args.driver.span() => compile_error!("Simple backlight matrix uses storage but no `storage` driver was specified. Either specify a `storage` driver, or remove `use_storage` from your simple backlight matrix settings.");
            });
        } else {
            match setup_backlight_driver(
                &kb_name,
                BacklightType::SimpleBacklightMatrix,
                args.driver.as_str(),
            ) {
                Some(driver_setup) => {
                    initialization.extend(driver_setup);

                    if args.use_storage {
                        spawning.extend(quote! {
                            spawner.spawn(::rumcake::simple_backlight_matrix_storage_task!(#kb_name, &DATABASE)).unwrap();
                        });
                    }

                    spawning.extend(quote! {
                        spawner.spawn(::rumcake::simple_backlight_matrix_task!(#kb_name, backlight_driver)).unwrap();
                    });
                }
                None => {
                    initialization.extend(quote_spanned! {
                        args.driver.span() => compile_error!("Unknown simple backlight matrix driver.");
                    });
                }
            }
        }
    }

    if let Some(args) = keyboard.rgb_backlight_matrix {
        if args.driver.is_empty() {
            initialization.extend(quote_spanned! {
                args.driver.span() => compile_error!("You must specify an RGB backlight matrix driver.");
            })
        } else if args.use_storage && keyboard.storage.is_none() {
            initialization.extend(quote_spanned! {
                args.driver.span() => compile_error!("RGB backlight matrix uses storage but no `storage` driver was specified. Either specify a `storage` driver, or remove `use_storage` from your RGB backlight matrix settings.");
            });
        } else {
            match setup_backlight_driver(
                &kb_name,
                BacklightType::RGBBacklightMatrix,
                args.driver.as_str(),
            ) {
                Some(driver_setup) => {
                    initialization.extend(driver_setup);

                    if args.use_storage {
                        spawning.extend(quote! {
                            spawner.spawn(::rumcake::rgb_backlight_matrix_storage_task!(#kb_name, &DATABASE)).unwrap();
                        });
                    }

                    spawning.extend(quote! {
                        spawner.spawn(::rumcake::rgb_backlight_matrix_task!(#kb_name, backlight_driver)).unwrap();
                    });
                }
                None => {
                    initialization.extend(quote_spanned! {
                        args.driver.span() => compile_error!("Unknown RGB backlight matrix driver.");
                    });
                }
            }
        }
    }

    // Display setup
    if let Some(ref args) = keyboard.display {
        if args.driver.is_empty() {
            initialization.extend(quote_spanned! {
                args.driver.span() => compile_error!("You must specify a display driver.");
            })
        } else {
            match setup_display_driver(&kb_name, args.driver.as_str()) {
                Some(driver_setup) => {
                    initialization.extend(driver_setup);
                    spawning.extend(quote! {
                        spawner.spawn(::rumcake::display_task!(#kb_name, display_driver)).unwrap();
                    });
                }
                None => {
                    initialization.extend(quote_spanned! {
                        args.driver.span() => compile_error!("Unknown display driver.");
                    });
                }
            }
        }
    }

    quote! {
        #[::embassy_executor::main]
        async fn main(spawner: ::embassy_executor::Spawner) {
            #initialization
            #spawning
        }

        #str
    }
}

#[derive(Debug)]
/// This is the exact same as [`Option<T>`], but has a different [`syn::parse::Parse`] implementation,
/// where "No" parses to `None`, and anything else that parses as `T` corresponds `Some(T)`
pub(crate) enum OptionalItem<T> {
    None,
    Some(T),
}

impl<T: Parse> Parse for OptionalItem<T> {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        input.step(|cursor| {
            if let Some((tt, next)) = cursor.token_tree() {
                if tt.to_string() == "No" {
                    return Ok((OptionalItem::None, next));
                }

                return if let Ok(t) = T::parse.parse2(tt.into_token_stream()) {
                    Ok((OptionalItem::Some(t), next))
                } else {
                    Err(cursor.error("Invalid item."))
                };
            };

            Err(cursor.error("No item found."))
        })
    }
}

impl<T: ToTokens> ToTokens for OptionalItem<T> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            OptionalItem::None => quote! { None }.to_tokens(tokens),
            OptionalItem::Some(item) => quote! { Some(#item) }.to_tokens(tokens),
        }
    }
}

#[derive(Debug)]
pub struct MatrixDefinition<T> {
    pub row_brace: syn::token::Brace,
    pub rows: Vec<T>,
    pub col_brace: syn::token::Brace,
    pub cols: Vec<T>,
}

impl<T: Parse> syn::parse::Parse for MatrixDefinition<T> {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let row_content;
        let row_brace = braced!(row_content in input);
        let mut rows = Vec::new();
        while let Ok(t) = row_content.parse() {
            rows.push(t)
        }
        if !row_content.is_empty() {
            return Err(syn::Error::new(
                row_content.span(),
                "Encountered an invalid token.",
            ));
        }

        let col_content;
        let col_brace = braced!(col_content in input);
        let mut cols = Vec::new();
        while let Ok(t) = col_content.parse() {
            cols.push(t)
        }
        if !col_content.is_empty() {
            return Err(syn::Error::new(
                row_content.span(),
                "Encountered an invalid token.",
            ));
        }

        Ok(Self {
            row_brace,
            rows,
            col_brace,
            cols,
        })
    }
}

pub fn build_matrix(input: MatrixDefinition<Ident>) -> TokenStream {
    let MatrixDefinition { rows, cols, .. } = input;
    let row_count = rows.len();
    let col_count = cols.len();

    quote! {
        const MATRIX_ROWS: usize = #row_count;
        const MATRIX_COLS: usize = #col_count;

        fn build_matrix(
        ) -> Result<::rumcake::keyberon::matrix::Matrix<impl ::rumcake::embedded_hal::digital::v2::InputPin<Error = core::convert::Infallible>, impl ::rumcake::embedded_hal::digital::v2::OutputPin<Error = core::convert::Infallible>, { Self::MATRIX_COLS }, { Self::MATRIX_ROWS }>, core::convert::Infallible> {
            ::rumcake::keyberon::matrix::Matrix::new([
                #(
                    ::rumcake::hw::mcu::input_pin!(#cols)
                ),*
            ], [
                #(
                    ::rumcake::hw::mcu::output_pin!(#rows)
                ),*
            ])
        }
    }
}

#[derive(Debug)]
pub struct LayoutLike<T> {
    pub layers: Vec<Layer<T>>,
}

impl<T: Parse> syn::parse::Parse for LayoutLike<T> {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut layers = Vec::new();
        while let Ok(t) = input.parse() {
            layers.push(t)
        }
        if !input.is_empty() {
            return Err(syn::Error::new(
                input.span(),
                "Encountered tokens that don't look like a layer definition.",
            ));
        }

        Ok(Self { layers })
    }
}

#[derive(Debug)]
pub struct Layer<T> {
    pub layer_brace: syn::token::Brace,
    pub layer: MatrixLike<T>,
}

impl<T: Parse> syn::parse::Parse for Layer<T> {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        let layer_brace = braced!(content in input);

        Ok(Self {
            layer_brace,
            layer: content.parse()?,
        })
    }
}

#[derive(Debug)]
pub struct MatrixLike<T> {
    pub rows: Vec<MatrixRow<T>>,
}

impl<T: Parse> syn::parse::Parse for MatrixLike<T> {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut rows = Vec::new();
        while let Ok(t) = input.parse() {
            rows.push(t)
        }
        if !input.is_empty() {
            return Err(syn::Error::new(
                input.span(),
                "Encountered tokens that don't look like a row definition.",
            ));
        }

        Ok(Self { rows })
    }
}

#[derive(Debug)]
pub struct MatrixRow<T> {
    pub row_bracket: syn::token::Bracket,
    pub cols: Vec<T>,
}

impl<T: Parse> syn::parse::Parse for MatrixRow<T> {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        let row_bracket = bracketed!(content in input);
        let mut cols = Vec::new();
        while let Ok(t) = content.parse() {
            cols.push(t)
        }
        if !content.is_empty() {
            return Err(syn::Error::new(
                input.span(),
                "Encountered an invalid token.",
            ));
        }

        Ok(Self { row_bracket, cols })
    }
}

pub fn build_layout(raw: TokenStream, layers: LayoutLike<TokenTree>) -> TokenStream {
    let rows = &layers
        .layers
        .first()
        .expect_or_abort("Expected at least one layer to be defined")
        .layer
        .rows;

    let first_row = rows
        .first()
        .expect_or_abort("Expected at least one row to be defined");

    let layer_count = layers.layers.len();
    let row_count = rows.len();
    let col_count = first_row.cols.len();

    quote! {
        const LAYOUT_COLS: usize = #col_count;
        const LAYOUT_ROWS: usize = #row_count;
        const LAYERS: usize = #layer_count;

        fn get_original_layout() -> ::rumcake::keyberon::layout::Layers<{ Self::LAYOUT_COLS }, { Self::LAYOUT_ROWS }, { Self::LAYERS }, ::rumcake::keyboard::Keycode> {
            use ::rumcake::keyberon;
            const LAYERS: ::rumcake::keyberon::layout::Layers<#col_count, #row_count, #layer_count, ::rumcake::keyboard::Keycode> = ::rumcake::keyberon::layout::layout! { #raw };
            LAYERS
        }

        fn get_layout(
        ) -> &'static ::rumcake::keyboard::Layout<{ Self::LAYOUT_COLS }, { Self::LAYOUT_ROWS }, { Self::LAYERS }> {
            use ::rumcake::keyberon;
            static KEYBOARD_LAYOUT: ::rumcake::keyboard::Layout<#col_count, #row_count, #layer_count> = ::rumcake::keyboard::Layout::new();
            static mut LAYERS: ::rumcake::keyberon::layout::Layers<#col_count, #row_count, #layer_count, ::rumcake::keyboard::Keycode> = ::rumcake::keyberon::layout::layout! { #raw };
            KEYBOARD_LAYOUT.init(unsafe { &mut LAYERS });
            &KEYBOARD_LAYOUT
        }
    }
}

pub struct RemapMacroInput {
    pub original_matrix_brace: syn::token::Brace,
    pub original_matrix: MatrixLike<OptionalItem<Ident>>,
    pub remapped_matrix_brace: syn::token::Brace,
    pub remapped_matrix: MatrixLike<Ident>,
}

impl Parse for RemapMacroInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let original_matrix_content;
        let original_matrix_brace = braced!(original_matrix_content in input);
        let remapped_matrix_content;
        let remapped_matrix_brace = braced!(remapped_matrix_content in input);
        Ok(RemapMacroInput {
            original_matrix_brace,
            original_matrix: original_matrix_content.parse()?,
            remapped_matrix_brace,
            remapped_matrix: remapped_matrix_content.parse()?,
        })
    }
}

pub fn remap_matrix(input: RemapMacroInput) -> TokenStream {
    let old = input.original_matrix.rows.iter().map(|row| {
        let items = row.cols.iter().map(|col| match col {
            OptionalItem::None => quote! { No },
            OptionalItem::Some(ident) => quote! { $#ident },
        });

        quote! { [ #(#items)* ] }
    });
    let old2 = old.clone();

    let new = input.remapped_matrix.rows.iter().map(|row| {
        let items = row.cols.iter().map(|col| quote! { $#col:tt });
        quote! { [ #(#items)* ] }
    });
    let new2 = new.clone();

    quote! {
        macro_rules! remap {
            ($macro:ident! { $({ #(#new)* })* }) => {
                $macro! {
                    $(
                        {
                            #(#old)*
                        }
                    )*
                }
            };
            ($macro:ident! { #(#new2)* }) => {
                $macro! {
                    #(#old2)*
                }
            };
        }
    }
}
