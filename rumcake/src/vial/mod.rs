//! Support for Vial's protocol (version 6).
//!
//! To use Vial, you will need to implement [`ViaKeyboard`] and [`VialKeyboard`].

use crate::backlight::{BacklightMatrixDevice, EmptyBacklightMatrix};
use defmt::{debug, error, Debug2Format};
use embassy_futures::join;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_usb::class::hid::HidWriter;
use embassy_usb::driver::Driver;
use smart_leds::RGB8;

use crate::via::{ViaKeyboard, VIA_REPORT_HID_SEND_CHANNEL};

mod handlers;

pub(crate) mod protocol;

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
        &'static crate::backlight::BacklightMatrix<
            { <Self::BacklightMatrixDevice as BacklightMatrixDevice>::LIGHTING_COLS },
            { <Self::BacklightMatrixDevice as BacklightMatrixDevice>::LIGHTING_ROWS },
        >,
    > {
        None
    }

    #[cfg(feature = "storage")]
    fn get_dynamic_keymap_tap_dance_storage_state(
    ) -> &'static mut crate::storage::StorageServiceState<1, 1>;

    #[cfg(feature = "storage")]
    fn get_dynamic_keymap_combo_storage_state(
    ) -> &'static mut crate::storage::StorageServiceState<1, 1>;

    #[cfg(feature = "storage")]
    fn get_dynamic_keymap_key_override_storage_state(
    ) -> &'static mut crate::storage::StorageServiceState<1, 1>;
}

#[macro_export]
macro_rules! enable_vial_rgb {
    () => {
        const VIALRGB_ENABLE: bool = true;
        type BacklightMatrixDevice = Self;
        fn get_backlight_matrix() -> Option<
            &'static $crate::backlight::BacklightMatrix<
                { <Self::BacklightMatrixDevice as $crate::backlight::BacklightMatrixDevice>::LIGHTING_COLS },
                { <Self::BacklightMatrixDevice as $crate::backlight::BacklightMatrixDevice>::LIGHTING_ROWS },
            >,
        > {
            Some(<Self::BacklightMatrixDevice as $crate::backlight::BacklightMatrixDevice>::get_backlight_matrix())
        }
    };
}

/// Channel used to update the frame buffer for the
/// [`crate::backlight::animations::BacklightEffect::DirectSet`] effect.
pub(crate) static VIAL_DIRECT_SET_CHANNEL: Channel<ThreadModeRawMutex, (u8, RGB8), 4> =
    Channel::new();

#[rumcake_macros::task]
pub async fn usb_hid_vial_write_task<K: VialKeyboard + 'static>(
    _k: K,
    mut hid: HidWriter<'static, impl Driver<'static>, 32>,
) where
    [(); K::BacklightMatrixDevice::LIGHTING_COLS]:,
    [(); K::BacklightMatrixDevice::LIGHTING_ROWS]:,
    [(); (K::LAYOUT_COLS + u8::BITS as usize - 1) / u8::BITS as usize * K::LAYOUT_ROWS]:,
    [(); K::LAYERS]:,
    [(); K::LAYOUT_ROWS]:,
    [(); K::LAYOUT_COLS]:,
{
    assert!(K::DYNAMIC_KEYMAP_LAYER_COUNT <= K::LAYERS);
    assert!(K::VIAL_UNLOCK_COMBO.len() < 15);

    let vial_state: Mutex<ThreadModeRawMutex, protocol::VialState> = Mutex::new(Default::default());
    let via_state: Mutex<ThreadModeRawMutex, protocol::via::ViaState<K>> =
        Mutex::new(Default::default());

    if K::VIAL_INSECURE {
        vial_state.lock().await.unlocked = true;
    }

    let report_fut = async {
        loop {
            let mut report = VIA_REPORT_HID_SEND_CHANNEL.receive().await;

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

                debug!("[VIAL] Writing HID raw report {:?}", Debug2Format(&report));
                if let Err(err) = hid.write(&report).await {
                    error!(
                        "[VIAL] Couldn't write HID raw report: {:?}",
                        Debug2Format(&err)
                    );
                };
            }
        }
    };

    join::join(report_fut, protocol::via::background_task::<K>(&via_state)).await;
}

