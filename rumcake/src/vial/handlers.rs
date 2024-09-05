use defmt::warn;
use smart_leds::hsv::hsv2rgb;

use super::protocol::via::ViaState;
use super::protocol::{VialState, VIAL_RAW_EPSIZE};
use super::{VialKeyboard, VIAL_DIRECT_SET_CHANNEL};
use crate::keyboard::KeyboardLayout;
use crate::lighting::BacklightMatrixDevice;
use crate::storage::{FlashStorage, StorageDevice, StorageKey};

// Unlike the other normal Via comands, Vial overwrites the command data received from the host

pub fn get_keyboard_id<K: VialKeyboard>(version: u32, data: &mut [u8]) {
    data[0..=3].copy_from_slice(&version.to_le_bytes());
    data[4..=11].copy_from_slice(&K::VIAL_KEYBOARD_UID);
    if K::VIALRGB_ENABLE {
        data[12] = 1;
    }
}

pub fn get_definition_size<K: VialKeyboard>(data: &mut [u8]) {
    data[0..=3].copy_from_slice(&(K::KEYBOARD_DEFINITION.len()).to_le_bytes())
}

pub fn get_definition<K: VialKeyboard>(data: &mut [u8]) {
    let page: u16 = u16::from_le_bytes(data[2..=3].try_into().unwrap());
    let start = page as usize * VIAL_RAW_EPSIZE;
    let mut end = start + VIAL_RAW_EPSIZE;

    if end < start || start >= K::KEYBOARD_DEFINITION.len() {
        return;
    }

    if end > K::KEYBOARD_DEFINITION.len() {
        end = K::KEYBOARD_DEFINITION.len()
    }

    data[0..(end - start)].copy_from_slice(&K::KEYBOARD_DEFINITION[start..end])
}

pub fn get_unlock_status<K: VialKeyboard>(data: &mut [u8], vial_state: &VialState) {
    data.fill(0xFF);
    data[0] = vial_state.unlocked as u8;
    data[1] = vial_state.unlock_in_progress as u8;

    if !K::VIAL_INSECURE {
        for i in 0..K::VIAL_UNLOCK_COMBO.len() {
            data[2 + i * 2] = K::VIAL_UNLOCK_COMBO[i].0;
            data[2 + i * 2 + 1] = K::VIAL_UNLOCK_COMBO[i].1;
        }
    }
}

pub fn unlock_start(vial_state: &mut VialState) {
    vial_state.unlock_in_progress = true;
    vial_state.unlock_counter = 50;
    vial_state.unlock_timer = embassy_time::Instant::now().as_millis() as u32
}

pub fn unlock_poll<K: VialKeyboard>(
    data: &mut [u8],
    vial_state: &mut VialState,
    via_state: &ViaState<K>,
) where
    [(); (K::Layout::LAYOUT_COLS + u8::BITS as usize - 1) / u8::BITS as usize
        * K::Layout::LAYOUT_ROWS]:,
{
    if !K::VIAL_INSECURE && vial_state.unlock_in_progress {
        let holding = K::VIAL_UNLOCK_COMBO.iter().all(|(row, col)| {
            // get the byte that stores the bit for the corresponding matrix coordinate
            // see [`crate::via::protocol::background_task`]
            let byte = (K::Layout::LAYOUT_COLS + u8::BITS as usize - 1) / u8::BITS as usize
                * (*row as usize + 1)
                - 1
                - *col as usize / u8::BITS as usize;
            let bit = 1 << (*col as usize % u8::BITS as usize);

            (via_state.layout_state[byte] & bit) == bit
        });

        if embassy_time::Instant::now().as_millis() as u32 - vial_state.unlock_timer > 100
            && holding
        {
            vial_state.unlock_timer = embassy_time::Instant::now().as_millis() as u32;

            vial_state.unlock_counter -= 1;
            if vial_state.unlock_counter == 0 {
                vial_state.unlock_in_progress = false;
                vial_state.unlocked = true;
            }
        } else {
            vial_state.unlock_counter = 50
        }
    };

    data[0] = vial_state.unlocked as u8;
    data[1] = vial_state.unlock_in_progress as u8;
    data[2] = vial_state.unlock_counter;
}

pub fn lock(vial_state: &mut VialState) {
    vial_state.unlocked = false
}

pub fn vialrgb_get_info(version: u16, data: &mut [u8]) {
    data[0..=1].copy_from_slice(&version.to_le_bytes());
    data[2] = 255; // TODO: make this configurable? (max brightness)
}

