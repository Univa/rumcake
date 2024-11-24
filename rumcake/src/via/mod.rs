//! Support for Via's protocol (version 12).
//!
//! To use Via, you will need to implement [`ViaKeyboard`]. If you would like to save your Via
//! changes, you will also need to enable the `storage` feature flag, and setup the appropriate
//! storage buffers using [`crate::setup_via_storage_buffers`].

use defmt::assert;
use embassy_futures::join;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;

use crate::hw::platform::RawMutex;
use crate::hw::HIDDevice;
use crate::keyboard::KeyboardLayout;
use crate::storage::private::EmptyStorageDevice;
use crate::storage::{FlashStorage, StorageDevice, StorageKey, StorageService};

pub(crate) mod handlers;
pub(crate) mod protocol_12;

pub(crate) use protocol_12 as protocol;

pub use rumcake_macros::{connect_storage_service, setup_macro_buffer};

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
pub trait ViaKeyboard {
    /// The layout that this Via instance will control.
    type Layout: KeyboardLayout;

    /// The storage device used to store Via data.
    type StorageType: StorageDevice = EmptyStorageDevice;
    fn get_storage_service() -> Option<
        &'static StorageService<
            'static,
            <Self::StorageType as StorageDevice>::FlashStorageType,
            Self::StorageType,
        >,
    >
    where
        [(); <<Self::StorageType as StorageDevice>::FlashStorageType as FlashStorage>::ERASE_SIZE]:,
    {
        None
    }

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
    const DYNAMIC_KEYMAP_LAYER_COUNT: usize = Self::Layout::LAYERS;
    // const DYNAMIC_KEYMAP_LAYER_COUNT: usize = 4; // This is the default if this variable isn't defined in QMK

    /// The number of macros that your keyboard can store. You should use [`setup_macro_buffer`] to
    /// implement this. If you plan on using macros, this should be non-zero.
    const DYNAMIC_KEYMAP_MACRO_COUNT: u8 = 0; // This is the default if this variable isn't defined in QMK

    /// The total amount of bytes that can be used to store macros assigned by Via. You should use
    /// [`setup_macro_buffer`] to implement this. If you plan on using macros, this should be
    /// non-zero.
    const DYNAMIC_KEYMAP_MACRO_BUFFER_SIZE: u16 = 0;

    /// Determines how QK_BACKLIGHT keycodes should be converted to a [`crate::keyboard::Keycode`]
    /// and vice versa. If this is `None`, then backlighting keycodes will not be converted.
    const BACKLIGHT_TYPE: Option<BacklightType> = None;

