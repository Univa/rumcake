use crate::backlight::BacklightMatrixDevice;
use crate::keyboard::MATRIX_EVENTS;
use crate::via::handlers::eeprom_reset as via_eeprom_reset;
use crate::via::handlers::*;
use crate::vial::handlers::eeprom_reset as vial_eeprom_reset;
use crate::vial::handlers::*;
use crate::vial::protocol::{lighting, VIALRGB_PROTOCOL_VERSION};
use crate::vial::VialKeyboard;
use defmt::{info, warn, Debug2Format};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use num_derive::FromPrimitive;

use super::VialState;
use crate::via::protocol::keycodes; // We just use the keycode conversions from the new via protocol

pub(crate) const VIA_PROTOCOL_VERSION: u16 = 0x0009;

#[derive(FromPrimitive, Debug)]
pub(crate) enum ViaCommandId {
    GetProtocolVersion = 0x01,
    GetKeyboardValue,
    SetKeyboardValue,
    DynamicKeymapGetKeycode,
    DynamicKeymapSetKeycode,
    DynamicKeymapReset,
    CustomSetValue, // Same as id_lighting_set_value
    CustomGetValue, // Same as id_lighting_get_value
    CustomSave,     // Same as id_lighting_save
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
    VialPrefix = 0xFE,
    Unhandled = 0xFF,
}

#[derive(FromPrimitive, Debug)]
enum ViaKeyboardValueId {
    Uptime = 0x01,
    LayoutOptions,
    SwitchMatrixState,
    FirmwareVersion,  // Unused
    DeviceIndication, // Unused
}

// Unused
#[derive(FromPrimitive, Debug)]
enum ViaChannelId {
    Custom = 0,
    Backlight,
    RGBLight, // underglow
    RGBMatrix,
    Audio,
    LEDMatrix,
}

#[derive(FromPrimitive, Debug)]
enum ViaLightingValue {
    // Backlight
    BacklightBrightness = 0x09,
    BacklightEffect,

    // VialRGB
    Info = 0x40,
    Mode,
    SupportedDirectFastSet,
    NumberLEDs,
    LEDInfo,

    // Underglow
    // Note: even though QMK code re-uses these IDs for handling RGB matrix (non-VialRGB, or
    // `qmk_rgb_matrix`), we choose not to handle it, and instead only handle underglow. The Vial
    // app handles the RGB matrix using the VialRGB protocol IDs, support for which is indicated as
    // `"lighting": "vialrgb"` in a keyboard definition JSON. The code that handles RGB matrix in
    // QMK was originally a hacky workaround:
    // https://github.com/qmk/qmk_firmware/commit/9056775e2050cc95abe888b93135dfdc8ef86609. This
    // commit was at protocol version 9 / Via V2, but V2 definitions did not seem to support it:
    // https://github.com/the-via/reader/blob/9dccbcbc2bbb26fface75def628f1254afbbc1f0/src/types.v2.ts#L41
    // Note the lack of `qmk_rgb_matrix`. Although there is `wt_rgb_backlight`, it does not use the
    // rgblight lighting value IDs (0x80 and above), and instead uses backlight lighting value IDs,
    // most of which are not implemented in QMK by default (0x01 to 0x17):
    // https://github.com/the-via/reader/blob/9dccbcbc2bbb26fface75def628f1254afbbc1f0/src/types.v2.ts#L12
    // https://github.com/the-via/reader/blob/9dccbcbc2bbb26fface75def628f1254afbbc1f0/src/lighting-presets.ts#L164
    // Official support for `qmk_rgb_matrix` in the Via app didn't seem to come until protocol
    // version 11 / Via V3, where you can use `"menus": ["qmk_rgb_matrix"]` in a V3 definition:
    // https://github.com/qmk/qmk_firmware/commit/bc6f8dc8b0822e5e03893eacffa42a7badb4c2fa
    // https://github.com/the-via/reader/blob/9dccbcbc2bbb26fface75def628f1254afbbc1f0/src/common-menus/qmk_rgb_matrix.ts
    RGBLightBrightness = 0x80,
    RGBLightEffect,
    RGBLightEffectSpeed,
    RGBLightColor,
}

