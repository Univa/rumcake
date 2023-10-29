use crate::storage::KeyboardWithEEPROM;
use crate::usb::USBKeyboard;
use defmt::{debug, error, warn, Debug2Format};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_usb::class::hid::{
    Config, HidReader, HidReaderWriter, HidWriter, ReportId, RequestHandler, State,
};
use embassy_usb::control::OutResponse;
use embassy_usb::driver::Driver;
use embassy_usb::Builder;
use embedded_storage::nor_flash::NorFlash;
use keyberon::debounce::Debouncer;
use num_derive::FromPrimitive;
use static_cell::StaticCell;

pub trait ViaKeyboard: USBKeyboard + KeyboardWithEEPROM {
    const VIA_ENABLED: bool = true;
    const VIA_FIRMWARE_VERSION: u32 = 1; // This is the default if not set in QMK.

    // Note: QMK uses an algorithm to emulate EEPROM in STM32 chips by using their flash peripherals
    const EEPROM_TOTAL_BYTE_COUNT: usize = if Self::VIA_ENABLED {
        1024
    } else {
        Self::EECONFIG_SIZE + 3
    }; // This is the default if not set in QMK (for STM32L0/L1)

    const VIA_EEPROM_MAGIC_ADDR: u8 = Self::EECONFIG_SIZE as u8; // This is the default if not set in QMK

    const VIA_EEPROM_LAYOUT_OPTIONS_ADDR: u8 = Self::VIA_EEPROM_MAGIC_ADDR + 3;
    const VIA_EEPROM_LAYOUT_OPTIONS_SIZE: usize = 1; // This is the default if not set in QMK
    const VIA_EEPROM_LAYOUT_OPTIONS_DEFAULT: u32 = 0x00000000; // This is the default if not set in QMK

    const VIA_EEPROM_CUSTOM_CONFIG_ADDR: u8 =
        Self::VIA_EEPROM_LAYOUT_OPTIONS_ADDR + Self::VIA_EEPROM_LAYOUT_OPTIONS_SIZE as u8;
    const VIA_EEPROM_CUSTOM_CONFIG_SIZE: usize = 0; // This is the default if not set in QMK
    const VIA_EEPROM_CONFIG_END: u8 =
        Self::VIA_EEPROM_CUSTOM_CONFIG_ADDR + Self::VIA_EEPROM_CUSTOM_CONFIG_SIZE as u8;

    const DYNAMIC_KEYMAP_LAYER_COUNT: u8 = 4; // This is the default if this variable isn't defined in QMK
    const DYNAMIC_KEYMAP_EEPROM_MAX_ADDR: u16 =
        (<Self as ViaKeyboard>::EEPROM_TOTAL_BYTE_COUNT - 1) as u16; // This is the default if not set in QMK. QMK also checks if this is greater than u16::MAX or EEPROM_TOTAL_BYTE_COUNT (e.g. if it was manually set), and generates a compile time error if it is.
                                                                     // Space is calculated for dynamic keymaps assumes 2 bytes per keycode
                                                                     // Note that keycode is 2 bytes instead of 1 because QMK has special keycodes for updating RGB light values, for example, that take 2 bytes instead of 1.
    const DYNAMIC_KEYMAP_EEPROM_SIZE: usize =
        (Self::DYNAMIC_KEYMAP_LAYER_COUNT as usize) * Self::LAYOUT_ROWS * Self::LAYOUT_COLS * 2; // Can't change this in QMK (it only exists as local variable, not a #define)
    const DYNAMIC_KEYMAP_EEPROM_ADDR: u8 = if Self::VIA_ENABLED {
        Self::VIA_EEPROM_CONFIG_END
    } else {
        Self::EECONFIG_SIZE as u8
    }; // This is the default if not defined in QMK

    // Probably won't support this
    const ENCODER_MAP_ENABLE: bool = false; // This is the default if this is not set in QMK.

    const DYNAMIC_KEYMAP_ENCODER_EEPROM_ADDR: u16 =
        (Self::DYNAMIC_KEYMAP_EEPROM_ADDR + (Self::DYNAMIC_KEYMAP_EEPROM_SIZE) as u8) as u16; // This is the default if this is not set in QMK.
    const DYNAMIC_KEYMAP_ENCODER_EEPROM_SIZE: usize =
        (Self::DYNAMIC_KEYMAP_LAYER_COUNT as usize) * (Self::NUM_ENCODERS as usize) * 2 * 2; // Not defined in QMK, just inferred based of patterns here.