#[cfg(feature = "storage")]
pub mod storage {
    use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
    use embassy_sync::channel::Channel;
    use embassy_sync::signal::Signal;
    use embedded_storage_async::nor_flash::NorFlash;

    use crate::storage::StorageKey;

    use super::VialKeyboard;

    #[macro_export]
    macro_rules! setup_vial_storage_buffers {
        ($k:ident) => {
            fn get_dynamic_keymap_tap_dance_storage_state(
            ) -> &'static mut $crate::storage::StorageServiceState<1, 1> {
                static mut DYNAMIC_KEYMAP_TAP_DANCE_STORAGE_STATE:
                    $crate::storage::StorageServiceState<1, 1> =
                    $crate::storage::StorageServiceState::new();
                unsafe { &mut DYNAMIC_KEYMAP_TAP_DANCE_STORAGE_STATE }
            }

            fn get_dynamic_keymap_combo_storage_state(
            ) -> &'static mut $crate::storage::StorageServiceState<1, 1> {
                static mut DYNAMIC_KEYMAP_COMBO_STORAGE_STATE:
                    $crate::storage::StorageServiceState<1, 1> =
                    $crate::storage::StorageServiceState::new();
                unsafe { &mut DYNAMIC_KEYMAP_COMBO_STORAGE_STATE }
            }

            fn get_dynamic_keymap_key_override_storage_state(
            ) -> &'static mut $crate::storage::StorageServiceState<1, 1> {
                static mut DYNAMIC_KEYMAP_KEY_OVERRIDE_STORAGE_STATE:
                    $crate::storage::StorageServiceState<1, 1> =
                    $crate::storage::StorageServiceState::new();
                unsafe { &mut DYNAMIC_KEYMAP_KEY_OVERRIDE_STORAGE_STATE }
            }
        };
    }

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

    static OPERATION_COMPLETE: Signal<ThreadModeRawMutex, ()> = Signal::new();
    static OPERATION_CHANNEL: Channel<ThreadModeRawMutex, Operation, 1> = Channel::new();

    #[rumcake_macros::task]
    pub async fn vial_storage_task<K: VialKeyboard + 'static, F: NorFlash>(
        _k: K,
        database: &'static crate::storage::Database<'static, F>,
    ) where
        [(); F::ERASE_SIZE]:,
    {
        // Initialize Vial data
        {
            let mut database = database.lock().await;

            // let tap_dance_metadata: [u8; core::mem::size_of::<TypeId>()] = unsafe {core::mem::transmute(TypeId::of::<>())};
            let tap_dance_metadata = [1];
            let _ = database
                .initialize(
                    K::get_dynamic_keymap_tap_dance_storage_state(),
                    StorageKey::DynamicKeymapTapDance,
                    &tap_dance_metadata,
                )
                .await;

            // let combo_metadata: [u8; core::mem::size_of::<TypeId>()] = unsafe {core::mem::transmute(TypeId::of::<>())};
            let combo_metadata = [1];
            let _ = database
                .initialize(
                    K::get_dynamic_keymap_combo_storage_state(),
                    StorageKey::DynamicKeymapCombo,
                    &combo_metadata,
                )
                .await;

            // let key_override_metadata: [u8; core::mem::size_of::<TypeId>()] = unsafe {core::mem::transmute(TypeId::of::<>())};
            let key_override_metadata = [1];
            let _ = database
                .initialize(
                    K::get_dynamic_keymap_key_override_storage_state(),
                    StorageKey::DynamicKeymapKeyOverride,
                    &key_override_metadata,
                )
                .await;
        }

        loop {
            match OPERATION_CHANNEL.receive().await {
                Operation::Write(data, key, offset, len) => {
                    let mut database = database.lock().await;

                    match key {
                        VialStorageKeys::DynamicKeymapTapDance => {}
                        VialStorageKeys::DynamicKeymapCombo => {}
                        VialStorageKeys::DynamicKeymapKeyOverride => {}
                    }
                }
                Operation::Delete => {
                    let mut database = database.lock().await;
                    let _ = database.delete(StorageKey::DynamicKeymapTapDance).await;
                    let _ = database.delete(StorageKey::DynamicKeymapCombo).await;
                    let _ = database.delete(StorageKey::DynamicKeymapKeyOverride).await;
                }
            }
        }
    }
}
