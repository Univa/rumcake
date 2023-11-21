//! Support for Via's protocol (version 12).
//!
//! To use Via, you will need to implement [`ViaKeyboard`]. If you would like to save your Via
//! changes, you will also need to enable the `storage` feature flag, and setup the appropriate
//! storage buffers using [`crate::setup_via_storage_buffers`].

use crate::keyboard::{Keyboard, KeyboardLayout};
use defmt::{assert, debug, error, Debug2Format};
use embassy_futures::join;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_usb::class::hid::{
    Config, HidReader, HidReaderWriter, HidWriter, ReportId, RequestHandler, State,
};
use embassy_usb::control::OutResponse;
use embassy_usb::driver::Driver;
use embassy_usb::Builder;
use static_cell::StaticCell;

pub(crate) mod handlers;
pub(crate) mod protocol_12;

pub(crate) use protocol_12 as protocol;

/// A trait that keyboards must implement to use the Via protocol.
pub trait ViaKeyboard: Keyboard + KeyboardLayout {
    const VIA_ENABLED: bool = true;

    /// Version of your firmware.
    const VIA_FIRMWARE_VERSION: u32 = 1; // This is the default if not set in QMK.

    /// How many bytes are needed to represent the number of possible layout options for your
    /// keyboard.
    const VIA_EEPROM_LAYOUT_OPTIONS_SIZE: usize = 1; // This is the default if not set in QMK

    /// The default layout option to use for your keyboard in the Via app.
    const VIA_EEPROM_LAYOUT_OPTIONS_DEFAULT: u32 = 0x00000000; // This is the default if not set in QMK

    /// The number of layers that you can modify in the Via app. This number must be equal to or
    /// less than the number of layers in your keyberon layout, [`KeyboardLayout::LAYERS`].
    const DYNAMIC_KEYMAP_LAYER_COUNT: usize = Self::LAYERS;
    // const DYNAMIC_KEYMAP_LAYER_COUNT: usize = 4; // This is the default if this variable isn't defined in QMK

    /// The number of macros that your keyboard can store.
    const DYNAMIC_KEYMAP_MACRO_COUNT: u8 = 0; // this is the default if this variable isn't defined in QMK, TODO: Change when macros are implemented

    /// The total amount of space allocated to macros in your storage peripheral.
    const DYNAMIC_KEYMAP_MACRO_EEPROM_SIZE: usize = 512;
    // const DYNAMIC_KEYMAP_MACRO_EEPROM_SIZE: usize = (Self::DYNAMIC_KEYMAP_EEPROM_MAX_ADDR
    //     - Self::DYNAMIC_KEYMAP_MACRO_EEPROM_ADDR
    //     + 1) as usize; // This is the default if not defined in QMK.

    #[cfg(feature = "storage")]
    fn get_layout_options_storage_state(
    ) -> &'static mut crate::storage::StorageServiceState<1, { Self::VIA_EEPROM_LAYOUT_OPTIONS_SIZE }>;

    #[cfg(feature = "storage")]
    fn get_dynamic_keymap_storage_state() -> &'static mut crate::storage::StorageServiceState<
        3,
        { Self::DYNAMIC_KEYMAP_LAYER_COUNT * Self::LAYOUT_COLS * Self::LAYOUT_ROWS * 2 },
    >;

    #[cfg(feature = "storage")]
    fn get_dynamic_keymap_encoder_storage_state(
    ) -> &'static mut crate::storage::StorageServiceState<
        2,
        { Self::DYNAMIC_KEYMAP_LAYER_COUNT * Self::NUM_ENCODERS * 2 * 2 },
    >;

    #[cfg(feature = "storage")]
    fn get_dynamic_keymap_macro_storage_state() -> &'static mut crate::storage::StorageServiceState<
        3,
        { Self::DYNAMIC_KEYMAP_MACRO_EEPROM_SIZE },
    >;