    /// Obtain a reference to macro data created by Via. You should use [`setup_macro_buffer`] to
    /// implement this. If this returns `Some`, then [`ViaKeyboard::DYNAMIC_KEYMAP_MACRO_COUNT`]
    /// and [`ViaKeyboard::DYNAMIC_KEYMAP_MACRO_BUFFER_SIZE`] should be non-zero. Otherwise,
    /// [`ViaKeyboard::DYNAMIC_KEYMAP_MACRO_COUNT`] should be 0.
    fn get_macro_buffer() -> Option<
        &'static mut MacroBuffer<
            'static,
            { Self::DYNAMIC_KEYMAP_MACRO_BUFFER_SIZE as usize },
            { Self::DYNAMIC_KEYMAP_MACRO_COUNT as usize },
        >,
    > {
        None
    }

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

pub async fn via_process_task<K: ViaKeyboard + 'static, T: HIDDevice + 'static>(_k: K, _t: T)
where
    [(); <<K::StorageType as StorageDevice>::FlashStorageType as FlashStorage>::ERASE_SIZE]:,
    [(); K::DYNAMIC_KEYMAP_LAYER_COUNT * K::Layout::LAYOUT_COLS * K::Layout::LAYOUT_ROWS * 2]:,
    [(); K::DYNAMIC_KEYMAP_LAYER_COUNT * K::Layout::NUM_ENCODERS * 2 * 2]:,
    [(); (K::Layout::LAYOUT_COLS + u8::BITS as usize - 1) / u8::BITS as usize
        * K::Layout::LAYOUT_ROWS]:,
    [(); K::Layout::LAYERS]:,
    [(); K::Layout::LAYOUT_ROWS]:,
    [(); K::Layout::LAYOUT_COLS]:,
    [(); K::DYNAMIC_KEYMAP_MACRO_BUFFER_SIZE as usize]:,
    [(); K::DYNAMIC_KEYMAP_MACRO_COUNT as usize]:,
{
    assert!(K::DYNAMIC_KEYMAP_LAYER_COUNT <= K::Layout::LAYERS);
    assert!(K::DYNAMIC_KEYMAP_LAYER_COUNT <= 16);
    if K::get_macro_buffer().is_some() {
        assert!(
            K::DYNAMIC_KEYMAP_MACRO_BUFFER_SIZE > 0,
            "Macro buffer size must be greater than 0 if you are using Via macros."
        );
        assert!(
            K::DYNAMIC_KEYMAP_MACRO_COUNT > 0,
            "Macro count must be greater than 0 if you are using Via macros."
        );
    } else {
        assert!(
            K::DYNAMIC_KEYMAP_MACRO_COUNT == 0,
            "Macro count should be 0 if you are not using Via macros."
        );
    }

    let via_state: Mutex<RawMutex, protocol::ViaState<K>> = Mutex::new(Default::default());
    let receive_channel = T::get_via_hid_receive_channel();
    let send_channel = T::get_via_hid_send_channel();

    let report_fut = async {
        loop {
            let mut report = receive_channel.receive().await;

            if K::VIA_ENABLED {
                {
                    let mut via_state = via_state.lock().await;
                    protocol::process_via_command::<K>(&mut report, &mut via_state).await;
                }

                send_channel.send(report).await;
            }
        }
    };

    join::join(report_fut, protocol::background_task::<K>(&via_state)).await;
}

static VIA_LAYOUT_OPTIONS: Signal<RawMutex, u32> = Signal::new();

pub async fn initialize_via_data<V: ViaKeyboard + 'static>(_v: V)
where
    [(); <<V::StorageType as StorageDevice>::FlashStorageType as FlashStorage>::ERASE_SIZE]:,
    [(); V::VIA_EEPROM_LAYOUT_OPTIONS_SIZE]:,
    [(); V::DYNAMIC_KEYMAP_LAYER_COUNT * V::Layout::LAYOUT_COLS * V::Layout::LAYOUT_ROWS * 2]:,
    [(); V::DYNAMIC_KEYMAP_LAYER_COUNT * V::Layout::NUM_ENCODERS * 2 * 2]:,
    [(); V::DYNAMIC_KEYMAP_MACRO_BUFFER_SIZE as usize]:,
    [(); V::DYNAMIC_KEYMAP_MACRO_COUNT as usize]:,
    [(); V::Layout::LAYERS]:,
    [(); V::Layout::LAYOUT_ROWS]:,
    [(); V::Layout::LAYOUT_COLS]:,
{
    if let Some(database) = V::get_storage_service() {
        // Initialize layout options
        let options_metadata = [V::VIA_EEPROM_LAYOUT_OPTIONS_SIZE as u8];
        let _ = database
            .check_metadata(crate::storage::StorageKey::LayoutOptions, &options_metadata)
            .await;
        if let Ok((stored_data, stored_len)) = database
            .read_raw(crate::storage::StorageKey::LayoutOptions)
            .await
        {
            let mut bytes = [0; 4];
            bytes[(4 - stored_len)..].copy_from_slice(&stored_data[..stored_len]);
            VIA_LAYOUT_OPTIONS.signal(u32::from_be_bytes(bytes))
        };

        // Initialize layout
        let layout_metadata = [
            V::DYNAMIC_KEYMAP_LAYER_COUNT as u8,
            V::Layout::LAYOUT_COLS as u8,
            V::Layout::LAYOUT_ROWS as u8,
        ];
        let _ = database
            .check_metadata(crate::storage::StorageKey::DynamicKeymap, &layout_metadata)
            .await;
        if let Ok((stored_data, stored_len)) = database
            .read_raw(crate::storage::StorageKey::DynamicKeymap)
            .await
        {
            // Load layout from flash
            let mut layout = V::Layout::get_layout().layout.lock().await;
            for byte in (0..stored_len).step_by(2) {
                if let Some(action) = protocol::keycodes::convert_keycode_to_action::<V>(
                    u16::from_be_bytes(stored_data[byte..byte + 2].try_into().unwrap()),
                ) {
                    let layer = byte / (V::Layout::LAYOUT_ROWS * V::Layout::LAYOUT_COLS * 2);
                    let row = (byte / (V::Layout::LAYOUT_COLS * 2)) % V::Layout::LAYOUT_ROWS;
                    let col = (byte / 2) % V::Layout::LAYOUT_COLS;

                    layout
                        .change_action((row as u8, col as u8), layer, action)
                        .unwrap();
                }
            }
        } else {
            // Save default layout to flash
            let mut layout = V::Layout::get_layout().layout.lock().await;
            let mut buf = [0; V::DYNAMIC_KEYMAP_LAYER_COUNT
                * V::Layout::LAYOUT_COLS
                * V::Layout::LAYOUT_ROWS
                * 2];
            for byte in (0..buf.len()).step_by(2) {
                let layer = byte / (V::Layout::LAYOUT_ROWS * V::Layout::LAYOUT_COLS * 2);
                let row = (byte / (V::Layout::LAYOUT_COLS * 2)) % V::Layout::LAYOUT_ROWS;
                let col = (byte / 2) % V::Layout::LAYOUT_COLS;

                buf[(byte)..(byte + 2)].copy_from_slice(
                    &protocol::keycodes::convert_action_to_keycode::<V>(
                        layout.get_action((row as u8, col as u8), layer).unwrap(),
                    )
                    .to_be_bytes(),
                );
            }
            let _ = database.write_raw(StorageKey::DynamicKeymap, &buf).await;
        };

        // Initialize encoder layout
        let encoder_metadata = [
            V::DYNAMIC_KEYMAP_LAYER_COUNT as u8,
            V::Layout::NUM_ENCODERS as u8,
        ];
        let _ = database
            .check_metadata(
                crate::storage::StorageKey::DynamicKeymapEncoder,
                &encoder_metadata,
            )
            .await;

        // Initialize macros
        let _ = database
            .check_metadata(
                crate::storage::StorageKey::DynamicKeymapMacro,
                &layout_metadata,
            )
            .await;
        if let Ok((stored_data, stored_len)) = database
            .read_raw(crate::storage::StorageKey::DynamicKeymapMacro)
            .await
        {
            if let Some(macro_data) = V::get_macro_buffer() {
                macro_data.update_buffer(0, &stored_data[..stored_len])
            }
        };
    }
}
