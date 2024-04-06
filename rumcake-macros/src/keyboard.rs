use darling::util::{Override, SpannedValue};
use darling::FromMeta;
use proc_macro2::{Ident, TokenStream, TokenTree};
use proc_macro_error::{abort, emit_error, OptionExt};
use quote::quote;
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::{
    braced, custom_keyword, Expr, ExprRange, ItemStruct, LitInt, LitStr, PathSegment, Token,
};

use crate::common::{Layer, LayoutLike, MatrixLike, OptionalItem, Row};
use crate::TuplePair;

#[derive(Debug, FromMeta, Default)]
#[darling(default)]
pub(crate) struct KeyboardSettings {
    no_matrix: bool,
    bluetooth: bool,
    usb: bool,
    encoders: bool,
    storage: Option<StorageSettings>,
    simple_backlight: Option<LightingSettings>,
    simple_backlight_matrix: Option<LightingSettings>,
    rgb_backlight_matrix: Option<LightingSettings>,
    underglow: Option<LightingSettings>,
    display: Option<DisplaySettings>,
    split_peripheral: Option<SplitPeripheralSettings>,
    split_central: Option<SplitCentralSettings>,
    via: Option<ViaSettings>,
    vial: Option<ViaSettings>,
    bootloader_double_tap_reset: Option<Override<LitInt>>,
}

#[derive(Debug, FromMeta)]
pub(crate) struct LightingSettings {
    id: Ident,
    driver_setup_fn: Ident,
    use_storage: Option<SpannedValue<bool>>,
}

#[derive(Debug, FromMeta)]
pub(crate) struct DisplaySettings {
    driver_setup_fn: Ident,
}

#[derive(Debug, FromMeta)]
pub(crate) struct SplitCentralSettings {
    driver_type: Option<LitStr>,
    driver_setup_fn: Ident,
}

#[derive(Debug, FromMeta)]
pub(crate) struct SplitPeripheralSettings {
    driver_type: Option<LitStr>,
    driver_setup_fn: Ident,
}

#[derive(Debug, FromMeta)]
pub(crate) struct ViaSettings {
    id: Ident,
    use_storage: Option<SpannedValue<bool>>,
}

#[derive(Debug, FromMeta)]
pub(crate) struct StorageSettings {
    driver: LitStr,
    buffer_size: Option<LitInt>,
    flash_size: Option<LitInt>,
    dma: Option<Ident>,
}