    /// Override for handling a Via/Vial protocol packet.
    ///
    /// Returning `true` indicates that a command is fully handled, so the Via/Vial task will not
    /// continue to process the data. Returning `false` will let the Via task continue to process
    /// the data, using the usual protocol. You should not send the data to the host using this
    /// method. Responses to the host are automatically handled by the Via/Vial task.
    fn handle_via_command(data: &mut [u8]) -> bool {
        false
    }

    /// Optional handler that allows you to handle changes to the current layout options setting.
    fn handle_set_layout_options(updated_layout: u32) {}

    /// Optional handler that you can use to handle custom UI channel commands. See
    /// <https://www.caniusevia.com/docs/custom_ui>. **This is currently only applicable to Via,
    /// not Vial.**
    ///
    /// This is called if the Via protocol is unable to handle a custom channel command. The
    /// current Via protocol implementation handles lighting (`rgblight`/`underglow`,
    /// `backlight`/`simple-backlight`, `led_matrix`/`simple-backlight-matrix`,
    /// `rgb_matrix`/`rgb-backlight-matrix`) channels.
    fn handle_custom_value_command(data: &mut [u8], _len: u8) {
        data[0] = protocol::ViaCommandId::Unhandled as u8;
    }
}

/// Report descriptor used for Via. Pulled from QMK.
const VIA_REPORT_DESCRIPTOR: &[u8] = &[
    0x06, 0x60, 0xFF, // Usage Page (Vendor Defined)
    0x09, 0x61, // Usage (Vendor Defined)
    0xA1, 0x01, // Collection (Application)
    // Data to host
    0x09, 0x62, //   Usage (Vendor Defined)
    0x15, 0x00, //   Logical Minimum (0)
    0x26, 0xFF, 0x00, //   Logical Maximum (255)
    0x95, 0x20, //   Report Count
    0x75, 0x08, //   Report Size (8)
    0x81, 0x02, //   Input (Data, Variable, Absolute)
    // Data from host
    0x09, 0x63, //   Usage (Vendor Defined)
    0x15, 0x00, //   Logical Minimum (0)
    0x26, 0xFF, 0x00, //   Logical Maximum (255)
    0x95, 0x20, //   Report Count
    0x75, 0x08, //   Report Size (8)
    0x91, 0x02, //   Output (Data, Variable, Absolute)
    0xC0, // End Collection
];

struct ViaCommandHandler;

static VIA_COMMAND_HANDLER: ViaCommandHandler = ViaCommandHandler;

impl RequestHandler for ViaCommandHandler {
    fn get_report(&self, _id: ReportId, _buf: &mut [u8]) -> Option<usize> {
        None
    }

    fn set_report(&self, _id: ReportId, buf: &[u8]) -> OutResponse {
        let mut data: [u8; 32] = [0; 32];
        data.copy_from_slice(buf);

        if let Err(err) = VIA_REPORT_HID_SEND_CHANNEL.try_send(data) {
            error!(
                "[VIA] Could not queue the Via command to be processed: {:?}",
                err
            );
        };

        OutResponse::Accepted
    }

    fn get_idle_ms(&self, _id: Option<ReportId>) -> Option<u32> {
        None
    }

    fn set_idle_ms(&self, _id: Option<ReportId>, _duration_ms: u32) {}
}

/// Configure the HID report reader and writer for Via/Vial packets.
///
/// The reader should be passed to [`usb_hid_via_read_task`], and the writer should be passed to
/// [`usb_hid_via_write_task`].
pub fn setup_usb_via_hid_reader_writer(
    builder: &mut Builder<'static, impl Driver<'static>>,
) -> HidReaderWriter<'static, impl Driver<'static>, 32, 32> {
    static VIA_STATE: StaticCell<State> = StaticCell::new();
    let via_state = VIA_STATE.init(State::new());
    let via_hid_config = Config {
        request_handler: Some(&VIA_COMMAND_HANDLER),
        report_descriptor: VIA_REPORT_DESCRIPTOR,
        poll_ms: 1,
        max_packet_size: 32,
    };
    HidReaderWriter::<_, 32, 32>::new(builder, via_state, via_hid_config)
}

