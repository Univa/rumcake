use super::ViaKeyboard;
use crate::keyboard::MATRIX_EVENTS;
use crate::storage::{FlashStorage, StorageDevice};
use crate::via::handlers::*;
use defmt::{info, warn, Debug2Format};
use embassy_futures::select;
use embassy_sync::mutex::Mutex;
use num_derive::FromPrimitive;

use crate::hw::platform::RawMutex;
use crate::keyboard::KeyboardLayout;

pub(crate) mod keycodes;

pub(crate) const VIA_PROTOCOL_VERSION: u16 = 0x000C;

#[derive(FromPrimitive, Debug, PartialEq, Eq)]
pub(crate) enum ViaCommandId {
    GetProtocolVersion = 0x01,
    GetKeyboardValue,
    SetKeyboardValue,
    DynamicKeymapGetKeycode,
    DynamicKeymapSetKeycode,
    DynamicKeymapReset,
    CustomSetValue,
    CustomGetValue,
    CustomSave,
    EEPROMReset,
    BootloaderJump,
    DynamicKeymapMacroGetCount,
    DynamicKeymapMacroGetBufferSize,
    DynamicKeymapMacroGetBuffer,
    DynamicKeymapMacroSetBuffer,
    DynamicKeymapMacroReset,
    DynamicKeymapGetLayerCount,
    DynamicKeymapGetBuffer,
    DynamicKeymapSetBuffer,
    DynamicKeymapGetEncoder,
    DynamicKeymapSetEncoder,
    Unhandled = 0xFF,
}

#[derive(FromPrimitive, Debug)]
enum ViaKeyboardValueId {
    Uptime = 0x01,
    LayoutOptions,
    SwitchMatrixState,
    FirmwareVersion,
    DeviceIndication,
}

// Note: Since Via V3 is capable of custom UI, we don't necessarily need to fully follow QMK's
// implementation. We don't implement unused channels (e.g. audio), and add separate commands for
// enabling/disabling lighting instead of setting an effect ID of 0 to disable, which is what QMK
// does.
#[derive(FromPrimitive, Debug)]
enum ViaChannelId {
    Custom = 0,
    Backlight,
    RGBLight, // underglow
    RGBMatrix,
    // Audio, // unused
    LEDMatrix = 5,
}

#[derive(FromPrimitive, Debug)]
enum ViaBacklightValue {
    Brightness = 1,
    Effect,
    EffectSpeed,
    Enabled,
}

#[derive(FromPrimitive, Debug)]
enum ViaRGBLightValue {
    Brightness = 1,
    Effect,
    EffectSpeed,
    Color,
    Enabled,
}

#[derive(FromPrimitive, Debug)]
enum ViaRGBMatrixValue {
    Brightness = 1,
    Effect,
    EffectSpeed,
    Color,
    Enabled,
}

#[derive(FromPrimitive, Debug)]
enum ViaLEDMatrixValue {
    Brightness = 1,
    Effect,
    EffectSpeed,
    Enabled,
}

pub(crate) struct ViaState<K: ViaKeyboard>
where
    [(); (K::Layout::LAYOUT_COLS + u8::BITS as usize - 1) / u8::BITS as usize
        * K::Layout::LAYOUT_ROWS]:,
{
    pub(crate) layout_state: [u8; (K::Layout::LAYOUT_COLS + u8::BITS as usize - 1)
        / u8::BITS as usize
        * K::Layout::LAYOUT_ROWS],
    pub(crate) layout_options: u32,
}

impl<K: ViaKeyboard> Default for ViaState<K>
where
    [(); (K::Layout::LAYOUT_COLS + u8::BITS as usize - 1) / u8::BITS as usize
        * K::Layout::LAYOUT_ROWS]:,
{
    fn default() -> Self {
        Self {
            layout_state: [0; (K::Layout::LAYOUT_COLS + u8::BITS as usize - 1) / u8::BITS as usize
                * K::Layout::LAYOUT_ROWS],
            layout_options: K::VIA_EEPROM_LAYOUT_OPTIONS_DEFAULT,
        }
    }
}