#[cfg(feature = "rgb-backlight-matrix")]
pub async fn vialrgb_get_mode<K: VialKeyboard + 'static>(data: &mut [u8])
where
    [(); <K::Layout as KeyboardLayout>::RGBBacklightMatrixDeviceType::LIGHTING_COLS]:,
    [(); <K::Layout as KeyboardLayout>::RGBBacklightMatrixDeviceType::LIGHTING_ROWS]:,
{
    if K::VIALRGB_ENABLE {
        if let Some(state) = <<K::Layout as KeyboardLayout>::RGBBacklightMatrixDeviceType as crate::lighting::rgb_backlight_matrix::private::MaybeRGBBacklightMatrixDevice>::get_state() {
            let config = state.get().await;
            if !config.enabled {
                data[0..=1].copy_from_slice(&[0; 2])
            } else {
                data[0..=1].copy_from_slice(
                    &super::protocol::vialrgb::convert_effect_to_vialrgb_id(config.effect)
                        .to_le_bytes(),
                );
            }
            data[2] = config.speed;
            data[3] = config.hue;
            data[4] = config.sat;
            data[5] = config.val;
        }
    }
}

#[cfg(feature = "rgb-backlight-matrix")]
pub fn vialrgb_get_supported<K: VialKeyboard + 'static>(data: &mut [u8])
where
    [(); <K::Layout as KeyboardLayout>::RGBBacklightMatrixDeviceType::LIGHTING_COLS]:,
    [(); <K::Layout as KeyboardLayout>::RGBBacklightMatrixDeviceType::LIGHTING_ROWS]:,
{
    if K::VIALRGB_ENABLE {
        let gt = u16::from_le_bytes(data[0..=1].try_into().unwrap());
        data.fill(0xFF);
        for id in gt..=super::protocol::vialrgb::MAX_VIALRGB_ID {
            if super::protocol::vialrgb::is_supported::<
                <K::Layout as KeyboardLayout>::RGBBacklightMatrixDeviceType,
            >(id)
            {
                data[(id as usize - gt as usize)..=(id as usize - gt as usize + 1)]
                    .copy_from_slice(&id.to_le_bytes())
            }
        }
    }
}

#[cfg(feature = "rgb-backlight-matrix")]
pub fn vialrgb_get_num_leds<K: VialKeyboard + 'static>(data: &mut [u8])
where
    [(); <K::Layout as KeyboardLayout>::RGBBacklightMatrixDeviceType::LIGHTING_COLS]:,
    [(); <K::Layout as KeyboardLayout>::RGBBacklightMatrixDeviceType::LIGHTING_ROWS]:,
{
    if K::VIALRGB_ENABLE {
        let num_leds = (<K::Layout as KeyboardLayout>::RGBBacklightMatrixDeviceType::LIGHTING_COLS
            * <K::Layout as KeyboardLayout>::RGBBacklightMatrixDeviceType::LIGHTING_ROWS)
            as u16;
        data[0..=1].copy_from_slice(&num_leds.to_le_bytes());
    }
}

#[cfg(feature = "rgb-backlight-matrix")]
pub fn vialrgb_get_led_info<K: VialKeyboard + 'static>(data: &mut [u8])
where
    [(); <K::Layout as KeyboardLayout>::RGBBacklightMatrixDeviceType::LIGHTING_COLS]:,
    [(); <K::Layout as KeyboardLayout>::RGBBacklightMatrixDeviceType::LIGHTING_ROWS]:,
{
    if K::VIALRGB_ENABLE {
        if let Some(backlight_matrix) = <<K::Layout as KeyboardLayout>::RGBBacklightMatrixDeviceType as crate::lighting::rgb_backlight_matrix::private::MaybeRGBBacklightMatrixDevice>::get_backlight_matrix() {
            let led = u16::from_le_bytes(data[0..=1].try_into().unwrap());
            let col = led as usize % <K::Layout as KeyboardLayout>::RGBBacklightMatrixDeviceType::LIGHTING_COLS;
            let row = (led as usize / <K::Layout as KeyboardLayout>::RGBBacklightMatrixDeviceType::LIGHTING_COLS)
                % <K::Layout as KeyboardLayout>::RGBBacklightMatrixDeviceType::LIGHTING_ROWS;
            if let Some((x, y)) = backlight_matrix.layout[row][col] {
                data[0] = x;
                data[1] = y;
                data[2] = backlight_matrix.flags[row][col].bits();
                data[3] = row as u8;
                data[4] = col as u8;
            }
        }
    }
}