/// Channel used to send reports from the Via app. Reports that are sent to this channel will be
/// processed by the Via task, and call the appropriate command handler, depending on the report
/// contents.
pub static VIA_REPORT_HID_SEND_CHANNEL: Channel<ThreadModeRawMutex, [u8; 32], 1> = Channel::new();

#[rumcake_macros::task]
pub async fn usb_hid_via_read_task(hid: HidReader<'static, impl Driver<'static>, 32>) {
    hid.run(false, &VIA_COMMAND_HANDLER).await;
}

#[rumcake_macros::task]
pub async fn usb_hid_via_write_task<K: ViaKeyboard + 'static>(
    _k: K,
    mut hid: HidWriter<'static, impl Driver<'static>, 32>,
) where
    [(); (K::LAYOUT_COLS + u8::BITS as usize - 1) / u8::BITS as usize * K::LAYOUT_ROWS]:,
    [(); K::LAYERS]:,
    [(); K::LAYOUT_ROWS]:,
    [(); K::LAYOUT_COLS]:,
{
    assert!(K::DYNAMIC_KEYMAP_LAYER_COUNT <= K::LAYERS);

    let via_state: Mutex<ThreadModeRawMutex, protocol::ViaState<K>> =
        Mutex::new(Default::default());

    let report_fut = async {
        loop {
            let mut report = VIA_REPORT_HID_SEND_CHANNEL.receive().await;

            if K::VIA_ENABLED {
                {
                    let mut via_state = via_state.lock().await;
                    protocol::process_via_command::<K>(&mut report, &mut via_state).await;
                }

                debug!("[VIA] Writing HID raw report {:?}", Debug2Format(&report));
                if let Err(err) = hid.write(&report).await {
                    error!(
                        "[VIA] Couldn't write HID raw report: {:?}",
                        Debug2Format(&err)
                    );
                };
            }
        }
    };

    join::join(report_fut, protocol::background_task::<K>(&via_state)).await;
}

#[cfg(feature = "storage")]
pub mod storage {
    use defmt::warn;
    use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
    use embassy_sync::channel::Channel;
    use embassy_sync::signal::Signal;
    use embedded_storage_async::nor_flash::NorFlash;

    use crate::storage::StorageKey;

    use super::ViaKeyboard;

    #[macro_export]
    macro_rules! setup_via_storage_buffers {
        ($k:ident) => {
            fn get_layout_options_storage_state(
            ) -> &'static mut $crate::storage::StorageServiceState<
                1,
                { $k::VIA_EEPROM_LAYOUT_OPTIONS_SIZE },
            > {
                static mut LAYOUT_OPTIONS_STORAGE_STATE: $crate::storage::StorageServiceState<
                    { 1 },
                    { $k::VIA_EEPROM_LAYOUT_OPTIONS_SIZE },
                > = $crate::storage::StorageServiceState::new();
                unsafe { &mut LAYOUT_OPTIONS_STORAGE_STATE }
            }

            fn get_dynamic_keymap_storage_state(
            ) -> &'static mut $crate::storage::StorageServiceState<
                3,
                { $k::DYNAMIC_KEYMAP_LAYER_COUNT * $k::LAYOUT_COLS * $k::LAYOUT_ROWS * 2 },
            > {
                static mut DYNAMIC_KEYMAP_STORAGE_STATE: $crate::storage::StorageServiceState<
                    { 3 },
                    { $k::DYNAMIC_KEYMAP_LAYER_COUNT * $k::LAYOUT_COLS * $k::LAYOUT_ROWS * 2 },
                > = $crate::storage::StorageServiceState::new();
                unsafe { &mut DYNAMIC_KEYMAP_STORAGE_STATE }
            }

            fn get_dynamic_keymap_encoder_storage_state(
            ) -> &'static mut $crate::storage::StorageServiceState<
                2,
                { $k::DYNAMIC_KEYMAP_LAYER_COUNT * $k::NUM_ENCODERS * 2 * 2 },
            > {
                static mut DYNAMIC_KEYMAP_ENCODER_STORAGE_STATE:
                    $crate::storage::StorageServiceState<
                        { 2 },
                        { $k::DYNAMIC_KEYMAP_LAYER_COUNT * $k::NUM_ENCODERS * 2 * 2 },
                    > = $crate::storage::StorageServiceState::new();
                unsafe { &mut DYNAMIC_KEYMAP_ENCODER_STORAGE_STATE }
            }

            fn get_dynamic_keymap_macro_storage_state(
            ) -> &'static mut $crate::storage::StorageServiceState<
                3,
                { $k::DYNAMIC_KEYMAP_MACRO_EEPROM_SIZE },
            > {
                static mut DYNAMIC_KEYMAP_MACRO_STORAGE_STATE:
                    $crate::storage::StorageServiceState<
                        { 3 },
                        { $k::DYNAMIC_KEYMAP_MACRO_EEPROM_SIZE },
                    > = $crate::storage::StorageServiceState::new();
                unsafe { &mut DYNAMIC_KEYMAP_MACRO_STORAGE_STATE }
            }
        };
    }

