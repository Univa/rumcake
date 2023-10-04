use crate::keyboard::Keyboard;

pub trait KeyboardWithEEPROM: Keyboard {
    // Probably not using these.
    const EECONFIG_KB_DATA_SIZE: usize = 0; // This is the default if it is not set in QMK
    const EECONFIG_USER_DATA_SIZE: usize = 0; // This is the default if it is not set in QMK

    // While most of these are not implemented in this firmware, our EECONFIG addresses will follow the same structure that QMK uses.
    const EECONFIG_MAGIC_ADDR: u16 = 0;
    const EECONFIG_DEBUG_ADDR: u8 = 2;
    const EECONFIG_DEFAULT_LAYER_ADDR: u8 = 3;
    const EECONFIG_KEYMAP_ADDR: u16 = 4;
    const EECONFIG_BACKLIGHT_ADDR: u8 = 6;
    const EECONFIG_AUDIO_ADDR: u8 = 7;
    const EECONFIG_RGBLIGHT_ADDR: u32 = 8;
    const EECONFIG_UNICODEMODE_ADDR: u8 = 12;
    const EECONFIG_STENOMODE_ADDR: u8 = 13;
    const EECONFIG_HANDEDNESS_ADDR: u8 = 14;
    const EECONFIG_KEYBOARD_ADDR: u32 = 15;
    const EECONFIG_USER_ADDR: u32 = 19;
    const EECONFIG_VELOCIKEY_ADDR: u8 = 23;
    const EECONFIG_LED_MATRIX_ADDR: u32 = 24;
    const EECONFIG_RGB_MATRIX_ADDR: u64 = 24;
    const EECONFIG_HAPTIC_ADDR: u32 = 32;
    const EECONFIG_RGBLIGHT_EXTENDED_ADDR: u8 = 36;

    // Note: this is just the *base* size to use the features above.
    // VIA will use more EECONFIG space starting at the address below (address 37) and beyond.
    const EECONFIG_BASE_SIZE: usize = 37;
    const EECONFIG_SIZE: usize =
        Self::EECONFIG_BASE_SIZE + Self::EECONFIG_KB_DATA_SIZE + Self::EECONFIG_USER_DATA_SIZE;

    // Note: QMK uses an algorithm to emulate EEPROM in STM32 chips by using their flash peripherals
    const EEPROM_TOTAL_BYTE_COUNT: usize = Self::EECONFIG_SIZE + 3;
}

// TODO: eeprom emulation with flash