// Unused
#[derive(FromPrimitive, Debug)]
enum ViaBacklightValue {
    Brightness = 0x09,
    Effect,
}

// Unused
#[derive(FromPrimitive, Debug)]
enum ViaRGBLightValue {
    Brightness = 0x80,
    Effect,
    EffectSpeed,
    Color,
}

// Unused
#[derive(FromPrimitive, Debug)]
enum ViaRGBMatrixValue {
    Brightness = 1,
    Effect,
    EffectSpeed,
    Color,
}

// Unused
#[derive(FromPrimitive, Debug)]
enum ViaLEDMatrixValue {
    Brightness = 1,
    Effect,
    EffectSpeed,
}

// Unused
#[derive(FromPrimitive, Debug)]
enum ViaAudioValue {
    Enable = 1,
    ClickyEnable,
}

pub(crate) struct ViaState<K: VialKeyboard>
where
    [(); (K::LAYOUT_COLS + u8::BITS as usize - 1) / u8::BITS as usize * K::LAYOUT_ROWS]:,
{
    pub(crate) layout_state:
        [u8; (K::LAYOUT_COLS + u8::BITS as usize - 1) / u8::BITS as usize * K::LAYOUT_ROWS],
    pub(crate) layout_options: u32,
}

impl<K: VialKeyboard> Default for ViaState<K>
where
    [(); (K::LAYOUT_COLS + u8::BITS as usize - 1) / u8::BITS as usize * K::LAYOUT_ROWS]:,
{
    fn default() -> Self {
        Self {
            layout_state: [0; (K::LAYOUT_COLS + u8::BITS as usize - 1) / u8::BITS as usize
                * K::LAYOUT_ROWS],
            layout_options: K::VIA_EEPROM_LAYOUT_OPTIONS_DEFAULT,
        }
    }
}