pub(crate) async fn process_via_command<K: ViaKeyboard + 'static>(
    data: &mut [u8],
    via_state: &mut ViaState<K>,
) where
    [(); <<K::StorageType as StorageDevice>::FlashStorageType as FlashStorage>::ERASE_SIZE]:,
    [(); K::DYNAMIC_KEYMAP_LAYER_COUNT * K::Layout::LAYOUT_COLS * K::Layout::LAYOUT_ROWS * 2]:,
    [(); K::DYNAMIC_KEYMAP_LAYER_COUNT * K::Layout::NUM_ENCODERS * 2 * 2]:,
    [(); (K::Layout::LAYOUT_COLS + u8::BITS as usize - 1) / u8::BITS as usize
        * K::Layout::LAYOUT_ROWS]:,
    [(); K::Layout::LAYERS]:,
    [(); K::Layout::LAYOUT_COLS]:,
    [(); K::Layout::LAYOUT_ROWS]:,
    [(); K::DYNAMIC_KEYMAP_MACRO_BUFFER_SIZE as usize]:,
    [(); K::DYNAMIC_KEYMAP_MACRO_COUNT as usize]:,
{
    if K::handle_via_command(data) {
        return;
    }

    info!("[VIA] Processing Via command");
    if let Some(command) = num::FromPrimitive::from_u8(data[0]) {
        info!("[VIA] Received command {:?}", Debug2Format(&command));

        match command {
            ViaCommandId::GetProtocolVersion => {
                get_protocol_version(VIA_PROTOCOL_VERSION, &mut data[1..=2])
            }
            ViaCommandId::GetKeyboardValue => match num::FromPrimitive::from_u8(data[1]) {
                Some(ViaKeyboardValueId::Uptime) => get_uptime(&mut data[2..=5]),
                Some(ViaKeyboardValueId::LayoutOptions) => {
                    get_layout_options::<K>(&via_state.layout_options, &mut data[2..=5]).await
                }
                Some(ViaKeyboardValueId::SwitchMatrixState) => {
                    get_switch_matrix_state::<K>(&via_state.layout_state, &mut data[2..]).await
                }
                Some(ViaKeyboardValueId::FirmwareVersion) => {
                    get_firmware_version::<K>(&mut data[2..=5])
                }
                Some(value) => {
                    data[0] = ViaCommandId::Unhandled as u8;
                    warn!(
                        "[VIA] Unimplemented get keyboard value subcommand received from host {:?}",
                        Debug2Format(&value)
                    );
                }
                None => {
                    data[0] = ViaCommandId::Unhandled as u8;
                    warn!(
                        "[VIA] Unknown get keyboard value subcommand received from host {:?}",
                        Debug2Format(&command)
                    );
                }
            },
            ViaCommandId::SetKeyboardValue => {
                match num::FromPrimitive::from_u8(data[1]) {
                    Some(ViaKeyboardValueId::LayoutOptions) => {
                        set_layout_options::<K>(&mut via_state.layout_options, &data[2..=5]).await
                    }
                    Some(ViaKeyboardValueId::DeviceIndication) => device_indication::<K>().await,
                    Some(value) => {
                        data[0] = ViaCommandId::Unhandled as u8;
                        warn!(
                            "[VIA] Unimplemented set keyboard value subcommand received from host {:?}",
                            Debug2Format(&value)
                        );
                    }
                    None => {
                        data[0] = ViaCommandId::Unhandled as u8;
                        warn!(
                            "[VIA] Unknown set keyboard value subcommand received from host {:?}",
                            Debug2Format(&command)
                        );
                    }
                };
            }
            ViaCommandId::EEPROMReset => eeprom_reset().await,
            ViaCommandId::BootloaderJump => bootloader_jump(),
            ViaCommandId::DynamicKeymapMacroGetCount => {
                dynamic_keymap_macro_get_count::<K>(&mut data[1..=1])
            }
            ViaCommandId::DynamicKeymapMacroGetBufferSize => {
                dynamic_keymap_macro_get_buffer_size::<K>(&mut data[1..=2])
            }
            ViaCommandId::DynamicKeymapMacroGetBuffer => {
                let offset = u16::from_be_bytes(data[1..=2].try_into().unwrap());
                let size = data[3];
                dynamic_keymap_macro_get_buffer::<K>(offset, size, &mut data[4..]).await
            }
            ViaCommandId::DynamicKeymapMacroSetBuffer => {
                let offset = u16::from_be_bytes(data[1..=2].try_into().unwrap());
                let size = data[3];
                dynamic_keymap_macro_set_buffer::<K>(offset, size, &data[4..]).await
            }
            ViaCommandId::DynamicKeymapMacroReset => dynamic_keymap_macro_reset::<K>(),
            ViaCommandId::DynamicKeymapGetLayerCount => {
                dynamic_keymap_get_layer_count::<K>(&mut data[1..=1])
            }
            ViaCommandId::DynamicKeymapGetKeycode => {
                let layer = data[1];
                let row = data[2];
                let col = data[3];
                dynamic_keymap_get_keycode::<K>(
                    layer,
                    row,
                    col,
                    &mut data[4..=5],
                    keycodes::convert_action_to_keycode::<K>,
                )
                .await
            }
            ViaCommandId::DynamicKeymapSetKeycode => {
                let layer = data[1];
                let row = data[2];
                let col = data[3];
                dynamic_keymap_set_keycode::<K>(
                    layer,
                    row,
                    col,
                    &data[4..=5],
                    keycodes::convert_keycode_to_action::<K>,
                )
                .await
            }
            ViaCommandId::DynamicKeymapGetEncoder => {
                let layer = data[1];
                let encoder_id = data[2];
                let clockwise = data[3] != 0;
                dynamic_keymap_get_encoder::<K>(layer, encoder_id, clockwise, &mut data[4..=5])
                    .await
            } // only if encoder map is enabled
            ViaCommandId::DynamicKeymapSetEncoder => {
                let layer = data[1];
                let encoder_id = data[2];
                let clockwise = data[3] != 0;
                dynamic_keymap_set_encoder::<K>(layer, encoder_id, clockwise, &data[4..=5]).await
            } // only if encoder map is enabled
            ViaCommandId::DynamicKeymapGetBuffer => {
                let offset = u16::from_be_bytes(data[1..=2].try_into().unwrap());
                let size = data[3];
                dynamic_keymap_get_buffer::<K>(
                    offset,
                    size,
                    &mut data[4..],
                    keycodes::convert_action_to_keycode::<K>,
                )
                .await
            }
            ViaCommandId::DynamicKeymapSetBuffer => {
                let offset = u16::from_be_bytes(data[1..=2].try_into().unwrap());
                let size = data[3];
                dynamic_keymap_set_buffer::<K>(
                    offset,
                    size,
                    &data[4..],
                    keycodes::convert_keycode_to_action::<K>,
                )
                .await
            }
            ViaCommandId::DynamicKeymapReset => dynamic_keymap_reset().await,
            command
                if command == ViaCommandId::CustomGetValue
                    || command == ViaCommandId::CustomSetValue
                    || command == ViaCommandId::CustomSave =>
            {
                match num::FromPrimitive::from_u8(data[1]) as Option<ViaChannelId> {
                    #[cfg(feature = "simple-backlight")]
                    Some(ViaChannelId::Backlight) => {
                        match command {
                            ViaCommandId::CustomGetValue => {
                                match num::FromPrimitive::from_u8(data[2]) {
                                    Some(ViaBacklightValue::Brightness) => {
                                        simple_backlight_get_brightness::<K>(&mut data[3..=3]).await
                                    }
                                    Some(ViaBacklightValue::Effect) => {
                                        simple_backlight_get_effect::<K>(
                                            &mut data[3..=3],
                                            |effect| {
                                                // Just directly convert to an ID. We assume that custom UI is being used.
                                                effect as u8
                                            },
                                        )
                                        .await
                                    }
                                    Some(ViaBacklightValue::EffectSpeed) => {
                                        simple_backlight_get_speed::<K>(&mut data[3..=3]).await
                                    }
                                    Some(ViaBacklightValue::Enabled) => {
                                        simple_backlight_get_enabled::<K>(&mut data[3..=3]).await
                                    }
                                    None => {
                                        warn!(
                                            "[VIA] Unknown backlight get command received from host {:?}",
                                            data[2]
                                        )
                                    }
                                };
                            }
                            ViaCommandId::CustomSetValue => {
                                match num::FromPrimitive::from_u8(data[2]) {
                                    Some(ViaBacklightValue::Brightness) => {
                                        simple_backlight_set_brightness::<K>(&data[3..=3]).await
                                    }
                                    Some(ViaBacklightValue::Effect) => {
                                        simple_backlight_set_effect::<K>(&data[3..=3], |id| {
                                            // Just directly convert to an effect from the ID. We assume that custom UI is being used.
                                            num::FromPrimitive::from_u8(id)
                                        })
                                        .await
                                    }
                                    Some(ViaBacklightValue::EffectSpeed) => {
                                        simple_backlight_set_speed::<K>(&data[3..=3]).await
                                    }
                                    Some(ViaBacklightValue::Enabled) => {
                                        simple_backlight_set_enabled::<K>(&data[3..=3]).await
                                    }
                                    None => {
                                        warn!(
                                            "[VIA] Unknown backlight set command received from host {:?}",
                                            data[2]
                                        )
                                    }
                                };
                            }
                            ViaCommandId::CustomSave => simple_backlight_save::<K>().await,
                            _ => unreachable!("Should not happen"),
                        };
                    }
                    #[cfg(feature = "simple-backlight-matrix")]
                    Some(ViaChannelId::LEDMatrix) => {
                        match command {
                            ViaCommandId::CustomGetValue => {
                                match num::FromPrimitive::from_u8(data[2]) {
                                    Some(ViaLEDMatrixValue::Brightness) => {
                                        simple_backlight_matrix_get_brightness::<K>(
                                            &mut data[3..=3],
                                        )
                                        .await
                                    }
                                    Some(ViaLEDMatrixValue::Effect) => {
                                        simple_backlight_matrix_get_effect::<K>(
                                            &mut data[3..=3],
                                            |effect| {
                                                // Just directly convert to an ID. We assume that custom UI is being used.
                                                effect as u8
                                            },
                                        )
                                        .await
                                    }
                                    Some(ViaLEDMatrixValue::EffectSpeed) => {
                                        simple_backlight_matrix_get_speed::<K>(&mut data[3..=3])
                                            .await
                                    }
                                    Some(ViaLEDMatrixValue::Enabled) => {
                                        simple_backlight_matrix_get_enabled::<K>(&mut data[3..=3])
                                            .await
                                    }
                                    None => {
                                        warn!(
                                            "[VIA] Unknown LED matrix get command received from host {:?}",
                                            data[2]
                                        )
                                    }
                                };
                            }
                            ViaCommandId::CustomSetValue => {
                                match num::FromPrimitive::from_u8(data[2]) {
                                    Some(ViaLEDMatrixValue::Brightness) => {
                                        simple_backlight_matrix_set_brightness::<K>(&data[3..=3])
                                            .await
                                    }
                                    Some(ViaLEDMatrixValue::Effect) => {
                                        simple_backlight_matrix_set_effect::<K>(
                                            &data[3..=3],
                                            |id| {
                                                // Just directly convert to an effect from the ID. We assume that custom UI is being used.
                                                num::FromPrimitive::from_u8(id)
                                            },
                                        )
                                        .await
                                    }
                                    Some(ViaLEDMatrixValue::EffectSpeed) => {
                                        simple_backlight_matrix_set_speed::<K>(&data[3..=3]).await
                                    }
                                    Some(ViaLEDMatrixValue::Enabled) => {
                                        simple_backlight_matrix_set_enabled::<K>(&data[3..=3]).await
                                    }
                                    None => {
                                        warn!(
                                            "[VIA] Unknown LED matrix set command received from host {:?}",
                                            data[2]
                                        )
                                    }
                                };
                            }
                            ViaCommandId::CustomSave => simple_backlight_matrix_save::<K>().await,
                            _ => unreachable!("Should not happen"),
                        };
                    }
                    #[cfg(feature = "rgb-backlight-matrix")]
                    Some(ViaChannelId::RGBMatrix) => {
                        match command {
                            ViaCommandId::CustomGetValue => {
                                match num::FromPrimitive::from_u8(data[2]) {
                                    Some(ViaRGBMatrixValue::Brightness) => {
                                        rgb_backlight_matrix_get_brightness::<K>(&mut data[3..=3])
                                            .await
                                    }
                                    Some(ViaRGBMatrixValue::Effect) => {
                                        rgb_backlight_matrix_get_effect::<K>(
                                            &mut data[3..=3],
                                            |effect| {
                                                // Just directly convert to an ID. We assume that custom UI is being used.
                                                effect as u8
                                            },
                                        )
                                        .await
                                    }
                                    Some(ViaRGBMatrixValue::EffectSpeed) => {
                                        rgb_backlight_matrix_get_speed::<K>(&mut data[3..=3]).await
                                    }
                                    Some(ViaRGBMatrixValue::Color) => {
                                        rgb_backlight_matrix_get_color::<K>(&mut data[3..=4]).await
                                    }
                                    Some(ViaRGBMatrixValue::Enabled) => {
                                        rgb_backlight_matrix_get_enabled::<K>(&mut data[3..=3])
                                            .await
                                    }
                                    None => {
                                        warn!(
                                            "[VIA] Unknown RGB matrix get command received from host {:?}",
                                            data[2]
                                        )
                                    }
                                };
                            }
                            ViaCommandId::CustomSetValue => {
                                match num::FromPrimitive::from_u8(data[2]) {
                                    Some(ViaRGBMatrixValue::Brightness) => {
                                        rgb_backlight_matrix_set_brightness::<K>(&data[3..=3]).await
                                    }
                                    Some(ViaRGBMatrixValue::Effect) => {
                                        rgb_backlight_matrix_set_effect::<K>(&data[3..=3], |id| {
                                            // Just directly convert to an effect from the ID. We assume that custom UI is being used.
                                            num::FromPrimitive::from_u8(id)
                                        })
                                        .await
                                    }
                                    Some(ViaRGBMatrixValue::EffectSpeed) => {
                                        rgb_backlight_matrix_set_speed::<K>(&data[3..=3]).await
                                    }
                                    Some(ViaRGBMatrixValue::Color) => {
                                        rgb_backlight_matrix_set_color::<K>(&data[3..=4]).await
                                    }
                                    Some(ViaRGBMatrixValue::Enabled) => {
                                        rgb_backlight_matrix_set_enabled::<K>(&data[3..=3]).await
                                    }
                                    None => {
                                        warn!(
                                            "[VIA] Unknown RGB matrix get command received from host {:?}",
                                            data[2]
                                        )
                                    }
                                };
                            }
                            ViaCommandId::CustomSave => rgb_backlight_matrix_save::<K>().await,
                            _ => unreachable!("Should not happen"),
                        };
                    }
                    #[cfg(feature = "underglow")]
                    Some(ViaChannelId::RGBLight) => {
                        match command {
                            ViaCommandId::CustomGetValue => {
                                match num::FromPrimitive::from_u8(data[2]) {
                                    Some(ViaRGBLightValue::Brightness) => {
                                        underglow_get_brightness::<K>(&mut data[3..=3]).await
                                    }
                                    Some(ViaRGBLightValue::Effect) => {
                                        underglow_get_effect::<K>(&mut data[3..=3], |config| {
                                            // Just directly convert to an ID. We assume that custom UI is being used.
                                            config.effect as u8
                                        })
                                        .await
                                    }
                                    Some(ViaRGBLightValue::EffectSpeed) => {
                                        underglow_get_speed::<K>(&mut data[3..=3]).await;
                                    }
                                    Some(ViaRGBLightValue::Color) => {
                                        underglow_get_color::<K>(&mut data[3..=4]).await;
                                    }
                                    Some(ViaRGBLightValue::Enabled) => {
                                        underglow_get_enabled::<K>(&mut data[3..=3]).await;
                                    }
                                    None => {
                                        warn!(
                                            "[VIA] Unknown RGB underglow get command received from host {:?}",
                                            data[2]
                                        )
                                    }
                                };
                            }
                            ViaCommandId::CustomSetValue => {
                                match num::FromPrimitive::from_u8(data[2]) {
                                    Some(ViaRGBLightValue::Brightness) => {
                                        underglow_set_brightness::<K>(&data[3..=3]).await
                                    }
                                    Some(ViaRGBLightValue::Effect) => {
                                        underglow_set_effect::<K>(&data[3..=3], |id| {
                                            // Just directly convert to an effect from the ID. We assume that custom UI is being used.
                                            (num::FromPrimitive::from_u8(id)
                                                as Option<
                                                    crate::lighting::underglow::UnderglowEffect,
                                                >)
                                                .map(|effect| (effect, None))
                                        })
                                        .await
                                    }
                                    Some(ViaRGBLightValue::EffectSpeed) => {
                                        underglow_set_speed::<K>(&data[3..=3]).await
                                    }
                                    Some(ViaRGBLightValue::Color) => {
                                        underglow_set_color::<K>(&data[3..=4]).await
                                    }
                                    Some(ViaRGBLightValue::Enabled) => {
                                        underglow_set_enabled::<K>(&data[3..=3]).await;
                                    }
                                    None => {
                                        warn!(
                                            "[VIA] Unknown RGB underglow get command received from host {:?}",
                                            data[2]
                                        )
                                    }
                                };
                            }
                            ViaCommandId::CustomSave => underglow_save::<K>().await,
                            _ => unreachable!("Should not happen"),
                        };
                    }
                    other => {
                        match other {
                            Some(channel) => {
                                warn!(
                                    "[VIA] Unimplemented channel ID received from host: {:?}",
                                    Debug2Format(&channel)
                                );
                            }
                            None => {
                                warn!(
                                    "[VIA] Unknown channel ID received from host, handle_custom_value_command called: {:?}",
                                    Debug2Format(&data[1])
                                )
                            }
                        }

                        K::handle_custom_value_command(data, 32);
                    }
                };
            }
            _ => {
                data[0] = ViaCommandId::Unhandled as u8;
                warn!(
                    "[VIA] Unimplemented command received from host {:?}",
                    Debug2Format(&command)
                );
            }
        }
    } else {
        warn!("[VIA] Unknown command received from host {:?}", data[0]);
    }
}

