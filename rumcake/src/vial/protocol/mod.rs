use super::VialKeyboard;
use crate::backlight::BacklightMatrixDevice;
use crate::via::handlers::{dynamic_keymap_get_encoder, dynamic_keymap_set_encoder};
use crate::vial::handlers::*;
use defmt::{info, warn, Debug2Format};
use num_derive::FromPrimitive;

pub(super) mod via;
use via::{process_via_command, ViaCommandId};

pub(super) mod lighting;
#[cfg(feature = "rgb-backlight-matrix")]
pub(super) mod vialrgb;

pub(crate) const VIAL_PROTOCOL_VERSION: u32 = 0x00000006;
pub(super) const VIALRGB_PROTOCOL_VERSION: u16 = 1;

// Packets must be of 32 bytes
pub(crate) const VIAL_RAW_EPSIZE: usize = 32;

#[derive(FromPrimitive, Debug)]
pub(crate) enum VialCommandId {
    GetKeyboardId = 0x00,
    GetSize,
    GetDef,
    GetEncoder,
    SetEncoder,
    GetUnlockStatus,
    UnlockStart,
    UnlockPoll,
    Lock,
    QmkSettingsQuery,
    QmkSettingsGet,
    QmkSettingsSet,
    QmkSettingsReset,
    DynamicEntryOp,
}

#[derive(FromPrimitive, Debug)]
enum VialDynamicValue {
    GetNumberOfEntries = 0x00,
    TapDanceGet,
    TapDanceSet,
    ComboGet,
    ComboSet,
    KeyOverrideGet,
    KeyOverrideSet,
}

#[derive(Default)]
pub(crate) struct VialState {
    pub(crate) unlocked: bool,
    pub(crate) unlock_in_progress: bool,
    pub(crate) unlock_counter: u8,
    pub(crate) unlock_timer: u32,
}

pub(crate) async fn process_vial_command<K: VialKeyboard + 'static>(
    data: &mut [u8],
    vial_state: &mut VialState,
    via_state: &mut via::ViaState<K>,
) where
    [(); K::BacklightMatrixDevice::LIGHTING_COLS]:,
    [(); K::BacklightMatrixDevice::LIGHTING_ROWS]:,
    [(); (K::LAYOUT_COLS + u8::BITS as usize - 1) / u8::BITS as usize * K::LAYOUT_ROWS]:,
    [(); K::LAYERS]:,
    [(); K::LAYOUT_ROWS]:,
    [(); K::LAYOUT_COLS]:,
{
    if K::handle_via_command(data) {
        return;
    }

    info!("[VIAL] Processing Vial command");
    if let Some(command) = num::FromPrimitive::from_u8(data[0]) {
        info!("[VIAL] Received command {:?}", Debug2Format(&command));

        if vial_state.unlock_in_progress && !matches!(command, ViaCommandId::VialPrefix) {
            return;
        }

        match command {
            ViaCommandId::VialPrefix => {
                if K::VIAL_ENABLED {
                    if let Some(cmd) = num::FromPrimitive::from_u8(data[1]) {
                        info!("[VIAL] Received command {:?}", Debug2Format(&cmd));
                        if vial_state.unlock_in_progress
                            && !matches!(
                                cmd,
                                VialCommandId::GetKeyboardId
                                    | VialCommandId::GetSize
                                    | VialCommandId::GetDef
                                    | VialCommandId::GetUnlockStatus
                                    | VialCommandId::UnlockStart
                                    | VialCommandId::UnlockPoll
                            )
                        {
                            return;
                        }

                        match cmd {
                            VialCommandId::GetKeyboardId => {
                                get_keyboard_id::<K>(VIAL_PROTOCOL_VERSION, data)
                            }
                            VialCommandId::GetSize => get_definition_size::<K>(data),
                            VialCommandId::GetDef => get_definition::<K>(data),
                            VialCommandId::GetEncoder => {
                                let layer = data[2];
                                let encoder_id = data[3];
                                dynamic_keymap_get_encoder::<K>(
                                    layer,
                                    encoder_id,
                                    false,
                                    &mut data[0..=1],
                                )
                                .await;
                                dynamic_keymap_get_encoder::<K>(
                                    layer,
                                    encoder_id,
                                    true,
                                    &mut data[2..=3],
                                )
                                .await;
                            }
                            VialCommandId::SetEncoder => {
                                let layer = data[2];
                                let encoder_id = data[3];
                                let clockwise = data[4] != 0;
                                dynamic_keymap_set_encoder::<K>(
                                    layer,
                                    encoder_id,
                                    clockwise,
                                    &mut data[5..=6],
                                )
                                .await;
                            }
                            VialCommandId::GetUnlockStatus => {
                                get_unlock_status::<K>(data, vial_state)
                            }
                            VialCommandId::UnlockStart => unlock_start(vial_state),
                            VialCommandId::UnlockPoll => {
                                unlock_poll::<K>(data, vial_state, via_state)
                            }
                            VialCommandId::Lock => lock(vial_state),
                            VialCommandId::QmkSettingsQuery => qmk_settings_query(data),
                            VialCommandId::QmkSettingsGet => qmk_settings_get(data),
                            VialCommandId::QmkSettingsSet => qmk_settings_set(data),
                            VialCommandId::QmkSettingsReset => qmk_settings_reset(data),
                            VialCommandId::DynamicEntryOp => {
                                if let Some(cmd) = num::FromPrimitive::from_u8(data[2]) {
                                    match cmd {
                                        VialDynamicValue::GetNumberOfEntries => {
                                            dynamic_keymap_get_number_of_entries::<K>(data)
                                        }
                                        VialDynamicValue::TapDanceGet => {
                                            dynamic_keymap_get_tap_dance(data)
                                        }
                                        VialDynamicValue::TapDanceSet => {
                                            dynamic_keymap_set_tap_dance(data)
                                        }
                                        VialDynamicValue::ComboGet => {
                                            dynamic_keymap_get_combo(data)
                                        }
                                        VialDynamicValue::ComboSet => {
                                            dynamic_keymap_set_combo(data)
                                        }
                                        VialDynamicValue::KeyOverrideGet => {
                                            dynamic_keymap_get_key_override(data)
                                        }
                                        VialDynamicValue::KeyOverrideSet => {
                                            dynamic_keymap_set_key_override(data)
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {
                process_via_command::<K>(data, via_state, vial_state).await;
            }
        }
    } else {
        warn!("[VIAL] Unknown command received from host {:?}", data[0]);
    }
}
