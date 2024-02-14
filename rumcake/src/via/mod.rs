//! Support for Via's protocol (version 12).
//!
//! To use Via, you will need to implement [`ViaKeyboard`]. If you would like to save your Via
//! changes, you will also need to enable the `storage` feature flag, and setup the appropriate
//! storage buffers using [`crate::setup_via_storage_buffers`].

use crate::keyboard::{Keyboard, KeyboardLayout};
use defmt::assert;
use embassy_futures::join;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;

pub(crate) mod handlers;
pub(crate) mod protocol_12;

pub(crate) use protocol_12 as protocol;

pub use rumcake_macros::setup_macro_buffer;

/// Data structure that contains data for macros created by Via. Requires the size of the buffer,
/// and the number of sequences that can be created to be specified.
#[derive(Debug)]
pub struct MacroBuffer<'a, const N: usize, const S: usize> {
    buffer: [u8; N],
    sequences: [&'a [u8]; S],
}

impl<'a, const N: usize, const S: usize> MacroBuffer<'a, N, S> {
    pub const fn new() -> Self {
        Self {
            buffer: [0; N],
            sequences: [&[]; S],
        }
    }

    pub fn update_buffer(&'a mut self, offset: usize, data: &[u8]) {
        self.buffer[offset..(offset + data.len())].copy_from_slice(data);

        // update existing actions
        let mut chunks = self.buffer.splitn(S + 1, |byte| *byte == 0);
        for (i, action) in self.sequences.iter_mut().enumerate() {
            if let Some(chunk) = chunks.nth(i) {
                *action = chunk
            }
        }
    }
}