#[cfg(feature = "rgb-backlight-matrix")]
pub async fn vialrgb_set_mode<K: VialKeyboard + 'static>(data: &[u8])
where
    [(); <K::Layout as KeyboardLayout>::RGBBacklightMatrixDeviceType::LIGHTING_COLS]:,
    [(); <K::Layout as KeyboardLayout>::RGBBacklightMatrixDeviceType::LIGHTING_ROWS]:,
{
    if K::VIALRGB_ENABLE {
        if let Some(channel) = <<K::Layout as KeyboardLayout>::RGBBacklightMatrixDeviceType as crate::lighting::rgb_backlight_matrix::private::MaybeRGBBacklightMatrixDevice>::get_command_channel() {
            // set mode
            let vialrgb_id = u16::from_le_bytes(data[0..=1].try_into().unwrap());
            if vialrgb_id == 0 {
                channel
                    .send(crate::lighting::rgb_backlight_matrix::RGBBacklightMatrixCommand::TurnOff)
                    .await;
            } else {
                channel
                    .send(crate::lighting::rgb_backlight_matrix::RGBBacklightMatrixCommand::TurnOn)
                    .await;
                if let Some(effect) =
                    super::protocol::vialrgb::convert_vialrgb_id_to_effect(vialrgb_id)
                {
                    channel
                    .send(crate::lighting::rgb_backlight_matrix::RGBBacklightMatrixCommand::SetEffect(
                        effect,
                    ))
                    .await;
                } else {
                    warn!(
                        "[VIA] Tried to set an unknown VialRGB effect: {:?}",
                        vialrgb_id
                    )
                }
            }

            // set speed
            channel
                .send(
                    crate::lighting::rgb_backlight_matrix::RGBBacklightMatrixCommand::SetSpeed(
                        data[2],
                    ),
                )
                .await;

            // set hsv
            channel
                .send(
                    crate::lighting::rgb_backlight_matrix::RGBBacklightMatrixCommand::SetHue(
                        data[3],
                    ),
                )
                .await;
            channel
                .send(
                    crate::lighting::rgb_backlight_matrix::RGBBacklightMatrixCommand::SetSaturation(
                        data[4],
                    ),
                )
                .await;
            channel
                .send(
                    crate::lighting::rgb_backlight_matrix::RGBBacklightMatrixCommand::SetValue(
                        data[5],
                    ),
                )
                .await;
        }
    }
}

#[cfg(feature = "rgb-backlight-matrix")]
pub async fn vialrgb_direct_fast_set<K: VialKeyboard + 'static>(data: &[u8])
where
    [(); <K::Layout as KeyboardLayout>::RGBBacklightMatrixDeviceType::LIGHTING_COLS]:,
    [(); <K::Layout as KeyboardLayout>::RGBBacklightMatrixDeviceType::LIGHTING_ROWS]:,
{
    if K::VIALRGB_ENABLE {
        let total_num_leds =
            (<K::Layout as KeyboardLayout>::RGBBacklightMatrixDeviceType::LIGHTING_COLS
                * <K::Layout as KeyboardLayout>::RGBBacklightMatrixDeviceType::LIGHTING_ROWS)
                as u8;

        let first_led = u16::from_le_bytes(data[0..=1].try_into().unwrap()) as u8; // We assume that a backlight matrix will not have more than 255 leds
        let num_leds = data[2];
        for led in first_led..(total_num_leds.min(first_led + num_leds)) {
            let hue = data[(3 + (led - first_led) * 3) as usize];
            let sat = data[(3 + (led - first_led) * 3 + 1) as usize];
            let val = data[(3 + (led - first_led) * 3 + 2) as usize];
            // TODO: use max brightness?
            VIAL_DIRECT_SET_CHANNEL
                .send((led, hsv2rgb(smart_leds::hsv::Hsv { hue, sat, val })))
                .await;
        }
    }
}

pub fn qmk_settings_query(data: &mut [u8]) {
    // TODO: maybe support some QMK settings
    data.fill(0xFF); // This indicates that we don't support any QMK settings
}

pub fn qmk_settings_get(data: &mut [u8]) {
    // TODO: maybe support some QMK settings
}

pub fn qmk_settings_set(data: &mut [u8]) {
    // TODO: maybe support some QMK settings
}

pub fn qmk_settings_reset(data: &mut [u8]) {
    // TODO: maybe support some QMK settings
}

pub fn dynamic_keymap_get_number_of_entries<K: VialKeyboard>(data: &mut [u8]) {
    data.fill(0);
    data[0] = K::VIAL_TAP_DANCE_ENTRIES;
    data[1] = K::VIAL_COMBO_ENTRIES;
    data[2] = K::VIAL_KEY_OVERRIDE_ENTRIES;
}

pub fn dynamic_keymap_get_tap_dance(data: &mut [u8]) {
    // TODO
}

pub fn dynamic_keymap_set_tap_dance(data: &mut [u8]) {
    // TODO
}

pub fn dynamic_keymap_get_combo(data: &mut [u8]) {
    // TODO
}

pub fn dynamic_keymap_set_combo(data: &mut [u8]) {
    // TODO
}

pub fn dynamic_keymap_get_key_override(data: &mut [u8]) {
    // TODO
}

pub fn dynamic_keymap_set_key_override(data: &mut [u8]) {
    // TODO
}

pub async fn eeprom_reset<K: VialKeyboard + 'static>()
where
    [(); <<K::StorageType as StorageDevice>::FlashStorageType as FlashStorage>::ERASE_SIZE]:,
{
    if let Some(database) = K::get_storage_service() {
        let _ = database.delete(StorageKey::DynamicKeymapTapDance).await;
        let _ = database.delete(StorageKey::DynamicKeymapCombo).await;
        let _ = database.delete(StorageKey::DynamicKeymapKeyOverride).await;
    }
}
