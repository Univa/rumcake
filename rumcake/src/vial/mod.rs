//! Support for Vial's protocol (version 6).
//!
//! To use Vial, you will need to implement [`ViaKeyboard`] and [`VialKeyboard`].

use crate::backlight::{BacklightMatrixDevice, EmptyBacklightMatrix};
use defmt::assert;
use embassy_futures::join;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use smart_leds::RGB8;

use crate::hw::mcu::RawMutex;
use crate::via::{ViaKeyboard, VIA_REPORT_HID_RECEIVE_CHANNEL, VIA_REPORT_HID_SEND_CHANNEL};

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
    type BacklightMatrixDevice: BacklightMatrixDevice = EmptyBacklightMatrix;
    fn get_backlight_matrix() -> Option<
        crate::backlight::BacklightMatrix<
            { <Self::BacklightMatrixDevice as BacklightMatrixDevice>::LIGHTING_COLS },
            { <Self::BacklightMatrixDevice as BacklightMatrixDevice>::LIGHTING_ROWS },
        >,
    > {
        None
    }
}

/// Channel used to update the frame buffer for the
/// [`crate::backlight::rgb_backlight_matrix::animations::BacklightEffect::DirectSet`] effect.
pub(crate) static VIAL_DIRECT_SET_CHANNEL: Channel<RawMutex, (u8, RGB8), 4> = Channel::new();

#[rumcake_macros::task]
pub async fn vial_process_task<K: VialKeyboard + 'static>(_k: K)
where
    [(); K::BacklightMatrixDevice::LIGHTING_COLS]:,
    [(); K::BacklightMatrixDevice::LIGHTING_ROWS]:,
    [(); (K::LAYOUT_COLS + u8::BITS as usize - 1) / u8::BITS as usize * K::LAYOUT_ROWS]:,
    [(); K::LAYERS]:,
    [(); K::LAYOUT_ROWS]:,
    [(); K::LAYOUT_COLS]:,
    [(); K::DYNAMIC_KEYMAP_MACRO_BUFFER_SIZE as usize]:,
    [(); K::DYNAMIC_KEYMAP_MACRO_COUNT as usize]:,
{
    assert!(K::DYNAMIC_KEYMAP_LAYER_COUNT <= K::LAYERS);
    assert!(K::DYNAMIC_KEYMAP_LAYER_COUNT <= 16);
    assert!(K::VIAL_UNLOCK_COMBO.len() < 15);
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

    let vial_state: Mutex<RawMutex, protocol::VialState> = Mutex::new(Default::default());
    let via_state: Mutex<RawMutex, protocol::via::ViaState<K>> = Mutex::new(Default::default());

    if K::VIAL_INSECURE {
        vial_state.lock().await.unlocked = true;
    }

    let report_fut = async {
        loop {
            let mut report = VIA_REPORT_HID_RECEIVE_CHANNEL.receive().await;

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

                VIA_REPORT_HID_SEND_CHANNEL.send(report).await;
            }
        }
    };

    join::join(report_fut, protocol::via::background_task::<K>(&via_state)).await;
}

#[cfg(feature = "storage")]
pub mod storage {
    use embassy_sync::channel::Channel;
    use embassy_sync::signal::Signal;

    use crate::hw::mcu::RawMutex;
    use crate::storage::{FlashStorage, StorageDevice, StorageKey};

    use super::VialKeyboard;

    pub(super) enum VialStorageKeys {
        DynamicKeymapTapDance,
        DynamicKeymapCombo,
        DynamicKeymapKeyOverride,
    }

    impl From<VialStorageKeys> for StorageKey {
        fn from(value: VialStorageKeys) -> Self {
            match value {
                VialStorageKeys::DynamicKeymapTapDance => StorageKey::DynamicKeymapTapDance,
                VialStorageKeys::DynamicKeymapCombo => StorageKey::DynamicKeymapCombo,
                VialStorageKeys::DynamicKeymapKeyOverride => StorageKey::DynamicKeymapKeyOverride,
            }
        }
    }

    enum Operation {
        Write([u8; 32], VialStorageKeys, usize, usize),
        Delete,
    }

    /// A function that dispatches a flash operation to the Vial storage task. This will obtain a
    /// lock, and hold onto it until the storage task signals a completion. `offset` corresponds to
    /// the first byte of the stored data for the given `key` that we want to update. For example,
    /// if [0x23, 0x65, 0xEB] is stored in flash for the key `LayoutOptions`, and we want to update
    /// the last 2 bytes, we would pass in an offset of 1, and a `data` slice with a length of 2.
    pub(super) async fn update_data(key: VialStorageKeys, offset: usize, data: &[u8]) {
        // TODO: this function will wait eternally if vial_storage_task is not there
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
        OPERATION_COMPLETE.wait().await
    }

    static OPERATION_COMPLETE: Signal<RawMutex, ()> = Signal::new();
    static OPERATION_CHANNEL: Channel<RawMutex, Operation, 1> = Channel::new();

    #[rumcake_macros::task]
    pub async fn vial_storage_task<K: StorageDevice + VialKeyboard + 'static, F: FlashStorage>(
        _k: K,
        database: &crate::storage::StorageService<'static, F>,
    ) where
        [(); F::ERASE_SIZE]:,
    {
        // Initialize Vial data
        {
            // let tap_dance_metadata: [u8; core::mem::size_of::<TypeId>()] = unsafe {core::mem::transmute(TypeId::of::<>())};
            let tap_dance_metadata = [1];
            let _ = database
                .check_metadata(
                    K::get_storage_buffer(),
                    StorageKey::DynamicKeymapTapDance,
                    &tap_dance_metadata,
                )
                .await;

            // let combo_metadata: [u8; core::mem::size_of::<TypeId>()] = unsafe {core::mem::transmute(TypeId::of::<>())};
            let combo_metadata = [1];
            let _ = database
                .check_metadata(
                    K::get_storage_buffer(),
                    StorageKey::DynamicKeymapCombo,
                    &combo_metadata,
                )
                .await;

            // let key_override_metadata: [u8; core::mem::size_of::<TypeId>()] = unsafe {core::mem::transmute(TypeId::of::<>())};
            let key_override_metadata = [1];
            let _ = database
                .check_metadata(
                    K::get_storage_buffer(),
                    StorageKey::DynamicKeymapKeyOverride,
                    &key_override_metadata,
                )
                .await;
        }

        loop {
            match OPERATION_CHANNEL.receive().await {
                Operation::Write(data, key, offset, len) => match key {
                    VialStorageKeys::DynamicKeymapTapDance => {}
                    VialStorageKeys::DynamicKeymapCombo => {}
                    VialStorageKeys::DynamicKeymapKeyOverride => {}
                },
                Operation::Delete => {
                    let _ = database.delete(StorageKey::DynamicKeymapTapDance).await;
                    let _ = database.delete(StorageKey::DynamicKeymapCombo).await;
                    let _ = database.delete(StorageKey::DynamicKeymapKeyOverride).await;
                }
            }
        }
    }
}