    pub(super) enum ViaStorageKeys {
        LayoutOptions,
        DynamicKeymap,
        DynamicKeymapMacro,
        DynamicKeymapEncoder,
    }

    impl From<ViaStorageKeys> for StorageKey {
        fn from(value: ViaStorageKeys) -> Self {
            match value {
                ViaStorageKeys::LayoutOptions => StorageKey::LayoutOptions,
                ViaStorageKeys::DynamicKeymap => StorageKey::DynamicKeymap,
                ViaStorageKeys::DynamicKeymapMacro => StorageKey::DynamicKeymapMacro,
                ViaStorageKeys::DynamicKeymapEncoder => StorageKey::DynamicKeymapEncoder,
            }
        }
    }

    #[repr(u8)]
    enum Operation {
        Write([u8; 32], ViaStorageKeys, usize, usize),
        Delete,
    }

    /// A function that dispatches a flash operation to the Via storage task. This will obtain a
    /// lock, and hold onto it until the storage task signals a completion. `offset` corresponds to
    /// the first byte of the stored data for the given `key` that we want to update. For example,
    /// if [0x23, 0x65, 0xEB] is stored in flash for the key `LayoutOptions`, and we want to update
    /// the last 2 bytes, we would pass in an offset of 1, and a `data` slice with a length of 2.
    pub(super) async fn update_data(key: ViaStorageKeys, offset: usize, data: &[u8]) {
        // TODO: this function will wait eternally if via_storage_task is not there
        // Buffer size of 32 is based off of the VIA packet size. This can actually be less, because
        // some bytes are used for the command IDs, but 32 should be fine.
        let mut buf = [0; 32];
        let len = data.len();
        buf[..len].copy_from_slice(data);
        OPERATION_CHANNEL
            .send(Operation::Write(buf, key, offset, len))
            .await;
        OPERATION_COMPLETE.wait().await;
    }

    pub(super) async fn reset_data() {
        OPERATION_CHANNEL.send(Operation::Delete).await;
        OPERATION_COMPLETE.wait().await;
    }

    static OPERATION_CHANNEL: Channel<ThreadModeRawMutex, Operation, 1> = Channel::new();
    static OPERATION_COMPLETE: Signal<ThreadModeRawMutex, ()> = Signal::new();

    pub(super) static VIA_LAYOUT_OPTIONS: Signal<ThreadModeRawMutex, u32> = Signal::new();