pub(crate) async fn process_via_command<K: VialKeyboard + 'static>(
    data: &mut [u8],
    via_state: &mut ViaState<K>,
    vial_state: &mut VialState,
) where
    [(); K::BacklightMatrixDevice::LIGHTING_COLS]:,
    [(); K::BacklightMatrixDevice::LIGHTING_ROWS]:,
    [(); (K::LAYOUT_COLS + u8::BITS as usize - 1) / u8::BITS as usize * K::LAYOUT_ROWS]:,
    [(); K::LAYERS]:,
    [(); K::LAYOUT_ROWS]:,
    [(); K::LAYOUT_COLS]:,
{
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
                    if vial_state.unlocked {
                        get_switch_matrix_state::<K>(&via_state.layout_state, &mut data[2..]).await
                    }
                }
                Some(value) => {
                    data[0] = ViaCommandId::Unhandled as u8;
                    warn!(
                        "[VIA] Unimplemented get keyboard value subcommand received from host {:?}, calling user function",
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
                        set_layout_options::<K>(&mut via_state.layout_options, &mut data[2..=5])
                            .await
                    }
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
            ViaCommandId::EEPROMReset => {
                via_eeprom_reset().await;
                vial_eeprom_reset().await;
            }
            ViaCommandId::BootloaderJump => {
                if vial_state.unlocked {
                    bootloader_jump()
                }
            }
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
                if vial_state.unlocked {
                    let offset = u16::from_be_bytes(data[1..=2].try_into().unwrap());
                    let size = data[3];
                    dynamic_keymap_macro_set_buffer::<K>(offset, size, &mut data[4..]).await
                }
            }
            ViaCommandId::DynamicKeymapMacroReset => todo!(),
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
                    keycodes::convert_action_to_keycode,
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
                    &mut data[4..=5],
                    keycodes::convert_keycode_to_action,
                )
                .await
            }
            ViaCommandId::DynamicKeymapGetBuffer => {
                let offset = u16::from_be_bytes(data[1..=2].try_into().unwrap());
                let size = data[3];
                dynamic_keymap_get_buffer::<K>(
                    offset,
                    size,
                    &mut data[4..],
                    keycodes::convert_action_to_keycode,
                )
                .await
            }
            ViaCommandId::DynamicKeymapSetBuffer => {
                // TODO: remove instances of QK_BOOT
                let offset = u16::from_be_bytes(data[1..=2].try_into().unwrap());
                let size = data[3];
                dynamic_keymap_set_buffer::<K>(
                    offset,
                    size,
                    &mut data[4..],
                    keycodes::convert_keycode_to_action,
                )
                .await
            }
            ViaCommandId::DynamicKeymapReset => dynamic_keymap_reset().await,
            ViaCommandId::CustomSetValue => {
                match num::FromPrimitive::from_u8(data[1]) as Option<ViaLightingValue> {
                    #[cfg(feature = "simple-backlight")]
                    Some(ViaLightingValue::BacklightBrightness) => {
                        backlight_set_brightness(&mut data[2..=2]).await
                    }
                    #[cfg(feature = "simple-backlight")]
                    Some(ViaLightingValue::BacklightEffect) => {
                        backlight_set_effect(&mut data[2..=2], |id| {
                            lighting::convert_qmk_id_to_backlight_effect(id)
                        })
                        .await;
                    }
                    #[cfg(feature = "underglow")]
                    Some(ViaLightingValue::RGBLightBrightness) => {
                        underglow_set_brightness(&mut data[2..=2]).await
                    }
                    #[cfg(feature = "underglow")]
                    Some(ViaLightingValue::RGBLightEffect) => {
                        if data[2] == 0 {
                            crate::underglow::UNDERGLOW_COMMAND_CHANNEL
                                .send(crate::underglow::animations::UnderglowCommand::TurnOff)
                                .await;
                        } else {
                            crate::underglow::UNDERGLOW_COMMAND_CHANNEL
                                .send(crate::underglow::animations::UnderglowCommand::TurnOn)
                                .await;
                            underglow_set_effect(&mut data[2..=2], |id| {
                                lighting::convert_qmk_id_to_underglow_effect(id)
                            })
                            .await
                        }
                    }
                    #[cfg(feature = "underglow")]
                    Some(ViaLightingValue::RGBLightEffectSpeed) => {
                        underglow_set_speed(&mut data[2..=2]).await
                    }
                    #[cfg(feature = "underglow")]
                    Some(ViaLightingValue::RGBLightColor) => {
                        underglow_set_color(&mut data[2..=3]).await
                    }
                    #[cfg(feature = "rgb-backlight-matrix")]
                    Some(ViaLightingValue::Mode) => vialrgb_set_mode::<K>(&mut data[2..=7]).await,
                    #[cfg(feature = "rgb-backlight-matrix")]
                    Some(ViaLightingValue::SupportedDirectFastSet) => {
                        vialrgb_direct_fast_set::<K>(&mut data[2..]).await
                    }
                    other => {
                        data[0] = ViaCommandId::Unhandled as u8;
                        if other.is_none() {
                            warn!(
                                "[VIA] Unknown get lighting value subcommand received from host {:?}",
                                Debug2Format(&command)
                            );
                        }
                    }
                };
            }
            ViaCommandId::CustomGetValue => {
                match num::FromPrimitive::from_u8(data[1]) as Option<ViaLightingValue> {
                    #[cfg(feature = "simple-backlight")]
                    Some(ViaLightingValue::BacklightBrightness) => {
                        backlight_get_brightness(&mut data[2..=2]).await
                    }
                    #[cfg(feature = "simple-backlight")]
                    Some(ViaLightingValue::BacklightEffect) => {
                        backlight_get_effect(&mut data[2..=2], |effect| {
                            lighting::convert_backlight_effect_to_qmk_id(effect)
                        })
                        .await
                    }
                    #[cfg(feature = "underglow")]
                    Some(ViaLightingValue::RGBLightBrightness) => {
                        underglow_get_brightness(&mut data[2..=2]).await
                    }
                    #[cfg(feature = "underglow")]
                    Some(ViaLightingValue::RGBLightEffect) => {
                        if !crate::underglow::UNDERGLOW_CONFIG_STATE.get().await.enabled {
                            data[2] = 0
                        } else {
                            underglow_get_effect(&mut data[2..=2], |config| {
                                lighting::convert_underglow_effect_to_qmk_id(config)
                            })
                            .await
                        }
                    }
                    #[cfg(feature = "underglow")]
                    Some(ViaLightingValue::RGBLightEffectSpeed) => {
                        underglow_get_speed(&mut data[2..=2]).await
                    }
                    #[cfg(feature = "underglow")]
                    Some(ViaLightingValue::RGBLightColor) => {
                        underglow_get_color(&mut data[2..=3]).await
                    }
                    #[cfg(feature = "rgb-backlight-matrix")]
                    Some(ViaLightingValue::Info) => {
                        vialrgb_get_info(VIALRGB_PROTOCOL_VERSION, &mut data[2..=4])
                    }
                    #[cfg(feature = "rgb-backlight-matrix")]
                    Some(ViaLightingValue::Mode) => vialrgb_get_mode::<K>(&mut data[2..=7]).await,
                    #[cfg(feature = "rgb-backlight-matrix")]
                    Some(ViaLightingValue::SupportedDirectFastSet) => {
                        vialrgb_get_supported::<K>(&mut data[2..])
                    }
                    #[cfg(feature = "rgb-backlight-matrix")]
                    Some(ViaLightingValue::NumberLEDs) => {
                        vialrgb_get_num_leds::<K>(&mut data[2..=3])
                    }
                    #[cfg(feature = "rgb-backlight-matrix")]
                    Some(ViaLightingValue::LEDInfo) => vialrgb_get_led_info::<K>(&mut data[2..=6]),
                    other => {
                        data[0] = ViaCommandId::Unhandled as u8;
                        if other.is_none() {
                            warn!(
                                "[VIA] Unknown get lighting value subcommand received from host {:?}",
                                Debug2Format(&command)
                            );
                        }
                    }
                };
            }
            ViaCommandId::CustomSave => {
                backlight_save().await; // This also handles vialrgb_save
                underglow_save().await;
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

pub(crate) async fn background_task<K: VialKeyboard>(
    via_state: &Mutex<ThreadModeRawMutex, ViaState<K>>,
) where
    [(); (K::LAYOUT_COLS + u8::BITS as usize - 1) / u8::BITS as usize * K::LAYOUT_ROWS]:,
{
    // Update the layout_state. Used for SwitchMatrixState
    let mut subscriber = MATRIX_EVENTS.subscriber().unwrap();

    loop {
        let event = subscriber.next_message_pure().await;
        let (row, col) = event.coord();
        // (cols + 8 bits - 1) / 8 bits: we get the number of bytes needed to store the state of a
        // row (based on number of cols). multiply this by (row + 1), subtract by 1 and subtract by
        // (col / 8 bits) to get the byte that contains the bit we need to update.
        let byte = (K::LAYOUT_COLS + u8::BITS as usize - 1) / u8::BITS as usize
            * (row as usize + 1)
            - 1
            - col as usize / u8::BITS as usize;

        if event.is_press() {
            via_state.lock().await.layout_state[byte] |= 1 << (col as usize % u8::BITS as usize);
        } else if event.is_release() {
            via_state.lock().await.layout_state[byte] &= !(1 << (col as usize % u8::BITS as usize));
        };
    }
}