fn setup_storage_driver(
    initialization: &mut TokenStream,
    outer: &mut TokenStream,
    kb_name: &Ident,
    config: &StorageSettings,
    uses_bluetooth: bool,
) -> bool {
    let buffer_size = if let Some(lit) = &config.buffer_size {
        lit.base10_parse::<usize>().unwrap_or_else(|_| {
            abort!(
                lit,
                "The provided buffer size could not be parsed as a usize value."
            )
        })
    } else {
        1024
    };

    match config.driver.value().as_str() {
        "internal" => {
            if cfg!(feature = "nrf") && uses_bluetooth {
                // TODO: Fix storage on nrf-ble targets
                outer.extend(quote! {
                    use ::rumcake::storage::FlashStorage;
                    static DATABASE: ::rumcake::storage::StorageService<'static, ::rumcake::hw::platform::nrf_softdevice::Flash, #kb_name> = ::rumcake::storage::StorageService::new();
                    impl ::rumcake::storage::StorageDevice for #kb_name {
                        type FlashStorageType = ::rumcake::hw::platform::nrf_softdevice::Flash;

                        fn get_storage_buffer() -> &'static mut [u8] {
                            static mut STORAGE_BUFFER: [u8; #buffer_size] = [0; #buffer_size];
                            unsafe { &mut STORAGE_BUFFER }
                        }

                        fn get_storage_service(
                        ) -> &'static rumcake::storage::StorageService<'static, Self::FlashStorageType, Self>
                        where
                            [(); Self::FlashStorageType::ERASE_SIZE]:,
                            Self: Sized,
                        {
                            &DATABASE
                        }
                    }
                });
                initialization.extend(quote! {
                    use ::rumcake::storage::FlashStorage;
                    let flash = ::rumcake::hw::platform::setup_internal_softdevice_flash(sd);
                    let config_start = unsafe { &::rumcake::hw::__config_start as *const u32 as usize };
                    let config_end = unsafe { &::rumcake::hw::__config_end as *const u32 as usize };
                    static mut READ_BUF: [u8; ::rumcake::hw::platform::nrf_softdevice::Flash::ERASE_SIZE] = [0; ::rumcake::hw::platform::nrf_softdevice::Flash::ERASE_SIZE];
                    static mut OP_BUF: [u8; ::rumcake::hw::platform::nrf_softdevice::Flash::ERASE_SIZE] = [0; ::rumcake::hw::platform::nrf_softdevice::Flash::ERASE_SIZE];
                    unsafe { DATABASE.setup(flash, config_start, config_end, &mut READ_BUF, &mut OP_BUF).await; }
                });

                return false;
            }

            if cfg!(any(feature = "stm32", feature = "nrf")) {
                outer.extend(quote! {
                    use ::rumcake::storage::FlashStorage;
                    static DATABASE: ::rumcake::storage::StorageService<'static, ::rumcake::hw::platform::Flash, #kb_name> = ::rumcake::storage::StorageService::new();
                    impl ::rumcake::storage::StorageDevice for #kb_name {
                        type FlashStorageType = ::rumcake::hw::platform::Flash;

                        fn get_storage_buffer() -> &'static mut [u8] {
                            static mut STORAGE_BUFFER: [u8; #buffer_size] = [0; #buffer_size];
                            unsafe { &mut STORAGE_BUFFER }
                        }

                        fn get_storage_service(
                        ) -> &'static rumcake::storage::StorageService<'static, Self::FlashStorageType, Self>
                        where
                            [(); Self::FlashStorageType::ERASE_SIZE]:,
                            Self: Sized,
                        {
                            &DATABASE
                        }
                    }
                });
                initialization.extend(quote! {
                    use ::rumcake::storage::FlashStorage;
                    let flash = ::rumcake::hw::platform::setup_internal_flash();
                    let config_start = unsafe { &::rumcake::hw::__config_start as *const u32 as usize };
                    let config_end = unsafe { &::rumcake::hw::__config_end as *const u32 as usize };
                    static mut READ_BUF: [u8; ::rumcake::hw::platform::Flash::ERASE_SIZE] = [0; ::rumcake::hw::platform::Flash::ERASE_SIZE];
                    static mut OP_BUF: [u8; ::rumcake::hw::platform::Flash::ERASE_SIZE] = [0; ::rumcake::hw::platform::Flash::ERASE_SIZE];
                    unsafe { DATABASE.setup(flash, config_start, config_end, &mut READ_BUF, &mut OP_BUF).await; }
                });

                return false;
            }

            if cfg!(feature = "rp") {
                if config.flash_size.is_none() {
                    emit_error!(
                        config.driver,
                        "You must specify a non-zero size for your flash chip."
                    );

                    return true;
                }

                if config.dma.is_none() {
                    emit_error!(config.driver, "You must specify a `dma` channel.");

                    return true;
                }

                let lit = config.flash_size.as_ref().unwrap();
                let dma = config.dma.as_ref().unwrap();

                let size = lit.base10_parse::<usize>().unwrap_or_else(|_| {
                    abort!(
                        lit,
                        "The provided flash size could not be parsed as a usize value."
                    );
                });

                if size == 0 {
                    emit_error!(
                        config.driver,
                        "You must specify a non-zero size for your flash chip."
                    );
                    return true;
                }

                outer.extend(quote! {
                    use ::rumcake::storage::FlashStorage;
                    static DATABASE: ::rumcake::storage::StorageService<'static, ::rumcake::hw::platform::Flash<#size>, #kb_name> = ::rumcake::storage::StorageService::new();
                    impl ::rumcake::storage::StorageDevice for #kb_name {
                        type FlashStorageType = ::rumcake::hw::platform::Flash<'static, #size>;

                        fn get_storage_buffer() -> &'static mut [u8] {
                            static mut STORAGE_BUFFER: [u8; #buffer_size] = [0; #buffer_size];
                            unsafe { &mut STORAGE_BUFFER }
                        }

                        fn get_storage_service(
                        ) -> &'static rumcake::storage::StorageService<'static, Self::FlashStorageType, Self>
                        where
                            [(); Self::FlashStorageType::ERASE_SIZE]:,
                            Self: Sized,
                        {
                            &DATABASE
                        }
                    }
                });
                initialization.extend(quote! {
                    let flash = ::rumcake::hw::platform::setup_internal_flash::<#size>(unsafe { ::rumcake::hw::platform::embassy_rp::peripherals::#dma::steal() });
                    let config_start = unsafe { &::rumcake::hw::__config_start as *const u32 as usize };
                    let config_end = unsafe { &::rumcake::hw::__config_end as *const u32 as usize };
                    static mut READ_BUF: [u8; ::rumcake::hw::platform::embassy_rp::flash::ERASE_SIZE] = [0; ::rumcake::hw::platform::embassy_rp::flash::ERASE_SIZE];
                    static mut OP_BUF: [u8; ::rumcake::hw::platform::embassy_rp::flash::ERASE_SIZE] = [0; ::rumcake::hw::platform::embassy_rp::flash::ERASE_SIZE];
                    unsafe { DATABASE.setup(flash, config_start, config_end, &mut READ_BUF, &mut OP_BUF).await; }
                });

                return false;
            }

            emit_error!(
                config.driver,
                "Internal storage driver is not available for your platform."
            );

            return true;
        }
        _ => (),
    };

    emit_error!(config.driver, "Unknown storage driver.");

    true
}