/// The different types of backlighting that can be used with Via. See
/// [`ViaKeyboard::BACKLIGHT_TYPE`].
pub enum BacklightType {
    SimpleBacklight,
    SimpleBacklightMatrix,
    RGBBacklightMatrix,
}

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

    /// The number of macros that your keyboard can store. You should use [`setup_macro_buffer`] to
    /// implement this.
    const DYNAMIC_KEYMAP_MACRO_COUNT: u8 = 0; // This is the default if this variable isn't defined in QMK

    /// The total amount of bytes that can be used to store macros assigned by Via.
    const DYNAMIC_KEYMAP_MACRO_BUFFER_SIZE: u16;

    /// Determines how QK_BACKLIGHT keycodes should be converted to a [`crate::keyboard::Keycode`]
    /// and vice versa. If this is `None`, then backlighting keycodes will not be converted.
    const BACKLIGHT_TYPE: Option<BacklightType> = None;

    /// Obtain a reference to macro data created by Via. You should use [`setup_macro_buffer`] to
    /// implement this.
    fn get_macro_buffer() -> &'static mut MacroBuffer<
        'static,
        { Self::DYNAMIC_KEYMAP_MACRO_BUFFER_SIZE as usize },
        { Self::DYNAMIC_KEYMAP_MACRO_COUNT as usize },
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
pub(crate) const VIA_REPORT_DESCRIPTOR: &[u8] = &[
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

/// Channel used to receive reports from the Via app to be processed. Reports that are sent to this
/// channel will be processed by the Via task, and call the appropriate command handler, depending
/// on the report contents.
pub static VIA_REPORT_HID_RECEIVE_CHANNEL: Channel<ThreadModeRawMutex, [u8; 32], 1> =
    Channel::new();

/// Channel used to send Via reports back to the Via host.
pub static VIA_REPORT_HID_SEND_CHANNEL: Channel<ThreadModeRawMutex, [u8; 32], 1> = Channel::new();

#[rumcake_macros::task]
pub async fn via_process_task<K: ViaKeyboard + 'static>(_k: K)
where
    [(); (K::LAYOUT_COLS + u8::BITS as usize - 1) / u8::BITS as usize * K::LAYOUT_ROWS]:,
    [(); K::LAYERS]:,
    [(); K::LAYOUT_ROWS]:,
    [(); K::LAYOUT_COLS]:,
    [(); K::DYNAMIC_KEYMAP_MACRO_BUFFER_SIZE as usize]:,
    [(); K::DYNAMIC_KEYMAP_MACRO_COUNT as usize]:,
{
    assert!(K::DYNAMIC_KEYMAP_LAYER_COUNT <= K::LAYERS);
    assert!(K::DYNAMIC_KEYMAP_LAYER_COUNT <= 16);

    let via_state: Mutex<ThreadModeRawMutex, protocol::ViaState<K>> =
        Mutex::new(Default::default());

    let report_fut = async {
        loop {
            let mut report = VIA_REPORT_HID_RECEIVE_CHANNEL.receive().await;

            if K::VIA_ENABLED {
                {
                    let mut via_state = via_state.lock().await;
                    protocol::process_via_command::<K>(&mut report, &mut via_state).await;
                }

                VIA_REPORT_HID_SEND_CHANNEL.send(report).await;
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

    use crate::storage::{StorageDevice, StorageKey};

    use super::ViaKeyboard;

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
    pub async fn via_storage_task<K: StorageDevice + ViaKeyboard + 'static, F: NorFlash>(
        _k: K,
        database: &crate::storage::StorageService<'_, F>,
    ) where
        [(); K::VIA_EEPROM_LAYOUT_OPTIONS_SIZE]:,
        [(); K::DYNAMIC_KEYMAP_LAYER_COUNT * K::LAYOUT_COLS * K::LAYOUT_ROWS * 2]:,
        [(); K::DYNAMIC_KEYMAP_LAYER_COUNT * K::NUM_ENCODERS * 2 * 2]:,
        [(); K::DYNAMIC_KEYMAP_MACRO_BUFFER_SIZE as usize]:,
        [(); K::DYNAMIC_KEYMAP_MACRO_COUNT as usize]:,
        [(); F::ERASE_SIZE]:,
        [(); K::LAYERS]:,
        [(); K::LAYOUT_ROWS]:,
        [(); K::LAYOUT_COLS]:,
    {
        // Initialize VIA data
        {
            // Initialize layout options
            let options_metadata = [K::VIA_EEPROM_LAYOUT_OPTIONS_SIZE as u8];
            let _ = database
                .check_metadata(
                    K::get_storage_buffer(),
                    crate::storage::StorageKey::LayoutOptions,
                    &options_metadata,
                )
                .await;
            if let Ok((stored_data, stored_len)) = database
                .read_raw(
                    K::get_storage_buffer(),
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
                .check_metadata(
                    K::get_storage_buffer(),
                    crate::storage::StorageKey::DynamicKeymap,
                    &layout_metadata,
                )
                .await;
            if let Ok((stored_data, stored_len)) = database
                .read_raw(
                    K::get_storage_buffer(),
                    crate::storage::StorageKey::DynamicKeymap,
                )
                .await
            {
                // Load layout from flash
                let mut layout = K::get_layout().lock().await;
                for byte in (0..stored_len).step_by(2) {
                    if let Some(action) = super::protocol::keycodes::convert_keycode_to_action::<K>(
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
                        &super::protocol::keycodes::convert_action_to_keycode::<K>(
                            layout.get_action((row as u8, col as u8), layer).unwrap(),
                        )
                        .to_be_bytes(),
                    );
                }
                let _ = database
                    .write_raw(K::get_storage_buffer(), StorageKey::DynamicKeymap, &buf)
                    .await;
            };

            // Initialize encoder layout
            let encoder_metadata = [K::DYNAMIC_KEYMAP_LAYER_COUNT as u8, K::NUM_ENCODERS as u8];
            let _ = database
                .check_metadata(
                    K::get_storage_buffer(),
                    crate::storage::StorageKey::DynamicKeymapEncoder,
                    &encoder_metadata,
                )
                .await;

            // Initialize macros
            let _ = database
                .check_metadata(
                    K::get_storage_buffer(),
                    crate::storage::StorageKey::DynamicKeymapMacro,
                    &layout_metadata,
                )
                .await;
            if let Ok((stored_data, stored_len)) = database
                .read_raw(
                    K::get_storage_buffer(),
                    crate::storage::StorageKey::DynamicKeymapMacro,
                )
                .await
            {
                K::get_macro_buffer().update_buffer(0, &stored_data[..stored_len])
            };
        }

        loop {
            match OPERATION_CHANNEL.receive().await {
                Operation::Write(data, key, offset, len) => {
                    match key {
                        ViaStorageKeys::LayoutOptions => {
                            // Update data
                            // For layout options, we just overwrite all of the old data
                            if let Err(()) = database
                                .write_raw(K::get_storage_buffer(), key.into(), &data[..len])
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
                            match database.read_raw(K::get_storage_buffer(), key).await {
                                Ok((stored_data, stored_len)) => {
                                    buf[..stored_len].copy_from_slice(stored_data);
                                }
                                Err(()) => {
                                    warn!("[VIA] Could not read dynamic keymap buffer.");
                                }
                            };

                            // Update data
                            buf[offset..(offset + len)].copy_from_slice(&data[..len]);

                            if let Err(()) =
                                database.write_raw(K::get_storage_buffer(), key, &buf).await
                            {
                                warn!("[VIA] Could not write dynamic keymap buffer.",)
                            };
                        }
                        ViaStorageKeys::DynamicKeymapMacro => {
                            let key = key.into();
                            let mut buf = [0; K::DYNAMIC_KEYMAP_MACRO_BUFFER_SIZE as usize];

                            // Read data
                            let stored_len =
                                match database.read_raw(K::get_storage_buffer(), key).await {
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
                                .write_raw(K::get_storage_buffer(), key, &buf[..new_length])
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
                            match database.read_raw(K::get_storage_buffer(), key).await {
                                Ok((stored_data, stored_len)) => {
                                    buf[..stored_len].copy_from_slice(stored_data);
                                }
                                Err(()) => {
                                    warn!("[VIA] Could not read dynamic keymap encoder.");
                                }
                            };

                            // Update data
                            buf[offset..(offset + len)].copy_from_slice(&data[..len]);

                            if let Err(()) =
                                database.write_raw(K::get_storage_buffer(), key, &buf).await
                            {
                                warn!("[VIA] Could not write dynamic keymap encoder.")
                            };
                        }
                    }
                }
                Operation::Delete => {
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