    const DYNAMIC_KEYMAP_MACRO_COUNT: u8 = 16; // this is the default if this variable isn't defined in QMK
    const DYNAMIC_KEYMAP_MACRO_EEPROM_ADDR: u16 = if Self::ENCODER_MAP_ENABLE {
        // Space is calculated assuming: 2 bytes per keycode, 2 directions (CW and CCW)
        Self::DYNAMIC_KEYMAP_ENCODER_EEPROM_ADDR + (Self::DYNAMIC_KEYMAP_ENCODER_EEPROM_SIZE as u16)
    } else {
        Self::DYNAMIC_KEYMAP_ENCODER_EEPROM_ADDR
    }; // This is the default if not defined in QMK.
    const DYNAMIC_KEYMAP_MACRO_EEPROM_SIZE: usize = (Self::DYNAMIC_KEYMAP_EEPROM_MAX_ADDR
        - Self::DYNAMIC_KEYMAP_MACRO_EEPROM_ADDR
        + 1) as usize; // This is the default if not defined in QMK.

    fn handle_via_init() {}

    fn handle_via_command() -> bool {
        false
    }

    fn handle_set_layout_options(_data: &mut [u8], _len: u8) {}

    fn handle_custom_value_command(data: &mut [u8], _len: u8) {
        data[0] = ViaCommandId::Unhandled as u8;
    }
}