pub(crate) async fn background_task<K: ViaKeyboard + 'static>(
    via_state: &Mutex<RawMutex, ViaState<K>>,
) where
    [(); <<K::StorageType as StorageDevice>::FlashStorageType as FlashStorage>::ERASE_SIZE]:,
    [(); (K::Layout::LAYOUT_COLS + u8::BITS as usize - 1) / u8::BITS as usize
        * K::Layout::LAYOUT_ROWS]:,
{
    // Update the layout_state. Used for SwitchMatrixState
    let mut subscriber = MATRIX_EVENTS.subscriber().unwrap();

    if K::get_storage_service().is_some() {
        via_state.lock().await.layout_options = super::VIA_LAYOUT_OPTIONS.wait().await;
    }

    loop {
        match select::select(
            subscriber.next_message_pure(),
            BOOTLOADER_JUMP_SIGNAL.wait(),
        )
        .await
        {
            select::Either::First(event) => {
                let (row, col) = event.coord();
                // (cols + 8 bits - 1) / 8 bits: we get the number of bytes needed to store the state of a
                // row (based on number of cols). multiply this by (row + 1), subtract by 1 and subtract by
                // (col / 8 bits) to get the byte that contains the bit we need to update.
                let byte = (K::Layout::LAYOUT_COLS + u8::BITS as usize - 1) / u8::BITS as usize
                    * (row as usize + 1)
                    - 1
                    - col as usize / u8::BITS as usize;

                if event.is_press() {
                    via_state.lock().await.layout_state[byte] |=
                        1 << (col as usize % u8::BITS as usize);
                } else if event.is_release() {
                    via_state.lock().await.layout_state[byte] &=
                        !(1 << (col as usize % u8::BITS as usize));
                };
            }
            select::Either::Second(()) => {
                // Wait for 500 ms. This should give enough time to send an HID report and let the host read it
                embassy_time::Timer::after(embassy_time::Duration::from_millis(500)).await;
                crate::hw::platform::jump_to_bootloader();
            }
        }
    }
}