pub(crate) fn keyboard_main(
    str: ItemStruct,
    kb_name: Ident,
    keyboard: KeyboardSettings,
) -> TokenStream {
    let mut initialization = TokenStream::new();
    let mut spawning = TokenStream::new();
    let mut outer = TokenStream::new();
    let mut error = false;

    let uses_bluetooth = keyboard.bluetooth
        || keyboard.split_peripheral.as_ref().is_some_and(|args| {
            args.driver_type
                .as_ref()
                .map_or(false, |d| d.value() == "nrf-ble")
        })
        || keyboard.split_central.as_ref().is_some_and(|args| {
            args.driver_type
                .as_ref()
                .map_or(false, |d| d.value() == "nrf-ble")
        });

    // Setup microcontroller
    initialization.extend(quote! {
        ::rumcake::hw::platform::initialize_rcc();
    });

    if cfg!(feature = "nrf") {
        spawning.extend(quote! {
            let sampler = setup_adc_sampler();
            spawner.spawn(::rumcake::adc_task!(sampler)).unwrap();
        });

        if uses_bluetooth {
            initialization.extend(quote! {
                let sd = ::rumcake::hw::platform::setup_softdevice::<#kb_name>();
            });
            spawning.extend(quote! {
                let sd = &*sd;
                spawner.spawn(::rumcake::softdevice_task!(sd)).unwrap();
            });
        }
    }

    // Keyboard setup, and matrix polling task
    if !keyboard.no_matrix {
        spawning.extend(quote! {
            spawner
                .spawn(::rumcake::matrix_poll!(#kb_name))
                .unwrap();
        });
    }

    if keyboard.encoders {
        spawning.extend(quote! {
            spawner
                .spawn(::rumcake::ec11_encoders_poll!(#kb_name))
                .unwrap();
        })
    }

    // Flash setup
    if let Some(ref driver) = keyboard.storage {
        if !cfg!(feature = "storage") {
            emit_error!(driver.driver, "Storage driver was specified, but rumcake's `storage` feature flag is not enabled. Please enable the feature.");
            error = true;
        } else {
            error = setup_storage_driver(
                &mut initialization,
                &mut outer,
                &kb_name,
                driver,
                uses_bluetooth,
            );
        }
    };

    if keyboard.bluetooth || keyboard.usb {
        outer.extend(quote! {
            impl ::rumcake::hw::HIDDevice for #kb_name {}
        });
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
            let mut builder = ::rumcake::hw::platform::setup_usb_driver::<#kb_name>();

            // HID Class setup
            let kb_class = ::rumcake::usb::setup_usb_hid_nkro_writer(&mut builder);
        });
        spawning.extend(quote! {
            let usb = builder.build();

            // Task spawning
            // Initialize USB device
            spawner.spawn(::rumcake::start_usb!(usb)).unwrap();

            // HID Keyboard Report sending
            spawner.spawn(::rumcake::usb_hid_kb_write_task!(#kb_name, kb_class)).unwrap();
        });

        if cfg!(feature = "media-keycodes") {
            initialization.extend(quote! {
                // HID consumer
                let consumer_class = ::rumcake::usb::setup_usb_hid_consumer_writer(&mut builder);
            });
            spawning.extend(quote! {
                // HID Consumer Report sending
                spawner.spawn(::rumcake::usb_hid_consumer_write_task!(#kb_name, consumer_class)).unwrap();
            });
        }
    }

    if keyboard.usb && (keyboard.via.is_some() || keyboard.vial.is_some()) {
        initialization.extend(quote! {
            static VIA_COMMAND_HANDLER: ::rumcake::usb::ViaCommandHandler<#kb_name> = ::rumcake::usb::ViaCommandHandler::new();
            // Via HID setup
            let (via_reader, via_writer) =
                ::rumcake::usb::setup_usb_via_hid_reader_writer(&VIA_COMMAND_HANDLER, &mut builder).split();
        });
        spawning.extend(quote! {
            // HID raw report (for VIA) reading and writing
            spawner
                .spawn(::rumcake::usb_hid_via_read_task!(&VIA_COMMAND_HANDLER, via_reader))
                .unwrap();
        });
        spawning.extend(quote! {
            spawner.spawn(::rumcake::usb_hid_via_write_task!(#kb_name, via_writer)).unwrap();
        });
    }

    if keyboard.via.is_some() && keyboard.vial.is_some() {
        emit_error!(
            str,
            "Via and Vial are both specified. Please only choose one."
        );
        error = true;
    } else if let Some(args) = keyboard.via {
        if args.use_storage.map_or(false, |b| *b) && keyboard.storage.is_none() {
            emit_error!(args.use_storage.unwrap().span(), "Via uses storage but no `storage` driver was specified. Either specify a `storage` driver, or remove `use_storage` from your Via settings.");
            error = true;
        } else if args.use_storage.map_or(false, |b| *b) {
            spawning.extend(quote! {
                ::rumcake::via::initialize_via_data(#kb_name).await;
            });
        }

        spawning.extend(quote! {
            spawner
                .spawn(::rumcake::via_process_task!(#kb_name))
                .unwrap();
        });
    } else if let Some(args) = keyboard.vial {
        if args.use_storage.map_or(false, |b| *b) && keyboard.storage.is_none() {
            emit_error!(args.use_storage.unwrap().span(), "Vial uses storage but no `storage` driver was specified. Either specify a `storage` driver, or remove `use_storage` from your Vial settings.");
            error = true;
        } else if args.use_storage.map_or(false, |b| *b) {
            spawning.extend(quote! {
                ::rumcake::vial::initialize_vial_data(#kb_name).await;
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
        emit_error!(str, "A device can not be a central device and a peripheral at the same time. Please only choose one.");
        error = true;
    } else if keyboard.split_peripheral.is_some() && keyboard.no_matrix {
        emit_error!(str, "A split peripheral must have a matrix. Please remove `no_matrix` or `split_peripheral`.");
        error = true;
    } else if let Some(args) = keyboard.split_peripheral {
        let setup_fn = args.driver_setup_fn;
        let driver_type = args
            .driver_type
            .as_ref()
            .map_or(String::from("standard"), |v| v.value());
        match driver_type.as_str() {
            "standard" => {
                initialization.extend(quote! {
                    let split_peripheral_driver = #setup_fn().await;
                });
            }
            "nrf-ble" => {
                initialization.extend(quote! {
                    let peripheral_server = ::rumcake::drivers::nrf_ble::peripheral::PeripheralDeviceServer::new(sd).unwrap();
                    let (split_peripheral_driver, central_address) = #setup_fn().await;
                });
                spawning.extend(quote! {
                    spawner.spawn(::rumcake::nrf_ble_peripheral_task!(central_address, sd, peripheral_server)).unwrap();
                });
            }
            _ => {
                emit_error!(args.driver_type, "Unknown split peripheral driver type.");
                error = true;
            }
        }
        spawning.extend(quote! {
            spawner.spawn(::rumcake::peripheral_task!(#kb_name, split_peripheral_driver)).unwrap();
        });
    }

    if let Some(args) = keyboard.split_central {
        let setup_fn = args.driver_setup_fn;
        let driver_type = args
            .driver_type
            .as_ref()
            .map_or(String::from("standard"), |v| v.value());
        match driver_type.as_str() {
            "standard" => {
                initialization.extend(quote! {
                    let split_central_driver = #setup_fn().await;
                });
            }
            "nrf-ble" => {
                initialization.extend(quote! {
                    let (split_central_driver, peripheral_addresses) = #setup_fn().await;
                });
                spawning.extend(quote! {
                    spawner.spawn(::rumcake::nrf_ble_central_task!(peripheral_addresses, sd)).unwrap();
                });
            }
            _ => {
                emit_error!(args.driver_type, "Unknown split peripheral driver type.");
                error = true;
            }
        }
        spawning.extend(quote! {
            spawner.spawn(::rumcake::central_task!(#kb_name, split_central_driver)).unwrap();
        });
    }

    // Underglow setup
    if let Some(args) = keyboard.underglow {
        if args.use_storage.map_or(false, |b| *b) && keyboard.storage.is_none() {
            emit_error!(args.use_storage.unwrap().span(), "Underglow uses storage but no `storage` driver was specified. Either specify a `storage` driver, or remove `use_storage` from your underglow settings.");
            error = true;
        } else {
            let setup_fn = args.driver_setup_fn;
            initialization.extend(quote! {
                let underglow_driver = #setup_fn().await;
            });
            let id = args.id;
            initialization.extend(quote! {
                let underglow_animator = ::rumcake::lighting::underglow::UnderglowAnimator::<#id, _>::new(Default::default(), underglow_driver);
            });

            if args.use_storage.map_or(false, |b| *b) {
                initialization.extend(quote! {
                    let underglow_animator_storage = underglow_animator.create_storage_instance();
                });
                spawning.extend(quote! {
                    ::rumcake::lighting::initialize_lighting_data(&underglow_animator_storage, &DATABASE).await;
                    spawner.spawn(::rumcake::lighting_storage_task!(underglow_animator_storage, &DATABASE)).unwrap();
                });
            }
            spawning.extend(quote! {
                spawner.spawn(::rumcake::lighting_task!(underglow_animator, None)).unwrap();
            });
        }
    }

    // Backlight setup
    if let Some(args) = keyboard.simple_backlight {
        if args.use_storage.map_or(false, |b| *b) && keyboard.storage.is_none() {
            emit_error!(args.use_storage.unwrap().span(), "Simple backlighting uses storage but no `storage` driver was specified. Either specify a `storage` driver, or remove `use_storage` from your simple backlight settings.");
            error = true;
        } else {
            let setup_fn = args.driver_setup_fn;
            initialization.extend(quote! {
                let simple_backlight_driver = #setup_fn().await;
            });
            let id = args.id;
            initialization.extend(quote! {
                let simple_backlight_animator = ::rumcake::lighting::simple_backlight::SimpleBacklightAnimator::<#id, _>::new(Default::default(), simple_backlight_driver);
            });
            if args.use_storage.map_or(false, |b| *b) {
                initialization.extend(quote! {
                    let simple_backlight_animator_storage = simple_backlight_animator.create_storage_instance();
                });
                spawning.extend(quote! {
                    ::rumcake::lighting::initialize_lighting_data(&simple_backlight_animator_storage, &DATABASE).await;
                    spawner.spawn(::rumcake::lighting_storage_task!(simple_backlight_animator_storage, &DATABASE)).unwrap();
                });
            }
            spawning.extend(quote! {
                spawner.spawn(::rumcake::lighting_task!(simple_backlight_animator, None)).unwrap();
            });
        }
    }

    if let Some(args) = keyboard.simple_backlight_matrix {
        if args.use_storage.map_or(false, |b| *b) && keyboard.storage.is_none() {
            emit_error!(args.use_storage.unwrap().span(), "Simple backlight matrix uses storage but no `storage` driver was specified. Either specify a `storage` driver, or remove `use_storage` from your simple backlight matrix settings.");
            error = true;
        } else {
            let setup_fn = args.driver_setup_fn;
            initialization.extend(quote! {
                let simple_backlight_matrix_driver = #setup_fn().await;
            });
            let id = args.id;
            initialization.extend(quote! {
                let simple_backlight_matrix_animator = ::rumcake::lighting::simple_backlight_matrix::SimpleBacklightMatrixAnimator::<#id, _>::new(Default::default(), simple_backlight_matrix_driver);
            });
            if args.use_storage.map_or(false, |b| *b) {
                initialization.extend(quote! {
                    let simple_backlight_matrix_animator_storage = simple_backlight_matrix_animator.create_storage_instance();
                });
                spawning.extend(quote! {
                    ::rumcake::lighting::initialize_lighting_data(&simple_backlight_matrix_animator_storage, &DATABASE).await;
                    spawner.spawn(::rumcake::lighting_storage_task!(simple_backlight_matrix_animator_storage, &DATABASE)).unwrap();
                });
            }
            spawning.extend(quote! {
                spawner.spawn(::rumcake::lighting_task!(simple_backlight_matrix_animator, None)).unwrap();
            });
        }
    }

    if let Some(args) = keyboard.rgb_backlight_matrix {
        if args.use_storage.map_or(false, |b| *b) && keyboard.storage.is_none() {
            emit_error!(args.use_storage.unwrap().span(), "RGB backlight matrix uses storage but no `storage` driver was specified. Either specify a `storage` driver, or remove `use_storage` from your RGB backlight matrix settings.");
            error = true;
        } else {
            let setup_fn = args.driver_setup_fn;
            initialization.extend(quote! {
                let rgb_backlight_matrix_driver = #setup_fn().await;
            });
            let id = args.id;
            initialization.extend(quote! {
                let rgb_backlight_matrix_animator = ::rumcake::lighting::rgb_backlight_matrix::RGBBacklightMatrixAnimator::<#id, _>::new(Default::default(), rgb_backlight_matrix_driver);
            });
            if args.use_storage.map_or(false, |b| *b) {
                initialization.extend(quote! {
                    let rgb_backlight_matrix_animator_storage = rgb_backlight_matrix_animator.create_storage_instance();
                });
                spawning.extend(quote! {
                    ::rumcake::lighting::initialize_lighting_data(&rgb_backlight_matrix_animator_storage, &DATABASE).await;
                    spawner.spawn(::rumcake::lighting_storage_task!(rgb_backlight_matrix_animator_storage, &DATABASE)).unwrap();
                });
            }
            spawning.extend(quote! {
                spawner.spawn(::rumcake::lighting_task!(rgb_backlight_matrix_animator, None)).unwrap();
            });
        }
    }

    // Display setup
    if let Some(args) = keyboard.display {
        let setup_fn = args.driver_setup_fn;
        initialization.extend(quote! {
            let display_driver = #setup_fn().await;
        });
        spawning.extend(quote! {
            spawner.spawn(::rumcake::display_task!(#kb_name, display_driver)).unwrap();
        });
    }

    if let Some(arg) = keyboard.bootloader_double_tap_reset {
        let timeout: u64 = match arg {
            Override::Inherit => 200,
            Override::Explicit(lit) => {
                let value = lit.base10_parse::<u64>().unwrap_or_else(|_| {
                    abort!(
                        lit,
                        "The provided timeout value could not be parsed as a u64 value."
                    );
                });

                if value == 0 {
                    emit_error!(
                        lit,
                        "The timeout for double tapping the reset button should be > 0"
                    );
                    error = true;
                }

                value
            }
        };

        spawning.extend(quote! {
            unsafe {
                ::rumcake::hw::check_double_tap_bootloader(#timeout).await;
            }
        });
    }

    if error {
        quote! {
            #str
        }
    } else {
        quote! {
            #[::embassy_executor::main]
            async fn main(spawner: ::embassy_executor::Spawner) {
                #initialization
                #spawning
            }

            #outer

            #str
        }
    }
}

crate::parse_as_custom_fields! {
    pub struct StandardMatrixDefinitionBuilder for StandardMatrixDefinition {
        pub rows: Row<Ident>,
        pub cols: Row<Ident>,
    }
}

pub fn build_standard_matrix(input: StandardMatrixDefinition) -> TokenStream {
    let StandardMatrixDefinition { rows, cols } = input;
    let row_count = rows.items.len();
    let col_count = cols.items.len();

    let rows = rows.items.iter();
    let cols = cols.items.iter();

    let hal_name: PathSegment = syn::parse_str(crate::hw::HAL_CRATE).unwrap();

    quote! {
        const MATRIX_ROWS: usize = #row_count;
        const MATRIX_COLS: usize = #col_count;

        fn get_matrix() -> &'static ::rumcake::keyboard::PollableMatrix<impl ::rumcake::keyboard::Pollable> {
            static MATRIX: ::rumcake::once_cell::sync::OnceCell<
                ::rumcake::keyboard::PollableMatrix<
                    ::rumcake::keyboard::PollableStandardMatrix<
                        ::rumcake::hw::platform::#hal_name::gpio::Input<'static>,
                        ::rumcake::hw::platform::#hal_name::gpio::Output<'static>,
                        #col_count,
                        #row_count
                    >
                >
            > = ::rumcake::once_cell::sync::OnceCell::new();
            MATRIX.get_or_init(|| {
                ::rumcake::keyboard::PollableMatrix::new(
                    ::rumcake::keyboard::setup_standard_keyboard_matrix(
                        [
                            #(
                                ::rumcake::hw::platform::input_pin!(#cols)
                            ),*
                        ],
                        [
                            #(
                                ::rumcake::hw::platform::output_pin!(#rows)
                            ),*
                        ],
                        Self::DEBOUNCE_MS
                    ).unwrap()
                )
            })
        }
    }
}

pub fn build_direct_pin_matrix(input: MatrixLike<OptionalItem<Ident>>) -> TokenStream {
    let values = input.rows.iter().map(|row| {
        let items = row.items.iter().map(|item| match item {
            OptionalItem::None => quote! { None },
            OptionalItem::Some(pin_ident) => {
                quote! { Some(::rumcake::hw::platform::input_pin!(#pin_ident)) }
            }
        });
        quote! { #(#items),* }
    });

    let row_count = input.rows.len();
    let col_count = input
        .rows
        .first()
        .expect_or_abort("At least one row is required.")
        .items
        .len();

    let hal_name: PathSegment = syn::parse_str(crate::hw::HAL_CRATE).unwrap();

    quote! {
        const MATRIX_ROWS: usize = #row_count;
        const MATRIX_COLS: usize = #col_count;

        fn get_matrix() -> &'static ::rumcake::keyboard::PollableMatrix<impl ::rumcake::keyboard::Pollable> {
            static MATRIX: ::rumcake::once_cell::sync::OnceCell<
                ::rumcake::keyboard::PollableMatrix<
                    ::rumcake::keyboard::PollableDirectPinMatrix<
                        ::rumcake::hw::platform::#hal_name::gpio::Input<'static>,
                        #col_count,
                        #row_count
                    >
                >
            > = ::rumcake::once_cell::sync::OnceCell::new();
            MATRIX.get_or_init(|| {
                ::rumcake::keyboard::PollableMatrix::new(
                    ::rumcake::keyboard::setup_direct_pin_keyboard_matrix(
                        [
                            #([ #values ]),*
                        ],
                        Self::DEBOUNCE_MS
                    ).unwrap()
                )
            })
        }
    }
}

crate::parse_as_custom_fields! {
    pub struct AnalogMatrixDefinitionBuilder for AnalogMatrixDefinition {
        pub channels: Layer<OptionalItem<TuplePair>>,
        pub ranges: Layer<OptionalItem<ExprRange>>,
    }
}

pub fn build_analog_matrix(input: AnalogMatrixDefinition) -> TokenStream {
    let pos_to_ch = input.channels.layer.rows.iter().map(|row| {
        let items = row.items.iter().map(|item| match item {
            OptionalItem::None => quote! { (0, 0) },
            OptionalItem::Some(tuple) => quote! { #tuple },
        });
        quote! { #(#items),* }
    });

    let ranges = input.ranges.layer.rows.iter().map(|row| {
        let items = row.items.iter().map(|item| match item {
            OptionalItem::None => quote! { 0..0 },
            OptionalItem::Some(range) => quote! { #range },
        });
        quote! { #(#items),* }
    });

    let row_count = pos_to_ch.len();
    let col_count = input
        .channels
        .layer
        .rows
        .first()
        .expect_or_abort("At least one row must be specified")
        .items
        .len();

    quote! {
        const MATRIX_ROWS: usize = #row_count;
        const MATRIX_COLS: usize = #col_count;

        fn get_matrix() -> &'static ::rumcake::keyboard::PollableMatrix<impl ::rumcake::keyboard::Pollable> {
            static MATRIX: ::rumcake::once_cell::sync::OnceCell<
                ::rumcake::keyboard::PollableMatrix<
                    ::rumcake::keyboard::PollableAnalogMatrix<
                        AdcSamplerType,
                        #col_count,
                        #row_count
                    >
                >
            > = ::rumcake::once_cell::sync::OnceCell::new();
            MATRIX.get_or_init(|| {
                ::rumcake::keyboard::PollableMatrix::new(
                    ::rumcake::keyboard::setup_analog_keyboard_matrix(
                        setup_adc_sampler(),
                        [
                            #([ #pos_to_ch ]),*
                        ],
                        [
                            #([ #ranges ]),*
                        ],
                    )
                )
            })
        }
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
    let col_count = first_row.items.len();

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
            static KEYBOARD_LAYOUT: ::rumcake::once_cell::sync::OnceCell<
                ::rumcake::keyboard::Layout<#col_count, #row_count, #layer_count>,
            > = ::rumcake::once_cell::sync::OnceCell::new();
            const LAYERS: ::rumcake::keyberon::layout::Layers<#col_count, #row_count, #layer_count, ::rumcake::keyboard::Keycode> = ::rumcake::keyberon::layout::layout! { #raw };
            KEYBOARD_LAYOUT.get_or_init(|| {
                static mut LAYERS: ::rumcake::keyberon::layout::Layers<
                    #col_count,
                    #row_count,
                    #layer_count,
                    ::rumcake::keyboard::Keycode,
                > = ::rumcake::keyberon::layout::layout! { #raw };
                ::rumcake::keyboard::Layout::new(::rumcake::keyberon::layout::Layout::new(
                    unsafe { &mut LAYERS }
                ))
            })
        }
    }
}

crate::parse_as_custom_fields! {
    pub struct SetupEncoderArgsBuilder for SetupEncoderArgs {
        sw_pin: Expr,
        output_a_pin: Expr,
        output_b_pin: Expr,
        sw_pos: TuplePair,
        cw_pos: TuplePair,
        ccw_pos: TuplePair,
    }
}

custom_keyword!(Encoder);

pub struct EncoderDefinition {
    encoder_keyword: Encoder,
    brace_token: syn::token::Brace,
    encoder_args: SetupEncoderArgs,
}

impl Parse for EncoderDefinition {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            encoder_keyword: input.parse()?,
            brace_token: braced!(content in input),
            encoder_args: content.parse()?,
        })
    }
}

pub fn setup_encoders(encoders: Punctuated<EncoderDefinition, Token![,]>) -> TokenStream {
    let count = encoders.len();

    let (positions, definitions): (Vec<TokenStream>, Vec<TokenStream>) = encoders
        .iter()
        .map(|EncoderDefinition { encoder_args, .. }| {
            let SetupEncoderArgs {
                sw_pin,
                output_a_pin,
                output_b_pin,
                sw_pos,
                cw_pos,
                ccw_pos,
            } = encoder_args;

            (
                quote! {
                    [#sw_pos, #cw_pos, #ccw_pos]
                },
                quote! {
                    ::rumcake::keyboard::EC11Encoder::new(#sw_pin, #output_a_pin, #output_b_pin)
                },
            )
        })
        .unzip();

    quote! {
        const ENCODER_COUNT: usize = #count;

        fn get_encoders() -> [impl ::rumcake::keyboard::Encoder; Self::ENCODER_COUNT] {
            [#(#definitions),*]
        }

        fn get_layout_mappings() -> [[(u8, u8); 3]; Self::ENCODER_COUNT] {
            [#(#positions),*]
        }
    }
}

crate::parse_as_custom_fields! {
    pub struct RemapMacroInputBuilder for RemapMacroInput {
        pub original: Layer<OptionalItem<Ident>>,
        pub remapped: Layer<Ident>,
    }
}

pub fn remap_matrix(input: RemapMacroInput) -> TokenStream {
    let old = input.original.layer.rows.iter().map(|row| {
        let items = row.items.iter().map(|col| match col {
            OptionalItem::None => quote! { No },
            OptionalItem::Some(ident) => quote! { $#ident },
        });

        quote! { [ #(#items)* ] }
    });
    let old2 = old.clone();
    let old3 = old.clone();

    let new = input.remapped.layer.rows.iter().map(|row| {
        let items = row.items.iter().map(|col| quote! { $#col:tt });
        quote! { [ #(#items)* ] }
    });
    let new2 = new.clone();
    let new3 = new.clone();

    quote! {
        macro_rules! remap {
            ($macro:ident! { $({ #(#new2)* })* }) => {
                $macro! {
                    $(
                        {
                            #(#old2)*
                        }
                    )*
                }
            };
            ($macro:ident! { #(#new3)* }) => {
                $macro! {
                    #(#old3)*
                }
            };
            ($macro:ident! { [ $field_name:ident: { #(#new)* } $(, $($rest:tt)*)? ] -> [$($processed:tt)*] }) => {
                remap! { $macro! {
                        [
                            $(
                                $($rest)*
                            )?
                        ] -> [
                            $($processed)*
                            $field_name: { #(#old)* },
                        ]
                    }
                }
            };
            ($macro:ident! { [ $field_name:ident: $($value:tt)* $(, $($rest:tt)*)? ] -> [$($processed:tt)*] }) => {
                remap! { $macro! {
                        [
                            $(
                                $($rest)*
                            )?
                        ] -> [
                            $($processed)*
                            $field_name: $($value)*,
                        ]
                    }
                }
            };
            ($macro:ident! { [] -> [$($processed:tt)*] }) => {
                $macro! {
                    $($processed)*
                }
            };
            ($macro:ident! { $($all:tt)* }) => {
                remap! { $macro! {
                        [
                            $(
                                $all
                            )*
                        ] -> []
                    }
                }
            };
        }
    }
}