// Pulled from QMK
pub const VIA_REPORT_DESCRIPTOR: &[u8] = &[
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

const VIA_PROTOCOL_VERSION: u16 = 0x000C;

#[derive(FromPrimitive, Debug, PartialEq, Eq)]
pub enum ViaCommandId {
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
    #[cfg(feature = "vial")]
    VialPrefix,
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
enum ViaBacklightValue {
    Brightness = 1,
    Effect,
}

#[derive(FromPrimitive, Debug)]
enum ViaRGBLightValue {
    Brightness = 1,
    Effect,
    EffectSpeed,
    Color,
}

#[derive(FromPrimitive, Debug)]
enum ViaRGBMatrixValue {
    Brightness = 1,
    Effect,
    EffectSpeed,
    Color,
}

#[derive(FromPrimitive, Debug)]
enum ViaLEDMatrixValue {
    Brightness = 1,
    Effect,
    EffectSpeed,
}

#[allow(dead_code)]
#[derive(FromPrimitive, Debug)]
enum ViaAudioValue {
    Enable,
    ClickyEnable,
}

pub async fn process_via_command<K: ViaKeyboard>(
    debouncer: &'static Mutex<
        ThreadModeRawMutex,
        Debouncer<[[bool; K::LAYOUT_COLS]; K::LAYOUT_ROWS]>,
    >,
    flash: &'static Mutex<ThreadModeRawMutex, impl NorFlash>,
    data: &mut [u8],
) {
    debug!("[VIA] Processing Via command");
    if let Some(command) = num::FromPrimitive::from_u8(data[0]) {
        debug!("[VIA] Received command {:?}", Debug2Format(&command));

        match command {
            ViaCommandId::GetProtocolVersion => {
                data[1..=2].copy_from_slice(&VIA_PROTOCOL_VERSION.to_be_bytes());
            }
            ViaCommandId::GetKeyboardValue => {
                match num::FromPrimitive::from_u8(data[1]) {
                    Some(ViaKeyboardValueId::Uptime) => {
                        data[2..=5].copy_from_slice(
                            &((embassy_time::Instant::now().as_millis() as u32).to_be_bytes()),
                        );
                    }
                    Some(ViaKeyboardValueId::LayoutOptions) => {
                        if let Err(err) = flash.lock().await.read(
                            K::VIA_EEPROM_LAYOUT_OPTIONS_ADDR as u32,
                            &mut data[(6 - K::VIA_EEPROM_LAYOUT_OPTIONS_SIZE)..=5],
                        ) {
                            warn!(
                                "[VIA] Could not read layout options: {:?}",
                                Debug2Format(&err)
                            )
                        };
                    }
                    Some(ViaKeyboardValueId::SwitchMatrixState) => {
                        // (cols + 8 bits - 1) / 8 bits: we get the number of bytes needed to store the state of a row (based on number of cols)
                        for (i, row) in debouncer.lock().await.get().iter().enumerate() {
                            for col in 0..K::LAYOUT_COLS {
                                data[2
                                    + ((K::LAYOUT_COLS + u8::BITS as usize - 1)
                                        / u8::BITS as usize
                                        * (i + 1)
                                        - 1
                                        - col / u8::BITS as usize)] |= (row[col] as u8) << col;
                            }
                        }
                    }
                    Some(ViaKeyboardValueId::FirmwareVersion) => {
                        data[2..=5].copy_from_slice(&K::VIA_FIRMWARE_VERSION.to_be_bytes());
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
                }
            }
            ViaCommandId::SetKeyboardValue => {
                match num::FromPrimitive::from_u8(data[1]) {
                    Some(ViaKeyboardValueId::LayoutOptions) => {
                        K::handle_set_layout_options(data, 32);

                        if let Err(err) = flash.lock().await.write(
                            K::VIA_EEPROM_LAYOUT_OPTIONS_ADDR as u32,
                            &data[2..(2 + K::VIA_EEPROM_LAYOUT_OPTIONS_SIZE)],
                        ) {
                            warn!(
                                "[VIA] Could not write layout options {:?}",
                                Debug2Format(&err)
                            )
                        };
                    }
                    Some(ViaKeyboardValueId::DeviceIndication) => {
                        #[cfg(feature = "backlight")]
                        crate::backlight::BACKLIGHT_COMMAND_CHANNEL
                            .send(crate::backlight::animations::BacklightCommand::Toggle)
                            .await;

                        #[cfg(feature = "underglow")]
                        crate::underglow::UNDERGLOW_COMMAND_CHANNEL
                            .send(crate::underglow::animations::UnderglowCommand::Toggle)
                            .await;
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
            ViaCommandId::EEPROMReset => todo!(),
            ViaCommandId::BootloaderJump => todo!(),
            ViaCommandId::DynamicKeymapMacroGetCount => {
                data[1] = K::DYNAMIC_KEYMAP_MACRO_COUNT;
            }
            ViaCommandId::DynamicKeymapMacroGetBufferSize => {
                data[1..=2].copy_from_slice(&K::DYNAMIC_KEYMAP_MACRO_EEPROM_SIZE.to_be_bytes());
            }
            ViaCommandId::DynamicKeymapMacroGetBuffer => {
                let offset = u16::from_be_bytes(data[1..=2].try_into().unwrap());
                let size = data[3];

                if let Err(err) = flash.lock().await.read(
                    (K::DYNAMIC_KEYMAP_MACRO_EEPROM_SIZE as u16 + offset) as u32,
                    &mut data[4..(4
                        + (if offset + (size as u16) > K::DYNAMIC_KEYMAP_MACRO_EEPROM_SIZE as u16 {
                            if offset as usize >= K::DYNAMIC_KEYMAP_MACRO_EEPROM_SIZE {
                                0
                            } else {
                                K::DYNAMIC_KEYMAP_MACRO_EEPROM_SIZE - offset as usize
                            }
                        } else {
                            size as usize
                        }))],
                ) {
                    warn!(
                        "[VIA] Could not read dynamic keymap macro buffer: {:?}",
                        Debug2Format(&err)
                    )
                };
            }
            ViaCommandId::DynamicKeymapMacroSetBuffer => {
                let offset = u16::from_be_bytes(data[1..=2].try_into().unwrap());
                let size = data[3];

                if let Err(err) = flash.lock().await.write(
                    (K::DYNAMIC_KEYMAP_MACRO_EEPROM_SIZE as u16 + offset) as u32,
                    &data[4..(4
                        + (if offset + (size as u16) > K::DYNAMIC_KEYMAP_MACRO_EEPROM_SIZE as u16 {
                            if offset as usize >= K::DYNAMIC_KEYMAP_MACRO_EEPROM_SIZE {
                                0
                            } else {
                                K::DYNAMIC_KEYMAP_MACRO_EEPROM_SIZE - offset as usize
                            }
                        } else {
                            size as usize
                        }))],
                ) {
                    warn!(
                        "[VIA] Could not write dynamic keymap macro buffer: {:?}",
                        Debug2Format(&err)
                    )
                };
            }
            ViaCommandId::DynamicKeymapMacroReset => todo!(),
            ViaCommandId::DynamicKeymapGetLayerCount => {
                data[1] = K::DYNAMIC_KEYMAP_LAYER_COUNT;
            }
            ViaCommandId::DynamicKeymapGetKeycode => {
                let layer = data[1];
                let row = data[2];
                let col = data[3];

                if !(layer >= K::DYNAMIC_KEYMAP_LAYER_COUNT
                    || row as usize >= K::LAYOUT_ROWS
                    || col as usize >= K::LAYOUT_COLS)
                {
                    if let Err(err) = flash.lock().await.read(
                        (K::DYNAMIC_KEYMAP_EEPROM_ADDR
                            + (layer * K::LAYOUT_ROWS as u8 * K::LAYOUT_COLS as u8 * 2)
                            + (row * K::LAYOUT_COLS as u8 * 2)
                            + (col * 2)) as u32,
                        &mut data[4..=5],
                    ) {
                        warn!(
                            "[VIA] Could not read dynamic keymap keycode: {:?}",
                            Debug2Format(&err)
                        )
                    };
                } else {
                    warn!("[VIA] Requested a dynamic keymap keycode that is out of bounds.")
                }
            }
            ViaCommandId::DynamicKeymapSetKeycode => {
                let layer = data[1];
                let row = data[2];
                let col = data[3];

                if !(layer >= K::DYNAMIC_KEYMAP_LAYER_COUNT
                    || row as usize >= K::LAYOUT_ROWS
                    || col as usize >= K::LAYOUT_COLS)
                {
                    if let Err(err) = flash.lock().await.write(
                        (K::DYNAMIC_KEYMAP_EEPROM_ADDR
                            + (layer * K::LAYOUT_ROWS as u8 * K::LAYOUT_COLS as u8 * 2)
                            + (row * K::LAYOUT_COLS as u8 * 2)
                            + (col * 2)) as u32,
                        &data[4..=5],
                    ) {
                        warn!(
                            "[VIA] Could not write dynamic keymap keycode: {:?}",
                            Debug2Format(&err)
                        )
                    };
                } else {
                    warn!("[VIA] Attempted to write a dynamic keymap keycode out of bounds.")
                }
            }
            ViaCommandId::DynamicKeymapGetEncoder => {
                let layer = data[1];
                let encoder_id = data[2];
                let clockwise = data[3] != 0;

                if !(layer >= K::DYNAMIC_KEYMAP_LAYER_COUNT || encoder_id >= K::NUM_ENCODERS) {
                    if let Err(err) = flash.lock().await.read(
                        (K::DYNAMIC_KEYMAP_ENCODER_EEPROM_ADDR
                            + (layer * K::NUM_ENCODERS * 2 * 2) as u16
                            + (encoder_id * 2 * 2) as u16) as u32
                            + if clockwise { 0 } else { 2 },
                        &mut data[4..=5],
                    ) {
                        warn!(
                            "[VIA] Could not read dynamic keymap encoder: {:?}",
                            Debug2Format(&err)
                        )
                    };
                } else {
                    warn!("[VIA] Requested a dynamic keymap encoder that is out of bounds.")
                }
            } // only if encoder map is enabled
            ViaCommandId::DynamicKeymapSetEncoder => {
                let layer = data[1];
                let encoder_id = data[2];
                let clockwise = data[3] != 0;

                if !(layer >= K::DYNAMIC_KEYMAP_LAYER_COUNT || encoder_id >= K::NUM_ENCODERS) {
                    if let Err(err) = flash.lock().await.write(
                        (K::DYNAMIC_KEYMAP_ENCODER_EEPROM_ADDR
                            + (layer * K::NUM_ENCODERS * 2 * 2) as u16
                            + (encoder_id * 2 * 2) as u16) as u32
                            + if clockwise { 0 } else { 2 },
                        &data[4..=5],
                    ) {
                        warn!(
                            "[VIA] Could not write dynamic keymap encoder: {:?}",
                            Debug2Format(&err)
                        )
                    };
                } else {
                    warn!("[VIA] Attempted to write a dynamic keymap encoder out of bounds.")
                }
            } // only if encoder map is enabled
            ViaCommandId::DynamicKeymapGetBuffer => {
                let offset = u16::from_be_bytes(data[1..=2].try_into().unwrap());
                let size = data[3];

                if let Err(err) = flash.lock().await.read(
                    (K::DYNAMIC_KEYMAP_EEPROM_ADDR as u16 + offset) as u32,
                    &mut data[4..(4
                        + (if offset + (size as u16) > K::DYNAMIC_KEYMAP_EEPROM_SIZE as u16 {
                            if offset as usize >= K::DYNAMIC_KEYMAP_EEPROM_SIZE {
                                0
                            } else {
                                K::DYNAMIC_KEYMAP_EEPROM_SIZE - offset as usize
                            }
                        } else {
                            size as usize
                        }))],
                ) {
                    warn!(
                        "[VIA] Could not read dynamic keymap buffer: {:?}",
                        Debug2Format(&err)
                    )
                };
            }
            ViaCommandId::DynamicKeymapSetBuffer => {
                let offset = u16::from_be_bytes(data[1..=2].try_into().unwrap());
                let size = data[3];

                if let Err(err) = flash.lock().await.write(
                    (K::DYNAMIC_KEYMAP_EEPROM_ADDR as u16 + offset) as u32,
                    &data[4..(4
                        + (if offset + (size as u16) > K::DYNAMIC_KEYMAP_EEPROM_SIZE as u16 {
                            if offset as usize >= K::DYNAMIC_KEYMAP_EEPROM_SIZE {
                                0
                            } else {
                                K::DYNAMIC_KEYMAP_EEPROM_SIZE - offset as usize
                            }
                        } else {
                            size as usize
                        }))],
                ) {
                    warn!(
                        "[VIA] Could not write dynamic keymap buffer: {:?}",
                        Debug2Format(&err)
                    )
                };
            }
            ViaCommandId::DynamicKeymapReset => todo!(), // Need to figure out how to mutate the keyberon layout
            command
                if command == ViaCommandId::CustomGetValue
                    || command == ViaCommandId::CustomSetValue
                    || command == ViaCommandId::CustomSave =>
            {
                match num::FromPrimitive::from_u8(data[1]) {
                    #[cfg(feature = "backlight")]
                    Some(ViaChannelId::Backlight) => {
                        match command {
                            ViaCommandId::CustomGetValue => {
                                let config = crate::backlight::BACKLIGHT_CONFIG_STATE.get().await;

                                match num::FromPrimitive::from_u8(data[2]) {
                                    Some(ViaBacklightValue::Brightness) => {
                                        data[3] = config.val;
                                    }
                                    Some(ViaBacklightValue::Effect) => {
                                        data[3] = config.effect as u8;
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
                                        crate::backlight::BACKLIGHT_COMMAND_CHANNEL
                                            .send(crate::backlight::animations::BacklightCommand::SetValue(data[3]))
                                            .await;
                                    }
                                    Some(ViaBacklightValue::Effect) => {
                                        if let Some(effect) = num::FromPrimitive::from_u8(data[3]) {
                                            crate::backlight::BACKLIGHT_COMMAND_CHANNEL
                                                .send(crate::backlight::animations::BacklightCommand::SetEffect(effect))
                                                .await;
                                        } else {
                                            warn!(
                                                "[VIA] Tried to set an unknown backlight effect: {:?}",
                                                data[3]
                                            )
                                        }
                                    }
                                    None => {
                                        warn!(
                                            "[VIA] Unknown backlight set command received from host {:?}",
                                            data[2]
                                        )
                                    }
                                };
                            }
                            ViaCommandId::CustomSave => {
                                #[cfg(feature = "storage")]
                                crate::backlight::BACKLIGHT_COMMAND_CHANNEL
                                    .send(
                                        crate::backlight::animations::BacklightCommand::SaveConfig,
                                    )
                                    .await;
                            }
                            _ => unreachable!("Should not happen"),
                        };
                    }
                    #[cfg(feature = "backlight")]
                    Some(ViaChannelId::LEDMatrix) => {
                        match command {
                            ViaCommandId::CustomGetValue => {
                                let config = crate::backlight::BACKLIGHT_CONFIG_STATE.get().await;

                                match num::FromPrimitive::from_u8(data[2]) {
                                    Some(ViaLEDMatrixValue::Brightness) => {
                                        data[3] = config.val;
                                    }
                                    Some(ViaLEDMatrixValue::Effect) => {
                                        data[3] = config.effect as u8;
                                    }
                                    Some(ViaLEDMatrixValue::EffectSpeed) => {
                                        data[3] = config.speed;
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
                                        crate::backlight::BACKLIGHT_COMMAND_CHANNEL
                                            .send(crate::backlight::animations::BacklightCommand::SetValue(data[3]))
                                            .await;
                                    }
                                    Some(ViaLEDMatrixValue::Effect) => {
                                        if let Some(effect) = num::FromPrimitive::from_u8(data[3]) {
                                            crate::backlight::BACKLIGHT_COMMAND_CHANNEL
                                                .send(crate::backlight::animations::BacklightCommand::SetEffect(effect))
                                                .await;
                                        } else {
                                            warn!(
                                                "[VIA] Tried to set an unknown LED matrix effect: {:?}",
                                                data[3]
                                            )
                                        }
                                    }
                                    Some(ViaLEDMatrixValue::EffectSpeed) => {
                                        crate::backlight::BACKLIGHT_COMMAND_CHANNEL
                                            .send(crate::backlight::animations::BacklightCommand::SetSpeed(data[3]))
                                            .await;
                                    }
                                    None => {
                                        warn!(
                                            "[VIA] Unknown LED matrix set command received from host {:?}",
                                            data[2]
                                        )
                                    }
                                };
                            }
                            ViaCommandId::CustomSave => {
                                #[cfg(feature = "storage")]
                                crate::backlight::BACKLIGHT_COMMAND_CHANNEL
                                    .send(
                                        crate::backlight::animations::BacklightCommand::SaveConfig,
                                    )
                                    .await;
                            }
                            _ => unreachable!("Should not happen"),
                        };
                    }
                    #[cfg(feature = "backlight")]
                    Some(ViaChannelId::RGBMatrix) => {
                        match command {
                            ViaCommandId::CustomGetValue => {
                                match num::FromPrimitive::from_u8(data[2]) {
                                    Some(ViaRGBMatrixValue::Brightness) => {
                                        // TODO: replace this with just the raw 8bit value of the brightness, no math needed
                                        data[3] = 0;
                                    }
                                    Some(ViaRGBMatrixValue::Effect) => {
                                        // TODO: get effect
                                        data[3] = 0;
                                    }
                                    Some(ViaRGBMatrixValue::EffectSpeed) => {
                                        // TODO: get speed
                                        data[3] = 0;
                                    }
                                    Some(ViaRGBMatrixValue::Color) => {
                                        // TODO: get hue and sat
                                        data[3] = 0; // hue
                                        data[4] = 0; // sat
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
                                        crate::backlight::BACKLIGHT_COMMAND_CHANNEL
                                            .send(crate::backlight::animations::BacklightCommand::SetValue(data[3]))
                                            .await;
                                    }
                                    Some(ViaRGBMatrixValue::Effect) => {
                                        if let Some(effect) = num::FromPrimitive::from_u8(data[3]) {
                                            crate::backlight::BACKLIGHT_COMMAND_CHANNEL
                                                .send(crate::backlight::animations::BacklightCommand::SetEffect(effect))
                                                .await;
                                        } else {
                                            warn!(
                                                "[VIA] Tried to set an unknown backlight effect: {:?}",
                                                data[3]
                                            )
                                        }
                                    }
                                    Some(ViaRGBMatrixValue::EffectSpeed) => {
                                        crate::backlight::BACKLIGHT_COMMAND_CHANNEL
                                            .send(crate::backlight::animations::BacklightCommand::SetSpeed(data[3]))
                                            .await;
                                    }
                                    Some(ViaRGBMatrixValue::Color) => {
                                        // TODO: build separate animation system for RGB matrix, and then implement this
                                    }
                                    None => {
                                        warn!(
                                            "[VIA] Unknown RGB matrix get command received from host {:?}",
                                            data[2]
                                        )
                                    }
                                };
                            }
                            ViaCommandId::CustomSave => {
                                #[cfg(feature = "storage")]
                                crate::backlight::BACKLIGHT_COMMAND_CHANNEL
                                    .send(
                                        crate::backlight::animations::BacklightCommand::SaveConfig,
                                    )
                                    .await;
                            }
                            _ => unreachable!("Should not happen"),
                        };
                    }
                    #[cfg(feature = "underglow")]
                    Some(ViaChannelId::RGBLight) => {
                        match command {
                            ViaCommandId::CustomGetValue => {
                                let config = crate::underglow::UNDERGLOW_CONFIG_STATE.get().await;

                                match num::FromPrimitive::from_u8(data[2]) {
                                    Some(ViaRGBLightValue::Brightness) => {
                                        data[3] = config.val;
                                    }
                                    Some(ViaRGBLightValue::Effect) => {
                                        data[3] = config.effect as u8;
                                    }
                                    Some(ViaRGBLightValue::EffectSpeed) => {
                                        data[3] = config.speed;
                                    }
                                    Some(ViaRGBLightValue::Color) => {
                                        data[3] = config.hue;
                                        data[4] = config.sat;
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
                                        crate::underglow::UNDERGLOW_COMMAND_CHANNEL
                                            .send(crate::underglow::animations::UnderglowCommand::SetValue(data[3]))
                                            .await;
                                    }
                                    Some(ViaRGBLightValue::Effect) => {
                                        if let Some(effect) = num::FromPrimitive::from_u8(data[3]) {
                                            crate::underglow::UNDERGLOW_COMMAND_CHANNEL
                                                .send(crate::underglow::animations::UnderglowCommand::SetEffect(effect))
                                                .await;
                                        } else {
                                            warn!(
                                                "[VIA] Tried to set an unknown underglow effect: {:?}",
                                                data[3]
                                            )
                                        }
                                    }
                                    Some(ViaRGBLightValue::EffectSpeed) => {
                                        crate::underglow::UNDERGLOW_COMMAND_CHANNEL
                                            .send(crate::underglow::animations::UnderglowCommand::SetSpeed(data[3]))
                                            .await;
                                    }
                                    Some(ViaRGBLightValue::Color) => {
                                        crate::underglow::UNDERGLOW_COMMAND_CHANNEL
                                            .send(crate::underglow::animations::UnderglowCommand::SetHue(data[3]))
                                            .await;
                                        crate::underglow::UNDERGLOW_COMMAND_CHANNEL
                                            .send(crate::underglow::animations::UnderglowCommand::SetSaturation(data[4]))
                                            .await;
                                    }
                                    None => {
                                        warn!(
                                            "[VIA] Unknown RGB underglow get command received from host {:?}",
                                            data[2]
                                        )
                                    }
                                };
                            }
                            ViaCommandId::CustomSave => {
                                #[cfg(feature = "storage")]
                                crate::underglow::UNDERGLOW_COMMAND_CHANNEL
                                    .send(
                                        crate::underglow::animations::UnderglowCommand::SaveConfig,
                                    )
                                    .await;
                            }
                            _ => unreachable!("Should not happen"),
                        };
                    }
                    Some(ViaChannelId::Audio) => {
                        // TODO: audio channel
                        data[0] = ViaCommandId::Unhandled as u8;
                        warn!(
                            "[VIA] Unimplemented channel ID received from host: {:?}",
                            Debug2Format(&ViaChannelId::Audio)
                        );
                    }
                    other => {
                        if other.is_none() {
                            warn!(
                                "[VIA] Unknown channel ID received from host, user function called: {:?}",
                                Debug2Format(&data[1])
                            )
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

pub struct ViaCommandHandler {}

static VIA_COMMAND_HANDLER: ViaCommandHandler = ViaCommandHandler {};

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

// Channel with via responses to send back to PC
pub static VIA_REPORT_HID_SEND_CHANNEL: Channel<ThreadModeRawMutex, [u8; 32], 1> = Channel::new();

#[rumcake_macros::task]
pub async fn usb_hid_via_read_task(hid: HidReader<'static, impl Driver<'static>, 32>) {
    hid.run(false, &VIA_COMMAND_HANDLER).await;
}

#[rumcake_macros::task]
pub async fn usb_hid_via_write_task<K: ViaKeyboard>(
    _k: K,
    debouncer: &'static Mutex<
        ThreadModeRawMutex,
        Debouncer<[[bool; K::LAYOUT_COLS]; K::LAYOUT_ROWS]>,
    >,
    flash: &'static Mutex<ThreadModeRawMutex, impl NorFlash>,
    mut hid: HidWriter<'static, impl Driver<'static>, 32>,
) {
    loop {
        let mut report = VIA_REPORT_HID_SEND_CHANNEL.receive().await;

        if K::VIA_ENABLED {
            process_via_command::<K>(debouncer, flash, &mut report).await;

            debug!("[VIA] Writing HID raw report {:?}", Debug2Format(&report));
            if let Err(err) = hid.write(&report).await {
                error!(
                    "[VIA] Couldn't write HID raw report: {:?}",
                    Debug2Format(&err)
                );
            };
        }
    }
}
