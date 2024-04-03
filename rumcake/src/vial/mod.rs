//! Support for Vial's protocol (version 6).
//!
//! To use Vial, you will need to implement [`ViaKeyboard`] and [`VialKeyboard`].

use defmt::assert;
use embassy_futures::join;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use smart_leds::RGB8;

use crate::hw::platform::RawMutex;
use crate::hw::HIDDevice;
use crate::keyboard::KeyboardLayout;
use crate::lighting::private::EmptyLightingDevice;
use crate::lighting::rgb_backlight_matrix::private::MaybeRGBBacklightMatrixDevice;
use crate::lighting::BacklightMatrixDevice;
use crate::storage::{FlashStorage, StorageDevice, StorageKey};
use crate::via::ViaKeyboard;

mod handlers;

pub(crate) mod protocol;

pub use rumcake_macros::enable_vial_rgb;

/// A trait that keyboards must implement to use the Vial protocol.
pub trait VialKeyboard: ViaKeyboard {
    const VIAL_ENABLED: bool = true;

    /// Whether Vial should not require your keyboard to be unlocked to use certain features.
    ///
    /// Enabling this will require your keyboard to be unlocked using the specified
    /// [`VialKeyboard::VIAL_UNLOCK_COMBO`] in order to use certain dynamic keymap features.
    const VIAL_INSECURE: bool = false;

    /// Unique 8-byte identifier for your keyboard.
    const VIAL_KEYBOARD_UID: [u8; 8];

    /// Matrix positions used to unlock your keyboard to use with Vial. Tuples in this array should
    /// be in the form of `(row, col)`. The combination must have less than 15 keys.
    const VIAL_UNLOCK_COMBO: &'static [(u8, u8)]; // Matrix positions to use as the unlock combo for Vial

    /// Raw bytes for an LZMA-compressed Vial JSON definition.
    const KEYBOARD_DEFINITION: &'static [u8];

    /// Whether RGB lighting features should be used. Usage of VialRGB assumes you have the
    /// [`rgb-backlight-matrix`] feature flag enabled. To enable this, you should use
    /// [`enable_vial_rgb`] instead of implementing this yourself.
    const VIALRGB_ENABLE: bool = false;
    const VIAL_TAP_DANCE_ENTRIES: u8 = 0; // TODO: Change when tap dance is implemented
    const VIAL_COMBO_ENTRIES: u8 = 0; // TODO: Change when combo is implemented
    const VIAL_KEY_OVERRIDE_ENTRIES: u8 = 0; // TODO: Change when key override is implemented

    // TODO: replace with specialization if it doesn't cause an ICE
    type RGBBacklightMatrixDevice: MaybeRGBBacklightMatrixDevice = EmptyLightingDevice;
    fn get_backlight_matrix() -> Option<
        crate::lighting::BacklightMatrix<
            { <Self::RGBBacklightMatrixDevice as BacklightMatrixDevice>::LIGHTING_COLS },
            { <Self::RGBBacklightMatrixDevice as BacklightMatrixDevice>::LIGHTING_ROWS },
        >,
    > {
        None
    }
}

/// Channel used to update the frame buffer for the
/// [`crate::lighting::rgb_backlight_matrix::RGBBacklightMatrixEffect::DirectSet`] effect.
pub(crate) static VIAL_DIRECT_SET_CHANNEL: Channel<RawMutex, (u8, RGB8), 4> = Channel::new();

#[rumcake_macros::task]
pub async fn vial_process_task<K: VialKeyboard + HIDDevice + 'static>(_k: K)
where
    [(); <<K::StorageType as StorageDevice>::FlashStorageType as FlashStorage>::ERASE_SIZE]:,
    [(); K::DYNAMIC_KEYMAP_LAYER_COUNT * K::Layout::LAYOUT_COLS * K::Layout::LAYOUT_ROWS * 2]:,
    [(); K::DYNAMIC_KEYMAP_LAYER_COUNT * K::Layout::NUM_ENCODERS * 2 * 2]:,
    [(); K::RGBBacklightMatrixDevice::LIGHTING_COLS]:,
    [(); K::RGBBacklightMatrixDevice::LIGHTING_ROWS]:,
    [(); (K::Layout::LAYOUT_COLS + u8::BITS as usize - 1) / u8::BITS as usize
        * K::Layout::LAYOUT_ROWS]:,
    [(); K::Layout::LAYERS]:,
    [(); K::DYNAMIC_KEYMAP_MACRO_BUFFER_SIZE as usize]:,
    [(); K::DYNAMIC_KEYMAP_MACRO_COUNT as usize]:,
{
    assert!(K::DYNAMIC_KEYMAP_LAYER_COUNT <= K::Layout::LAYERS);
    assert!(K::DYNAMIC_KEYMAP_LAYER_COUNT <= 16);
    assert!(K::VIAL_UNLOCK_COMBO.len() < 15);
    if <K as ViaKeyboard>::get_macro_buffer().is_some() {
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

    let vial_state: Mutex<RawMutex, protocol::VialState> = Mutex::new(Default::default());
    let via_state: Mutex<RawMutex, protocol::via::ViaState<K>> = Mutex::new(Default::default());
    let receive_channel = K::get_via_hid_receive_channel();
    let send_channel = K::get_via_hid_send_channel();

    if K::VIAL_INSECURE {
        vial_state.lock().await.unlocked = true;
    }

    let report_fut = async {
        loop {
            let mut report = receive_channel.receive().await;

            if K::VIAL_ENABLED && K::VIA_ENABLED {
                {
                    let mut vial_state = vial_state.lock().await;
                    let mut via_state = via_state.lock().await;
                    protocol::process_vial_command::<K>(
                        &mut report,
                        &mut vial_state,
                        &mut via_state,
                    )
                    .await;
                }

                send_channel.send(report).await;
            }
        }
    };

    join::join(report_fut, protocol::via::background_task::<K>(&via_state)).await;
}

pub async fn initialize_vial_data<V: VialKeyboard + 'static>(_v: V)
where
    [(); <<V::StorageType as StorageDevice>::FlashStorageType as FlashStorage>::ERASE_SIZE]:,
{
    if let Some(database) = V::get_storage_service() {
        // let tap_dance_metadata: [u8; core::mem::size_of::<TypeId>()] = unsafe {core::mem::transmute(TypeId::of::<>())};
        let tap_dance_metadata = [1];
        let _ = database
            .check_metadata(StorageKey::DynamicKeymapTapDance, &tap_dance_metadata)
            .await;

        // let combo_metadata: [u8; core::mem::size_of::<TypeId>()] = unsafe {core::mem::transmute(TypeId::of::<>())};
        let combo_metadata = [1];
        let _ = database
            .check_metadata(StorageKey::DynamicKeymapCombo, &combo_metadata)
            .await;

        // let key_override_metadata: [u8; core::mem::size_of::<TypeId>()] = unsafe {core::mem::transmute(TypeId::of::<>())};
        let key_override_metadata = [1];
        let _ = database
            .check_metadata(StorageKey::DynamicKeymapKeyOverride, &key_override_metadata)
            .await;
    }
}