    #[rumcake_macros::task]
    pub async fn via_storage_task<K: ViaKeyboard + 'static, F: NorFlash>(
        _k: K,
        database: &'static crate::storage::Database<'static, F>,
    ) where
        [(); K::VIA_EEPROM_LAYOUT_OPTIONS_SIZE]:,
        [(); K::DYNAMIC_KEYMAP_LAYER_COUNT * K::LAYOUT_COLS * K::LAYOUT_ROWS * 2]:,
        [(); K::DYNAMIC_KEYMAP_LAYER_COUNT * K::NUM_ENCODERS * 2 * 2]:,
        [(); K::DYNAMIC_KEYMAP_MACRO_EEPROM_SIZE]:,
        [(); F::ERASE_SIZE]:,
        [(); K::LAYERS]:,
        [(); K::LAYOUT_ROWS]:,
        [(); K::LAYOUT_COLS]:,
    {
        // Initialize VIA data
        {
            let mut database = database.lock().await;

            // Initialize layout options
            let options_metadata = [K::VIA_EEPROM_LAYOUT_OPTIONS_SIZE as u8];
            let _ = database
                .initialize(
                    K::get_layout_options_storage_state(),
                    crate::storage::StorageKey::LayoutOptions,
                    &options_metadata,
                )
                .await;
            if let Ok((stored_data, stored_len)) = database
                .read_raw(
                    K::get_layout_options_storage_state(),
                    crate::storage::StorageKey::LayoutOptions,
                )
                .await
            {
                let mut bytes = [0; 4];
                bytes[(4 - stored_len)..].copy_from_slice(&stored_data[..stored_len]);
                VIA_LAYOUT_OPTIONS.signal(u32::from_be_bytes(bytes))
            };

            // Initialize layout
            let layout_metadata = [
                K::DYNAMIC_KEYMAP_LAYER_COUNT as u8,
                K::LAYOUT_COLS as u8,
                K::LAYOUT_ROWS as u8,
            ];
            let _ = database
                .initialize(
                    K::get_dynamic_keymap_storage_state(),
                    crate::storage::StorageKey::DynamicKeymap,
                    &layout_metadata,
                )
                .await;
            if let Ok((stored_data, stored_len)) = database
                .read_raw(
                    K::get_dynamic_keymap_storage_state(),
                    crate::storage::StorageKey::DynamicKeymap,
                )
                .await
            {
                // Load layout from flash
                let mut layout = K::get_layout().lock().await;
                for byte in (0..stored_len).step_by(2) {
                    if let Some(action) = super::protocol::keycodes::convert_keycode_to_action(
                        u16::from_be_bytes(stored_data[byte..byte + 2].try_into().unwrap()),
                    ) {
                        let layer = byte / (K::LAYOUT_ROWS * K::LAYOUT_COLS * 2);
                        let row = (byte / (K::LAYOUT_COLS * 2)) % K::LAYOUT_ROWS;
                        let col = (byte / 2) % K::LAYOUT_COLS;

                        layout
                            .change_action((row as u8, col as u8), layer, action)
                            .unwrap();
                    }
                }
            } else {
                // Save default layout to flash
                let mut layout = K::get_layout().lock().await;
                let mut buf =
                    [0; K::DYNAMIC_KEYMAP_LAYER_COUNT * K::LAYOUT_COLS * K::LAYOUT_ROWS * 2];
                for byte in (0..buf.len()).step_by(2) {
                    let layer = byte / (K::LAYOUT_ROWS * K::LAYOUT_COLS * 2);
                    let row = (byte / (K::LAYOUT_COLS * 2)) % K::LAYOUT_ROWS;
                    let col = (byte / 2) % K::LAYOUT_COLS;

                    buf[(byte)..(byte + 2)].copy_from_slice(
                        &super::protocol::keycodes::convert_action_to_keycode(
                            layout.get_action((row as u8, col as u8), layer).unwrap(),
                        )
                        .to_be_bytes(),
                    );
                }
                let _ = database
                    .write_raw(
                        K::get_dynamic_keymap_storage_state(),
                        StorageKey::DynamicKeymap,
                        &buf,
                    )
                    .await;
            };

            // Initialize encoder layout
            let encoder_metadata = [K::DYNAMIC_KEYMAP_LAYER_COUNT as u8, K::NUM_ENCODERS as u8];
            let _ = database
                .initialize(
                    K::get_dynamic_keymap_encoder_storage_state(),
                    crate::storage::StorageKey::DynamicKeymapEncoder,
                    &encoder_metadata,
                )
                .await;

            // Initialize macros
            let _ = database
                .initialize(
                    K::get_dynamic_keymap_macro_storage_state(),
                    crate::storage::StorageKey::DynamicKeymapMacro,
                    &layout_metadata,
                )
                .await;
        }

        loop {
            match OPERATION_CHANNEL.receive().await {
                Operation::Write(data, key, offset, len) => {
                    let mut database = database.lock().await;

                    match key {
                        ViaStorageKeys::LayoutOptions => {
                            // Update data
                            // For layout options, we just overwrite all of the old data
                            if let Err(()) = database
                                .write_raw(
                                    K::get_layout_options_storage_state(),
                                    key.into(),
                                    &data[..len],
                                )
                                .await
                            {
                                warn!("[VIA] Could not write layout options.")
                            };
                        }
                        ViaStorageKeys::DynamicKeymap => {
                            let key = key.into();
                            let mut buf = [0; K::DYNAMIC_KEYMAP_LAYER_COUNT
                                * K::LAYOUT_COLS
                                * K::LAYOUT_ROWS
                                * 2];

                            // Read data
                            match database
                                .read_raw(K::get_dynamic_keymap_storage_state(), key)
                                .await
                            {
                                Ok((stored_data, stored_len)) => {
                                    buf[..stored_len].copy_from_slice(stored_data);
                                }
                                Err(()) => {
                                    warn!("[VIA] Could not read dynamic keymap buffer.");
                                }
                            };

                            // Update data
                            buf[offset..(offset + len)].copy_from_slice(&data[..len]);

                            if let Err(()) = database
                                .write_raw(K::get_dynamic_keymap_storage_state(), key, &buf)
                                .await
                            {
                                warn!("[VIA] Could not write dynamic keymap buffer.",)
                            };
                        }
                        ViaStorageKeys::DynamicKeymapMacro => {
                            let key = key.into();
                            let mut buf = [0; K::DYNAMIC_KEYMAP_MACRO_EEPROM_SIZE];

                            // Read data
                            let stored_len = match database
                                .read_raw(K::get_dynamic_keymap_macro_storage_state(), key)
                                .await
                            {
                                Ok((stored_data, stored_len)) => {
                                    buf[..stored_len].copy_from_slice(stored_data);
                                    stored_len
                                }
                                Err(()) => {
                                    warn!("[VIA] Could not read dynamic keymap macro buffer.");
                                    0 // Assume that there is no data yet
                                }
                            };

                            // Update data
                            buf[offset..(offset + len)].copy_from_slice(&data[..len]);

                            let new_length = stored_len.max(offset + len);

                            if let Err(()) = database
                                .write_raw(
                                    K::get_dynamic_keymap_macro_storage_state(),
                                    key,
                                    &buf[..new_length],
                                )
                                .await
                            {
                                warn!("[VIA] Could not write dynamic keymap macro buffer.")
                            };
                        }
                        ViaStorageKeys::DynamicKeymapEncoder => {
                            let key = key.into();
                            let mut buf =
                                [0; K::DYNAMIC_KEYMAP_LAYER_COUNT * K::NUM_ENCODERS * 2 * 2];

                            // Read data
                            match database
                                .read_raw(K::get_dynamic_keymap_encoder_storage_state(), key)
                                .await
                            {
                                Ok((stored_data, stored_len)) => {
                                    buf[..stored_len].copy_from_slice(stored_data);
                                }
                                Err(()) => {
                                    warn!("[VIA] Could not read dynamic keymap encoder.");
                                }
                            };

                            // Update data
                            buf[offset..(offset + len)].copy_from_slice(&data[..len]);

                            if let Err(()) = database
                                .write_raw(K::get_dynamic_keymap_encoder_storage_state(), key, &buf)
                                .await
                            {
                                warn!("[VIA] Could not write dynamic keymap encoder.")
                            };
                        }
                    }
                }
                Operation::Delete => {
                    let mut database = database.lock().await;
                    let _ = database.delete(StorageKey::LayoutOptions).await;
                    let _ = database.delete(StorageKey::DynamicKeymap).await;
                    let _ = database.delete(StorageKey::DynamicKeymapMacro).await;
                    let _ = database.delete(StorageKey::DynamicKeymapEncoder).await;
                }
            }

            OPERATION_COMPLETE.signal(())
        }
    }
}
