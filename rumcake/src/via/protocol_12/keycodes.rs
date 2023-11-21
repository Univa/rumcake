use keyberon::action::Action;
use num_derive::FromPrimitive;

use crate::keyboard::Keycode;

#[repr(u16)]
#[allow(non_camel_case_types)]
/// List of QMK Keycode ranges. This is mainly used for reference.
enum QMKKeycodeRanges {
    QK_BASIC = 0x0000, // TODO: consumer-related keycodes
    QK_BASIC_MAX = 0x00FF,
    QK_MODS = 0x0100, // TODO: unhandled
    QK_MODS_MAX = 0x1FFF,
    QK_MOD_TAP = 0x2000, // TODO: unhandled
    QK_MOD_TAP_MAX = 0x3FFF,
    QK_LAYER_TAP = 0x4000, // TODO: unhandled
    QK_LAYER_TAP_MAX = 0x4FFF,
    QK_LAYER_MOD = 0x5000, // TODO: unhandled
    QK_LAYER_MOD_MAX = 0x51FF,
    QK_TO = 0x5200, // TODO: unhandled
    QK_TO_MAX = 0x521F,
    QK_MOMENTARY = 0x5220,
    QK_MOMENTARY_MAX = 0x523F,
    QK_DEF_LAYER = 0x5240,
    QK_DEF_LAYER_MAX = 0x525F,
    QK_TOGGLE_LAYER = 0x5260, // TODO: unhandled
    QK_TOGGLE_LAYER_MAX = 0x527F,
    QK_ONE_SHOT_LAYER = 0x5280, // TODO: unhandled, switch to kanata keyberon fork
    QK_ONE_SHOT_LAYER_MAX = 0x529F,
    QK_ONE_SHOT_MOD = 0x52A0, // TODO: unhandled, switch to kanata keyberon fork
    QK_ONE_SHOT_MOD_MAX = 0x52BF,
    QK_LAYER_TAP_TOGGLE = 0x52C0, // TODO: unhandled
    QK_LAYER_TAP_TOGGLE_MAX = 0x52DF,
    QK_SWAP_HANDS = 0x5600, // TODO: unhandled
    QK_SWAP_HANDS_MAX = 0x56FF,
    QK_TAP_DANCE = 0x5700, // TODO: unhandled, switch to kanata keyberon fork
    QK_TAP_DANCE_MAX = 0x57FF,
    QK_MAGIC = 0x7000, // TODO: unhandled
    QK_MAGIC_MAX = 0x70FF,
    QK_MIDI = 0x7100, // TODO: unhandled
    QK_MIDI_MAX = 0x71FF,
    QK_SEQUENCER = 0x7200, // TODO: unhandled
    QK_SEQUENCER_MAX = 0x73FF,
    QK_JOYSTICK = 0x7400, // TODO: unhandled
    QK_JOYSTICK_MAX = 0x743F,
    QK_PROGRAMMABLE_BUTTON = 0x7440, // TODO: unhandled
    QK_PROGRAMMABLE_BUTTON_MAX = 0x747F,
    QK_AUDIO = 0x7480, // TODO: unhandled
    QK_AUDIO_MAX = 0x74BF,
    QK_STENO = 0x74C0, // TODO: unhandled
    QK_STENO_MAX = 0x74FF,
    QK_MACRO = 0x7700, // TODO: unhandled
    QK_MACRO_MAX = 0x777F,
    QK_LIGHTING = 0x7800,
    QK_LIGHTING_MAX = 0x78FF,
    QK_QUANTUM = 0x7C00, // TODO: unhandled
    QK_QUANTUM_MAX = 0x7DFF,
    QK_KB = 0x7E00, // TODO: unhandled
    QK_KB_MAX = 0x7E3F,
    QK_USER = 0x7E40,
    QK_USER_MAX = 0x7FFF,
    // QK_UNICODEMAP = 0x8000, // same as QK_UNICODE
    QK_UNICODEMAP_MAX = 0xBFFF,
    QK_UNICODE = 0x8000, // TODO: unhandled
    QK_UNICODE_MAX = 0xFFFF,
    QK_UNICODEMAP_PAIR = 0xC000,
    // QK_UNICODEMAP_PAIR_MAX = 0xFFFF, // same as QK_UNICODE_MAX
}

#[repr(u16)]
#[allow(non_camel_case_types)]
#[derive(FromPrimitive)]
/// List of QMK Keycodes. This is mainly used for reference.
enum QMKKeycodes {
    // QK_BASIC start
    KC_NO = 0x0000,
    KC_TRANSPARENT = 0x0001,
    KC_A = 0x0004,
    KC_B = 0x0005,
    KC_C = 0x0006,
    KC_D = 0x0007,
    KC_E = 0x0008,
    KC_F = 0x0009,
    KC_G = 0x000A,
    KC_H = 0x000B,
    KC_I = 0x000C,
    KC_J = 0x000D,
    KC_K = 0x000E,
    KC_L = 0x000F,
    KC_M = 0x0010,
    KC_N = 0x0011,
    KC_O = 0x0012,
    KC_P = 0x0013,
    KC_Q = 0x0014,
    KC_R = 0x0015,
    KC_S = 0x0016,
    KC_T = 0x0017,
    KC_U = 0x0018,
    KC_V = 0x0019,
    KC_W = 0x001A,
    KC_X = 0x001B,
    KC_Y = 0x001C,
    KC_Z = 0x001D,
    KC_1 = 0x001E,
    KC_2 = 0x001F,
    KC_3 = 0x0020,
    KC_4 = 0x0021,
    KC_5 = 0x0022,
    KC_6 = 0x0023,
    KC_7 = 0x0024,
    KC_8 = 0x0025,
    KC_9 = 0x0026,
    KC_0 = 0x0027,
    KC_ENTER = 0x0028,
    KC_ESCAPE = 0x0029,
    KC_BACKSPACE = 0x002A,
    KC_TAB = 0x002B,
    KC_SPACE = 0x002C,
    KC_MINUS = 0x002D,
    KC_EQUAL = 0x002E,
    KC_LEFT_BRACKET = 0x002F,
    KC_RIGHT_BRACKET = 0x0030,
    KC_BACKSLASH = 0x0031,
    KC_NONUS_HASH = 0x0032,
    KC_SEMICOLON = 0x0033,
    KC_QUOTE = 0x0034,
    KC_GRAVE = 0x0035,
    KC_COMMA = 0x0036,
    KC_DOT = 0x0037,
    KC_SLASH = 0x0038,
    KC_CAPS_LOCK = 0x0039,
    KC_F1 = 0x003A,
    KC_F2 = 0x003B,
    KC_F3 = 0x003C,
    KC_F4 = 0x003D,
    KC_F5 = 0x003E,
    KC_F6 = 0x003F,
    KC_F7 = 0x0040,
    KC_F8 = 0x0041,
    KC_F9 = 0x0042,
    KC_F10 = 0x0043,
    KC_F11 = 0x0044,
    KC_F12 = 0x0045,
    KC_PRINT_SCREEN = 0x0046,
    KC_SCROLL_LOCK = 0x0047,
    KC_PAUSE = 0x0048,
    KC_INSERT = 0x0049,
    KC_HOME = 0x004A,
    KC_PAGE_UP = 0x004B,
    KC_DELETE = 0x004C,
    KC_END = 0x004D,
    KC_PAGE_DOWN = 0x004E,
    KC_RIGHT = 0x004F,
    KC_LEFT = 0x0050,
    KC_DOWN = 0x0051,
    KC_UP = 0x0052,
    KC_NUM_LOCK = 0x0053,
    KC_KP_SLASH = 0x0054,
    KC_KP_ASTERISK = 0x0055,
    KC_KP_MINUS = 0x0056,
    KC_KP_PLUS = 0x0057,
    KC_KP_ENTER = 0x0058,
    KC_KP_1 = 0x0059,
    KC_KP_2 = 0x005A,
    KC_KP_3 = 0x005B,
    KC_KP_4 = 0x005C,
    KC_KP_5 = 0x005D,
    KC_KP_6 = 0x005E,
    KC_KP_7 = 0x005F,
    KC_KP_8 = 0x0060,
    KC_KP_9 = 0x0061,
    KC_KP_0 = 0x0062,
    KC_KP_DOT = 0x0063,
    KC_NONUS_BACKSLASH = 0x0064,
    KC_APPLICATION = 0x0065,
    KC_KB_POWER = 0x0066,
    KC_KP_EQUAL = 0x0067,
    KC_F13 = 0x0068,
    KC_F14 = 0x0069,
    KC_F15 = 0x006A,
    KC_F16 = 0x006B,
    KC_F17 = 0x006C,
    KC_F18 = 0x006D,
    KC_F19 = 0x006E,
    KC_F20 = 0x006F,
    KC_F21 = 0x0070,
    KC_F22 = 0x0071,
    KC_F23 = 0x0072,
    KC_F24 = 0x0073,
    KC_EXECUTE = 0x0074,
    KC_HELP = 0x0075,
    KC_MENU = 0x0076,
    KC_SELECT = 0x0077,
    KC_STOP = 0x0078,
    KC_AGAIN = 0x0079,
    KC_UNDO = 0x007A,
    KC_CUT = 0x007B,
    KC_COPY = 0x007C,
    KC_PASTE = 0x007D,
    KC_FIND = 0x007E,
    KC_KB_MUTE = 0x007F,
    KC_KB_VOLUME_UP = 0x0080,
    KC_KB_VOLUME_DOWN = 0x0081,
    KC_LOCKING_CAPS_LOCK = 0x0082,
    KC_LOCKING_NUM_LOCK = 0x0083,
    KC_LOCKING_SCROLL_LOCK = 0x0084,
    KC_KP_COMMA = 0x0085,
    KC_KP_EQUAL_AS400 = 0x0086,
    KC_INTERNATIONAL_1 = 0x0087,
    KC_INTERNATIONAL_2 = 0x0088,
    KC_INTERNATIONAL_3 = 0x0089,
    KC_INTERNATIONAL_4 = 0x008A,
    KC_INTERNATIONAL_5 = 0x008B,
    KC_INTERNATIONAL_6 = 0x008C,
    KC_INTERNATIONAL_7 = 0x008D,
    KC_INTERNATIONAL_8 = 0x008E,
    KC_INTERNATIONAL_9 = 0x008F,
    KC_LANGUAGE_1 = 0x0090,
    KC_LANGUAGE_2 = 0x0091,
    KC_LANGUAGE_3 = 0x0092,
    KC_LANGUAGE_4 = 0x0093,
    KC_LANGUAGE_5 = 0x0094,
    KC_LANGUAGE_6 = 0x0095,
    KC_LANGUAGE_7 = 0x0096,
    KC_LANGUAGE_8 = 0x0097,
    KC_LANGUAGE_9 = 0x0098,
    KC_ALTERNATE_ERASE = 0x0099,
    KC_SYSTEM_REQUEST = 0x009A,
    KC_CANCEL = 0x009B,
    KC_CLEAR = 0x009C,
    KC_PRIOR = 0x009D,
    KC_RETURN = 0x009E,
    KC_SEPARATOR = 0x009F,
    KC_OUT = 0x00A0,
    KC_OPER = 0x00A1,
    KC_CLEAR_AGAIN = 0x00A2,
    KC_CRSEL = 0x00A3,
    KC_EXSEL = 0x00A4,
    // 0xA5-0xDF start (these values are reserved, but used by QMK for consumer-related keycodes)
    KC_SYSTEM_POWER = 0x00A5,
    KC_SYSTEM_SLEEP = 0x00A6,
    KC_SYSTEM_WAKE = 0x00A7,
    KC_AUDIO_MUTE = 0x00A8,
    KC_AUDIO_VOL_UP = 0x00A9,
    KC_AUDIO_VOL_DOWN = 0x00AA,
    KC_MEDIA_NEXT_TRACK = 0x00AB,
    KC_MEDIA_PREV_TRACK = 0x00AC,
    KC_MEDIA_STOP = 0x00AD,
    KC_MEDIA_PLAY_PAUSE = 0x00AE,
    KC_MEDIA_SELECT = 0x00AF,
    KC_MEDIA_EJECT = 0x00B0,
    KC_MAIL = 0x00B1,
    KC_CALCULATOR = 0x00B2,
    KC_MY_COMPUTER = 0x00B3,
    KC_WWW_SEARCH = 0x00B4,
    KC_WWW_HOME = 0x00B5,
    KC_WWW_BACK = 0x00B6,
    KC_WWW_FORWARD = 0x00B7,
    KC_WWW_STOP = 0x00B8,
    KC_WWW_REFRESH = 0x00B9,
    KC_WWW_FAVORITES = 0x00BA,
    KC_MEDIA_FAST_FORWARD = 0x00BB,
    KC_MEDIA_REWIND = 0x00BC,
    KC_BRIGHTNESS_UP = 0x00BD,
    KC_BRIGHTNESS_DOWN = 0x00BE,
    KC_CONTROL_PANEL = 0x00BF,
    KC_ASSISTANT = 0x00C0,
    KC_MISSION_CONTROL = 0x00C1,
    KC_LAUNCHPAD = 0x00C2,
    KC_MS_UP = 0x00CD,
    KC_MS_DOWN = 0x00CE,
    KC_MS_LEFT = 0x00CF,
    KC_MS_RIGHT = 0x00D0,
    KC_MS_BTN1 = 0x00D1,
    KC_MS_BTN2 = 0x00D2,
    KC_MS_BTN3 = 0x00D3,
    KC_MS_BTN4 = 0x00D4,
    KC_MS_BTN5 = 0x00D5,
    KC_MS_BTN6 = 0x00D6,
    KC_MS_BTN7 = 0x00D7,
    KC_MS_BTN8 = 0x00D8,
    KC_MS_WH_UP = 0x00D9,
    KC_MS_WH_DOWN = 0x00DA,
    KC_MS_WH_LEFT = 0x00DB,
    KC_MS_WH_RIGHT = 0x00DC,
    KC_MS_ACCEL0 = 0x00DD,
    KC_MS_ACCEL1 = 0x00DE,
    KC_MS_ACCEL2 = 0x00DF,
    // 0xA5-0xDF end (these values are reserved, but used by QMK for consumer-related keycodes)
    KC_LEFT_CTRL = 0x00E0,
    KC_LEFT_SHIFT = 0x00E1,
    KC_LEFT_ALT = 0x00E2,
    KC_LEFT_GUI = 0x00E3,
    KC_RIGHT_CTRL = 0x00E4,
    KC_RIGHT_SHIFT = 0x00E5,
    KC_RIGHT_ALT = 0x00E6,
    KC_RIGHT_GUI = 0x00E7,
    // 0xE7-0xFF (these values are reserved)
    // QK_BASIC end

    // QK_MODS = 0x0100,
    // QK_MODS_MAX = 0x1FFF,

    // QK_MOD_TAP = 0x2000,
    // QK_MOD_TAP_MAX = 0x3FFF,

    // QK_LAYER_TAP = 0x4000,
    // QK_LAYER_TAP_MAX = 0x4FFF,

    // QK_LAYER_MOD = 0x5000,
    // QK_LAYER_MOD_MAX = 0x51FF,

    // QK_TO = 0x5200,
    // QK_TO_MAX = 0x521F,

    // QK_MOMENTARY = 0x5220,
    // QK_MOMENTARY_MAX = 0x523F,

    // QK_DEF_LAYER = 0x5240,
    // QK_DEF_LAYER_MAX = 0x525F,

    // QK_TOGGLE_LAYER = 0x5260,
    // QK_TOGGLE_LAYER_MAX = 0x527F,

    // QK_ONE_SHOT_LAYER = 0x5280,
    // QK_ONE_SHOT_LAYER_MAX = 0x529F,

    // QK_ONE_SHOT_MOD = 0x52A0,
    // QK_ONE_SHOT_MOD_MAX = 0x52BF,

    // QK_LAYER_TAP_TOGGLE = 0x52C0,
    // QK_LAYER_TAP_TOGGLE_MAX = 0x52DF,

    // QK_SWAP_HANDS start
    QK_SWAP_HANDS_TOGGLE = 0x56F0,
    QK_SWAP_HANDS_TAP_TOGGLE = 0x56F1,
    QK_SWAP_HANDS_MOMENTARY_ON = 0x56F2,
    QK_SWAP_HANDS_MOMENTARY_OFF = 0x56F3,
    QK_SWAP_HANDS_OFF = 0x56F4,
    QK_SWAP_HANDS_ON = 0x56F5,
    QK_SWAP_HANDS_ONE_SHOT = 0x56F6,
    // QK_SWAP_HANDS end

    // QK_MAGIC start
    QK_MAGIC_SWAP_CONTROL_CAPS_LOCK = 0x7000,
    QK_MAGIC_UNSWAP_CONTROL_CAPS_LOCK = 0x7001,
    QK_MAGIC_TOGGLE_CONTROL_CAPS_LOCK = 0x7002,
    QK_MAGIC_CAPS_LOCK_AS_CONTROL_OFF = 0x7003,
    QK_MAGIC_CAPS_LOCK_AS_CONTROL_ON = 0x7004,
    QK_MAGIC_SWAP_LALT_LGUI = 0x7005,
    QK_MAGIC_UNSWAP_LALT_LGUI = 0x7006,
    QK_MAGIC_SWAP_RALT_RGUI = 0x7007,
    QK_MAGIC_UNSWAP_RALT_RGUI = 0x7008,
    QK_MAGIC_GUI_ON = 0x7009,
    QK_MAGIC_GUI_OFF = 0x700A,
    QK_MAGIC_TOGGLE_GUI = 0x700B,
    QK_MAGIC_SWAP_GRAVE_ESC = 0x700C,
    QK_MAGIC_UNSWAP_GRAVE_ESC = 0x700D,
    QK_MAGIC_SWAP_BACKSLASH_BACKSPACE = 0x700E,
    QK_MAGIC_UNSWAP_BACKSLASH_BACKSPACE = 0x700F,
    QK_MAGIC_TOGGLE_BACKSLASH_BACKSPACE = 0x7010,
    QK_MAGIC_NKRO_ON = 0x7011,
    QK_MAGIC_NKRO_OFF = 0x7012,
    QK_MAGIC_TOGGLE_NKRO = 0x7013,
    QK_MAGIC_SWAP_ALT_GUI = 0x7014,
    QK_MAGIC_UNSWAP_ALT_GUI = 0x7015,
    QK_MAGIC_TOGGLE_ALT_GUI = 0x7016,
    QK_MAGIC_SWAP_LCTL_LGUI = 0x7017,
    QK_MAGIC_UNSWAP_LCTL_LGUI = 0x7018,
    QK_MAGIC_SWAP_RCTL_RGUI = 0x7019,
    QK_MAGIC_UNSWAP_RCTL_RGUI = 0x701A,
    QK_MAGIC_SWAP_CTL_GUI = 0x701B,
    QK_MAGIC_UNSWAP_CTL_GUI = 0x701C,
    QK_MAGIC_TOGGLE_CTL_GUI = 0x701D,
    QK_MAGIC_EE_HANDS_LEFT = 0x701E,
    QK_MAGIC_EE_HANDS_RIGHT = 0x701F,
    QK_MAGIC_SWAP_ESCAPE_CAPS_LOCK = 0x7020,
    QK_MAGIC_UNSWAP_ESCAPE_CAPS_LOCK = 0x7021,
    QK_MAGIC_TOGGLE_ESCAPE_CAPS_LOCK = 0x7022,
    // QK_MAGIC end

    // QK_MIDI start
    QK_MIDI_ON = 0x7100,
    QK_MIDI_OFF = 0x7101,
    QK_MIDI_TOGGLE = 0x7102,
    QK_MIDI_NOTE_C_0 = 0x7103,
    QK_MIDI_NOTE_C_SHARP_0 = 0x7104,
    QK_MIDI_NOTE_D_0 = 0x7105,
    QK_MIDI_NOTE_D_SHARP_0 = 0x7106,
    QK_MIDI_NOTE_E_0 = 0x7107,
    QK_MIDI_NOTE_F_0 = 0x7108,
    QK_MIDI_NOTE_F_SHARP_0 = 0x7109,
    QK_MIDI_NOTE_G_0 = 0x710A,
    QK_MIDI_NOTE_G_SHARP_0 = 0x710B,
    QK_MIDI_NOTE_A_0 = 0x710C,
    QK_MIDI_NOTE_A_SHARP_0 = 0x710D,
    QK_MIDI_NOTE_B_0 = 0x710E,
    QK_MIDI_NOTE_C_1 = 0x710F,
    QK_MIDI_NOTE_C_SHARP_1 = 0x7110,
    QK_MIDI_NOTE_D_1 = 0x7111,
    QK_MIDI_NOTE_D_SHARP_1 = 0x7112,
    QK_MIDI_NOTE_E_1 = 0x7113,
    QK_MIDI_NOTE_F_1 = 0x7114,
    QK_MIDI_NOTE_F_SHARP_1 = 0x7115,
    QK_MIDI_NOTE_G_1 = 0x7116,
    QK_MIDI_NOTE_G_SHARP_1 = 0x7117,
    QK_MIDI_NOTE_A_1 = 0x7118,
    QK_MIDI_NOTE_A_SHARP_1 = 0x7119,
    QK_MIDI_NOTE_B_1 = 0x711A,
    QK_MIDI_NOTE_C_2 = 0x711B,
    QK_MIDI_NOTE_C_SHARP_2 = 0x711C,
    QK_MIDI_NOTE_D_2 = 0x711D,
    QK_MIDI_NOTE_D_SHARP_2 = 0x711E,
    QK_MIDI_NOTE_E_2 = 0x711F,
    QK_MIDI_NOTE_F_2 = 0x7120,
    QK_MIDI_NOTE_F_SHARP_2 = 0x7121,
    QK_MIDI_NOTE_G_2 = 0x7122,
    QK_MIDI_NOTE_G_SHARP_2 = 0x7123,
    QK_MIDI_NOTE_A_2 = 0x7124,
    QK_MIDI_NOTE_A_SHARP_2 = 0x7125,
    QK_MIDI_NOTE_B_2 = 0x7126,
    QK_MIDI_NOTE_C_3 = 0x7127,
    QK_MIDI_NOTE_C_SHARP_3 = 0x7128,
    QK_MIDI_NOTE_D_3 = 0x7129,
    QK_MIDI_NOTE_D_SHARP_3 = 0x712A,
    QK_MIDI_NOTE_E_3 = 0x712B,
    QK_MIDI_NOTE_F_3 = 0x712C,
    QK_MIDI_NOTE_F_SHARP_3 = 0x712D,
    QK_MIDI_NOTE_G_3 = 0x712E,
    QK_MIDI_NOTE_G_SHARP_3 = 0x712F,
    QK_MIDI_NOTE_A_3 = 0x7130,
    QK_MIDI_NOTE_A_SHARP_3 = 0x7131,
    QK_MIDI_NOTE_B_3 = 0x7132,
    QK_MIDI_NOTE_C_4 = 0x7133,
    QK_MIDI_NOTE_C_SHARP_4 = 0x7134,
    QK_MIDI_NOTE_D_4 = 0x7135,
    QK_MIDI_NOTE_D_SHARP_4 = 0x7136,
    QK_MIDI_NOTE_E_4 = 0x7137,
    QK_MIDI_NOTE_F_4 = 0x7138,
    QK_MIDI_NOTE_F_SHARP_4 = 0x7139,
    QK_MIDI_NOTE_G_4 = 0x713A,
    QK_MIDI_NOTE_G_SHARP_4 = 0x713B,
    QK_MIDI_NOTE_A_4 = 0x713C,
    QK_MIDI_NOTE_A_SHARP_4 = 0x713D,
    QK_MIDI_NOTE_B_4 = 0x713E,
    QK_MIDI_NOTE_C_5 = 0x713F,
    QK_MIDI_NOTE_C_SHARP_5 = 0x7140,
    QK_MIDI_NOTE_D_5 = 0x7141,
    QK_MIDI_NOTE_D_SHARP_5 = 0x7142,
    QK_MIDI_NOTE_E_5 = 0x7143,
    QK_MIDI_NOTE_F_5 = 0x7144,
    QK_MIDI_NOTE_F_SHARP_5 = 0x7145,
    QK_MIDI_NOTE_G_5 = 0x7146,
    QK_MIDI_NOTE_G_SHARP_5 = 0x7147,
    QK_MIDI_NOTE_A_5 = 0x7148,
    QK_MIDI_NOTE_A_SHARP_5 = 0x7149,
    QK_MIDI_NOTE_B_5 = 0x714A,
    QK_MIDI_OCTAVE_N2 = 0x714B,
    QK_MIDI_OCTAVE_N1 = 0x714C,
    QK_MIDI_OCTAVE_0 = 0x714D,
    QK_MIDI_OCTAVE_1 = 0x714E,
    QK_MIDI_OCTAVE_2 = 0x714F,
    QK_MIDI_OCTAVE_3 = 0x7150,
    QK_MIDI_OCTAVE_4 = 0x7151,
    QK_MIDI_OCTAVE_5 = 0x7152,
    QK_MIDI_OCTAVE_6 = 0x7153,
    QK_MIDI_OCTAVE_7 = 0x7154,
    QK_MIDI_OCTAVE_DOWN = 0x7155,
    QK_MIDI_OCTAVE_UP = 0x7156,
    QK_MIDI_TRANSPOSE_N6 = 0x7157,
    QK_MIDI_TRANSPOSE_N5 = 0x7158,
    QK_MIDI_TRANSPOSE_N4 = 0x7159,
    QK_MIDI_TRANSPOSE_N3 = 0x715A,
    QK_MIDI_TRANSPOSE_N2 = 0x715B,
    QK_MIDI_TRANSPOSE_N1 = 0x715C,
    QK_MIDI_TRANSPOSE_0 = 0x715D,
    QK_MIDI_TRANSPOSE_1 = 0x715E,
    QK_MIDI_TRANSPOSE_2 = 0x715F,
    QK_MIDI_TRANSPOSE_3 = 0x7160,
    QK_MIDI_TRANSPOSE_4 = 0x7161,
    QK_MIDI_TRANSPOSE_5 = 0x7162,
    QK_MIDI_TRANSPOSE_6 = 0x7163,
    QK_MIDI_TRANSPOSE_DOWN = 0x7164,
    QK_MIDI_TRANSPOSE_UP = 0x7165,
    QK_MIDI_VELOCITY_0 = 0x7166,
    QK_MIDI_VELOCITY_1 = 0x7167,
    QK_MIDI_VELOCITY_2 = 0x7168,
    QK_MIDI_VELOCITY_3 = 0x7169,
    QK_MIDI_VELOCITY_4 = 0x716A,
    QK_MIDI_VELOCITY_5 = 0x716B,
    QK_MIDI_VELOCITY_6 = 0x716C,
    QK_MIDI_VELOCITY_7 = 0x716D,
    QK_MIDI_VELOCITY_8 = 0x716E,
    QK_MIDI_VELOCITY_9 = 0x716F,
    QK_MIDI_VELOCITY_10 = 0x7170,
    QK_MIDI_VELOCITY_DOWN = 0x7171,
    QK_MIDI_VELOCITY_UP = 0x7172,
    QK_MIDI_CHANNEL_1 = 0x7173,
    QK_MIDI_CHANNEL_2 = 0x7174,
    QK_MIDI_CHANNEL_3 = 0x7175,
    QK_MIDI_CHANNEL_4 = 0x7176,
    QK_MIDI_CHANNEL_5 = 0x7177,
    QK_MIDI_CHANNEL_6 = 0x7178,
    QK_MIDI_CHANNEL_7 = 0x7179,
    QK_MIDI_CHANNEL_8 = 0x717A,
    QK_MIDI_CHANNEL_9 = 0x717B,
    QK_MIDI_CHANNEL_10 = 0x717C,
    QK_MIDI_CHANNEL_11 = 0x717D,
    QK_MIDI_CHANNEL_12 = 0x717E,
    QK_MIDI_CHANNEL_13 = 0x717F,
    QK_MIDI_CHANNEL_14 = 0x7180,
    QK_MIDI_CHANNEL_15 = 0x7181,
    QK_MIDI_CHANNEL_16 = 0x7182,
    QK_MIDI_CHANNEL_DOWN = 0x7183,
    QK_MIDI_CHANNEL_UP = 0x7184,
    QK_MIDI_ALL_NOTES_OFF = 0x7185,
    QK_MIDI_SUSTAIN = 0x7186,
    QK_MIDI_PORTAMENTO = 0x7187,
    QK_MIDI_SOSTENUTO = 0x7188,
    QK_MIDI_SOFT = 0x7189,
    QK_MIDI_LEGATO = 0x718A,
    QK_MIDI_MODULATION = 0x718B,
    QK_MIDI_MODULATION_SPEED_DOWN = 0x718C,
    QK_MIDI_MODULATION_SPEED_UP = 0x718D,
    QK_MIDI_PITCH_BEND_DOWN = 0x718E,
    QK_MIDI_PITCH_BEND_UP = 0x718F,
    // QK_MIDI end

    // QK_SEQUENCER start
    QK_SEQUENCER_ON = 0x7200,
    QK_SEQUENCER_OFF = 0x7201,
    QK_SEQUENCER_TOGGLE = 0x7202,
    QK_SEQUENCER_TEMPO_DOWN = 0x7203,
    QK_SEQUENCER_TEMPO_UP = 0x7204,
    QK_SEQUENCER_RESOLUTION_DOWN = 0x7205,
    QK_SEQUENCER_RESOLUTION_UP = 0x7206,
    QK_SEQUENCER_STEPS_ALL = 0x7207,
    QK_SEQUENCER_STEPS_CLEAR = 0x7208,
    // QK_SEQUENCER end

    // QK_JOYSTICK start
    QK_JOYSTICK_BUTTON_0 = 0x7400,
    QK_JOYSTICK_BUTTON_1 = 0x7401,
    QK_JOYSTICK_BUTTON_2 = 0x7402,
    QK_JOYSTICK_BUTTON_3 = 0x7403,
    QK_JOYSTICK_BUTTON_4 = 0x7404,
    QK_JOYSTICK_BUTTON_5 = 0x7405,
    QK_JOYSTICK_BUTTON_6 = 0x7406,
    QK_JOYSTICK_BUTTON_7 = 0x7407,
    QK_JOYSTICK_BUTTON_8 = 0x7408,
    QK_JOYSTICK_BUTTON_9 = 0x7409,
    QK_JOYSTICK_BUTTON_10 = 0x740A,
    QK_JOYSTICK_BUTTON_11 = 0x740B,
    QK_JOYSTICK_BUTTON_12 = 0x740C,
    QK_JOYSTICK_BUTTON_13 = 0x740D,
    QK_JOYSTICK_BUTTON_14 = 0x740E,
    QK_JOYSTICK_BUTTON_15 = 0x740F,
    QK_JOYSTICK_BUTTON_16 = 0x7410,
    QK_JOYSTICK_BUTTON_17 = 0x7411,
    QK_JOYSTICK_BUTTON_18 = 0x7412,
    QK_JOYSTICK_BUTTON_19 = 0x7413,
    QK_JOYSTICK_BUTTON_20 = 0x7414,
    QK_JOYSTICK_BUTTON_21 = 0x7415,
    QK_JOYSTICK_BUTTON_22 = 0x7416,
    QK_JOYSTICK_BUTTON_23 = 0x7417,
    QK_JOYSTICK_BUTTON_24 = 0x7418,
    QK_JOYSTICK_BUTTON_25 = 0x7419,
    QK_JOYSTICK_BUTTON_26 = 0x741A,
    QK_JOYSTICK_BUTTON_27 = 0x741B,
    QK_JOYSTICK_BUTTON_28 = 0x741C,
    QK_JOYSTICK_BUTTON_29 = 0x741D,
    QK_JOYSTICK_BUTTON_30 = 0x741E,
    QK_JOYSTICK_BUTTON_31 = 0x741F,
    // QK_JOYSTICK end

    // QK_PROGRAMMABLE_BUTTON start
    QK_PROGRAMMABLE_BUTTON_1 = 0x7440,
    QK_PROGRAMMABLE_BUTTON_2 = 0x7441,
    QK_PROGRAMMABLE_BUTTON_3 = 0x7442,
    QK_PROGRAMMABLE_BUTTON_4 = 0x7443,
    QK_PROGRAMMABLE_BUTTON_5 = 0x7444,
    QK_PROGRAMMABLE_BUTTON_6 = 0x7445,
    QK_PROGRAMMABLE_BUTTON_7 = 0x7446,
    QK_PROGRAMMABLE_BUTTON_8 = 0x7447,
    QK_PROGRAMMABLE_BUTTON_9 = 0x7448,
    QK_PROGRAMMABLE_BUTTON_10 = 0x7449,
    QK_PROGRAMMABLE_BUTTON_11 = 0x744A,
    QK_PROGRAMMABLE_BUTTON_12 = 0x744B,
    QK_PROGRAMMABLE_BUTTON_13 = 0x744C,
    QK_PROGRAMMABLE_BUTTON_14 = 0x744D,
    QK_PROGRAMMABLE_BUTTON_15 = 0x744E,
    QK_PROGRAMMABLE_BUTTON_16 = 0x744F,
    QK_PROGRAMMABLE_BUTTON_17 = 0x7450,
    QK_PROGRAMMABLE_BUTTON_18 = 0x7451,
    QK_PROGRAMMABLE_BUTTON_19 = 0x7452,
    QK_PROGRAMMABLE_BUTTON_20 = 0x7453,
    QK_PROGRAMMABLE_BUTTON_21 = 0x7454,
    QK_PROGRAMMABLE_BUTTON_22 = 0x7455,
    QK_PROGRAMMABLE_BUTTON_23 = 0x7456,
    QK_PROGRAMMABLE_BUTTON_24 = 0x7457,
    QK_PROGRAMMABLE_BUTTON_25 = 0x7458,
    QK_PROGRAMMABLE_BUTTON_26 = 0x7459,
    QK_PROGRAMMABLE_BUTTON_27 = 0x745A,
    QK_PROGRAMMABLE_BUTTON_28 = 0x745B,
    QK_PROGRAMMABLE_BUTTON_29 = 0x745C,
    QK_PROGRAMMABLE_BUTTON_30 = 0x745D,
    QK_PROGRAMMABLE_BUTTON_31 = 0x745E,
    QK_PROGRAMMABLE_BUTTON_32 = 0x745F,
    // QK_PROGRAMMABLE_BUTTON end

    // QK_AUDIO start
    QK_AUDIO_ON = 0x7480,
    QK_AUDIO_OFF = 0x7481,
    QK_AUDIO_TOGGLE = 0x7482,
    QK_AUDIO_CLICKY_TOGGLE = 0x748A,
    QK_AUDIO_CLICKY_ON = 0x748B,
    QK_AUDIO_CLICKY_OFF = 0x748C,
    QK_AUDIO_CLICKY_UP = 0x748D,
    QK_AUDIO_CLICKY_DOWN = 0x748E,
    QK_AUDIO_CLICKY_RESET = 0x748F,
    QK_MUSIC_ON = 0x7490,
    QK_MUSIC_OFF = 0x7491,
    QK_MUSIC_TOGGLE = 0x7492,
    QK_MUSIC_MODE_NEXT = 0x7493,
    QK_AUDIO_VOICE_NEXT = 0x7494,
    QK_AUDIO_VOICE_PREVIOUS = 0x7495,
    // QK_AUDIO end

    // QK_STENO start
    QK_STENO_BOLT = 0x74F0,
    QK_STENO_GEMINI = 0x74F1,
    QK_STENO_COMB = 0x74F2,
    QK_STENO_COMB_MAX = 0x74FC,
    // QK_STENO end

    // QK_MACRO start
    QK_MACRO_0 = 0x7700,
    QK_MACRO_1 = 0x7701,
    QK_MACRO_2 = 0x7702,
    QK_MACRO_3 = 0x7703,
    QK_MACRO_4 = 0x7704,
    QK_MACRO_5 = 0x7705,
    QK_MACRO_6 = 0x7706,
    QK_MACRO_7 = 0x7707,
    QK_MACRO_8 = 0x7708,
    QK_MACRO_9 = 0x7709,
    QK_MACRO_10 = 0x770A,
    QK_MACRO_11 = 0x770B,
    QK_MACRO_12 = 0x770C,
    QK_MACRO_13 = 0x770D,
    QK_MACRO_14 = 0x770E,
    QK_MACRO_15 = 0x770F,
    QK_MACRO_16 = 0x7710,
    QK_MACRO_17 = 0x7711,
    QK_MACRO_18 = 0x7712,
    QK_MACRO_19 = 0x7713,
    QK_MACRO_20 = 0x7714,
    QK_MACRO_21 = 0x7715,
    QK_MACRO_22 = 0x7716,
    QK_MACRO_23 = 0x7717,
    QK_MACRO_24 = 0x7718,
    QK_MACRO_25 = 0x7719,
    QK_MACRO_26 = 0x771A,
    QK_MACRO_27 = 0x771B,
    QK_MACRO_28 = 0x771C,
    QK_MACRO_29 = 0x771D,
    QK_MACRO_30 = 0x771E,
    QK_MACRO_31 = 0x771F,
    // QK_MACRO end

    // QK_LIGHTING start
    QK_BACKLIGHT_ON = 0x7800,
    QK_BACKLIGHT_OFF = 0x7801,
    QK_BACKLIGHT_TOGGLE = 0x7802,
    QK_BACKLIGHT_DOWN = 0x7803,
    QK_BACKLIGHT_UP = 0x7804,
    QK_BACKLIGHT_STEP = 0x7805,
    QK_BACKLIGHT_TOGGLE_BREATHING = 0x7806,
    // TODO/Note: In QMK, these RGB_* keycodes are shared for `rgblight` and `rgb_matrix`
    // (corresponding to rumcake's underglow and rgb-backlight-matrix). We will only make these
    // keycodes work for underglow for the time being. So, controlling colors (and speed) on
    // RGB backlighting matrices using QMK keycodes will not be possible for now.
    RGB_TOG = 0x7820,
    RGB_MODE_FORWARD = 0x7821,
    RGB_MODE_REVERSE = 0x7822,
    RGB_HUI = 0x7823,
    RGB_HUD = 0x7824,
    RGB_SAI = 0x7825,
    RGB_SAD = 0x7826,
    RGB_VAI = 0x7827,
    RGB_VAD = 0x7828,
    RGB_SPI = 0x7829,
    RGB_SPD = 0x782A,
    RGB_MODE_PLAIN = 0x782B,
    RGB_MODE_BREATHE = 0x782C,
    RGB_MODE_RAINBOW = 0x782D,
    RGB_MODE_SWIRL = 0x782E,
    RGB_MODE_SNAKE = 0x782F,
    RGB_MODE_KNIGHT = 0x7830,
    RGB_MODE_XMAS = 0x7831,
    RGB_MODE_GRADIENT = 0x7832,
    RGB_MODE_RGBTEST = 0x7833,
    RGB_MODE_TWINKLE = 0x7834,
    // QK_LIGHTING end

    // QK_QUANTUM start
    QK_BOOTLOADER = 0x7C00,
    QK_REBOOT = 0x7C01,
    QK_DEBUG_TOGGLE = 0x7C02,
    QK_CLEAR_EEPROM = 0x7C03,
    QK_MAKE = 0x7C04,
    QK_AUTO_SHIFT_DOWN = 0x7C10,
    QK_AUTO_SHIFT_UP = 0x7C11,
    QK_AUTO_SHIFT_REPORT = 0x7C12,
    QK_AUTO_SHIFT_ON = 0x7C13,
    QK_AUTO_SHIFT_OFF = 0x7C14,
    QK_AUTO_SHIFT_TOGGLE = 0x7C15,
    QK_GRAVE_ESCAPE = 0x7C16,
    QK_VELOCIKEY_TOGGLE = 0x7C17,
    QK_SPACE_CADET_LEFT_CTRL_PARENTHESIS_OPEN = 0x7C18,
    QK_SPACE_CADET_RIGHT_CTRL_PARENTHESIS_CLOSE = 0x7C19,
    QK_SPACE_CADET_LEFT_SHIFT_PARENTHESIS_OPEN = 0x7C1A,
    QK_SPACE_CADET_RIGHT_SHIFT_PARENTHESIS_CLOSE = 0x7C1B,
    QK_SPACE_CADET_LEFT_ALT_PARENTHESIS_OPEN = 0x7C1C,
    QK_SPACE_CADET_RIGHT_ALT_PARENTHESIS_CLOSE = 0x7C1D,
    QK_SPACE_CADET_RIGHT_SHIFT_ENTER = 0x7C1E,
    QK_OUTPUT_AUTO = 0x7C20,
    QK_OUTPUT_USB = 0x7C21,
    QK_OUTPUT_BLUETOOTH = 0x7C22,
    QK_UNICODE_MODE_NEXT = 0x7C30,
    QK_UNICODE_MODE_PREVIOUS = 0x7C31,
    QK_UNICODE_MODE_MACOS = 0x7C32,
    QK_UNICODE_MODE_LINUX = 0x7C33,
    QK_UNICODE_MODE_WINDOWS = 0x7C34,
    QK_UNICODE_MODE_BSD = 0x7C35,
    QK_UNICODE_MODE_WINCOMPOSE = 0x7C36,
    QK_UNICODE_MODE_EMACS = 0x7C37,
    QK_HAPTIC_ON = 0x7C40,
    QK_HAPTIC_OFF = 0x7C41,
    QK_HAPTIC_TOGGLE = 0x7C42,
    QK_HAPTIC_RESET = 0x7C43,
    QK_HAPTIC_FEEDBACK_TOGGLE = 0x7C44,
    QK_HAPTIC_BUZZ_TOGGLE = 0x7C45,
    QK_HAPTIC_MODE_NEXT = 0x7C46,
    QK_HAPTIC_MODE_PREVIOUS = 0x7C47,
    QK_HAPTIC_CONTINUOUS_TOGGLE = 0x7C48,
    QK_HAPTIC_CONTINUOUS_UP = 0x7C49,
    QK_HAPTIC_CONTINUOUS_DOWN = 0x7C4A,
    QK_HAPTIC_DWELL_UP = 0x7C4B,
    QK_HAPTIC_DWELL_DOWN = 0x7C4C,
    QK_COMBO_ON = 0x7C50,
    QK_COMBO_OFF = 0x7C51,
    QK_COMBO_TOGGLE = 0x7C52,
    QK_DYNAMIC_MACRO_RECORD_START_1 = 0x7C53,
    QK_DYNAMIC_MACRO_RECORD_START_2 = 0x7C54,
    QK_DYNAMIC_MACRO_RECORD_STOP = 0x7C55,
    QK_DYNAMIC_MACRO_PLAY_1 = 0x7C56,
    QK_DYNAMIC_MACRO_PLAY_2 = 0x7C57,
    QK_LEADER = 0x7C58,
    QK_LOCK = 0x7C59,
    QK_ONE_SHOT_ON = 0x7C5A,
    QK_ONE_SHOT_OFF = 0x7C5B,
    QK_ONE_SHOT_TOGGLE = 0x7C5C,
    QK_KEY_OVERRIDE_TOGGLE = 0x7C5D,
    QK_KEY_OVERRIDE_ON = 0x7C5E,
    QK_KEY_OVERRIDE_OFF = 0x7C5F,
    QK_SECURE_LOCK = 0x7C60,
    QK_SECURE_UNLOCK = 0x7C61,
    QK_SECURE_TOGGLE = 0x7C62,
    QK_SECURE_REQUEST = 0x7C63,
    QK_DYNAMIC_TAPPING_TERM_PRINT = 0x7C70,
    QK_DYNAMIC_TAPPING_TERM_UP = 0x7C71,
    QK_DYNAMIC_TAPPING_TERM_DOWN = 0x7C72,
    QK_CAPS_WORD_TOGGLE = 0x7C73,
    QK_AUTOCORRECT_ON = 0x7C74,
    QK_AUTOCORRECT_OFF = 0x7C75,
    QK_AUTOCORRECT_TOGGLE = 0x7C76,
    QK_TRI_LAYER_LOWER = 0x7C77,
    QK_TRI_LAYER_UPPER = 0x7C78,
    QK_REPEAT_KEY = 0x7C79,
    QK_ALT_REPEAT_KEY = 0x7C7A,
    // QK_QUANTUM end

    // QK_KB start
    // This range is used if you use `customKeycodes` in your JSON definition
    // https://github.com/the-via/app/blob/ee4443bbdcad79a9568d43488e5097a9c6d96bbe/src/components/panes/configure-panes/keycode.tsx#L183
    // https://github.com/the-via/app/blob/ee4443bbdcad79a9568d43488e5097a9c6d96bbe/src/utils/advanced-keys.ts#L78
    // https://github.com/the-via/app/blob/ee4443bbdcad79a9568d43488e5097a9c6d96bbe/src/utils/key.ts#L975
    QK_KB_0 = 0x7E00, // Also currently aliased as SAFE_RANGE: https://github.com/qmk/qmk_firmware/pull/19697
    QK_KB_1 = 0x7E01,
    QK_KB_2 = 0x7E02,
    QK_KB_3 = 0x7E03,
    QK_KB_4 = 0x7E04,
    QK_KB_5 = 0x7E05,
    QK_KB_6 = 0x7E06,
    QK_KB_7 = 0x7E07,
    QK_KB_8 = 0x7E08,
    QK_KB_9 = 0x7E09,
    QK_KB_10 = 0x7E0A,
    QK_KB_11 = 0x7E0B,
    QK_KB_12 = 0x7E0C,
    QK_KB_13 = 0x7E0D,
    QK_KB_14 = 0x7E0E,
    QK_KB_15 = 0x7E0F,
    QK_KB_16 = 0x7E10,
    QK_KB_17 = 0x7E11,
    QK_KB_18 = 0x7E12,
    QK_KB_19 = 0x7E13,
    QK_KB_20 = 0x7E14,
    QK_KB_21 = 0x7E15,
    QK_KB_22 = 0x7E16,
    QK_KB_23 = 0x7E17,
    QK_KB_24 = 0x7E18,
    QK_KB_25 = 0x7E19,
    QK_KB_26 = 0x7E1A,
    QK_KB_27 = 0x7E1B,
    QK_KB_28 = 0x7E1C,
    QK_KB_29 = 0x7E1D,
    QK_KB_30 = 0x7E1E,
    QK_KB_31 = 0x7E1F,
    // QK_KB end

    // QK_USER start
    // QK_USER_0 is formerly known as USER00 in older versions of the protocol. Also known as
    // SAFE_RANGE in Vial QMK:
    // https://github.com/vial-kb/vial-qmk/blob/9caabddf4cfa187d2a77ee27e896d25c30a6c125/quantum/quantum_keycodes.h#L37.
    // Uses of USER00 were changed to QK_KB_0 starting from protocol 12:
    // https://github.com/qmk/qmk_firmware/pull/19697
    // For the purposes of converting `customKeycodes` in the JSON definition, both the Via and Vial app use QK_KB_0.
    // https://github.com/vial-kb/vial-gui/blob/f5ef91fc30915c6d6946f86d80bd2711e7de4463/src/main/python/keycodes/keycodes_v6.py#L579
    // https://github.com/vial-kb/vial-gui/blob/f5ef91fc30915c6d6946f86d80bd2711e7de4463/src/main/python/keycodes/keycodes.py#L805C4-L805C4
    QK_USER_0 = 0x7E40,
    QK_USER_1 = 0x7E41,
    QK_USER_2 = 0x7E42,
    QK_USER_3 = 0x7E43,
    QK_USER_4 = 0x7E44,
    QK_USER_5 = 0x7E45,
    QK_USER_6 = 0x7E46,
    QK_USER_7 = 0x7E47,
    QK_USER_8 = 0x7E48,
    QK_USER_9 = 0x7E49,
    QK_USER_10 = 0x7E4A,
    QK_USER_11 = 0x7E4B,
    QK_USER_12 = 0x7E4C,
    QK_USER_13 = 0x7E4D,
    QK_USER_14 = 0x7E4E,
    QK_USER_15 = 0x7E4F,
    QK_USER_16 = 0x7E50,
    QK_USER_17 = 0x7E51,
    QK_USER_18 = 0x7E52,
    QK_USER_19 = 0x7E53,
    QK_USER_20 = 0x7E54,
    QK_USER_21 = 0x7E55,
    QK_USER_22 = 0x7E56,
    QK_USER_23 = 0x7E57,
    QK_USER_24 = 0x7E58,
    QK_USER_25 = 0x7E59,
    QK_USER_26 = 0x7E5A,
    QK_USER_27 = 0x7E5B,
    QK_USER_28 = 0x7E5C,
    QK_USER_29 = 0x7E5D,
    QK_USER_30 = 0x7E5E,
    QK_USER_31 = 0x7E5F,
    // QK_USER end

    // QK_UNICODEMAP = 0x8000,
    // QK_UNICODEMAP_MAX = 0xBFFF,

    // QK_UNICODE = 0x8000,
    // QK_UNICODE_MAX = 0xFFFF,

    // QK_UNICODEMAP_PAIR = 0xC000,
    // QK_UNICODEMAP_PAIR_MAX = 0xFFFF,
}

/// This variant doesn't exist in QMK. We introduce it in rumcake to handle actions that can
/// not be converted to a 16-bit QMK keycode. This is used in convert_action_to_keycode. So when we
/// try to send keycodes back to the VIA app, or When the dynamic keymap is saved to a storage
/// peripheral, any actions that are unable to be converted to a QMK keycode will be represented as
/// 0xFFFF. More importantly, when we restore the layout from storage, any time we encounter the
/// 0xFFFF keycode, we can avoid overwriting the corresponding action in the rumcake layout.
/// Without this, any actions that can't be represented as a QMK keycode would not work after
/// restoring a dynamic keymap from storage.
const UNKNOWN_KEYCODE: u16 = 0xFFFF;

/// This function converts an action to a QMK-compatible keycode. It converts to u16 instead of
/// implementing `From` for QMKKeycodes, because there are some keycodes that fall between certain
/// QMKKeycodeRanges, but don't exist in the QMKKeycodes enum. Any actions that can not be
/// converted to a keycode are converted to the [`UNKNOWN_KEYCODE`].
///
/// Rule of thumb for conversions: if there exists a QMKKeycode enum value that falls in between a
/// given QMKKeycodeRanges, then we will use all of the QMKKeycode enum values in that range, and
/// no more. If a QMKKeycode enum value does not exist for a given QMKKeycodeRanges, then we will
/// derive the possible keycodes based on the start and end value of the range.
///
/// For example, for QK_DEF_LAYER, there are no corresponding QMKKeycodes enum values that fall in
/// that range, so we will derive the u16 keycode based on the start and end value of the range.
/// For QK_USER, there exists 32 QMKKeycodes enum values that fall in that range. Even though the
/// QK_USER range spans more than 32 possible values (0x7E40 to 0x7FFF), we will only use the codes
/// that are defined as enum values in QMKKeycodes (QK_USER_0 to QK_USER_31), and no more.
pub(crate) fn convert_action_to_keycode(action: Action<Keycode>) -> u16 {
    match action {
        Action::NoOp => QMKKeycodes::KC_NO as u16,
        Action::Trans => QMKKeycodes::KC_TRANSPARENT as u16,
        Action::KeyCode(key) => (num::FromPrimitive::from_u8(key as u8) as Option<QMKKeycodes>)
            .map_or(UNKNOWN_KEYCODE, |key| key as u16),
        Action::Layer(layer) => {
            if (layer as u16)
                <= QMKKeycodeRanges::QK_MOMENTARY_MAX as u16 - QMKKeycodeRanges::QK_MOMENTARY as u16
            {
                QMKKeycodeRanges::QK_MOMENTARY as u16 + layer as u16
            } else {
                UNKNOWN_KEYCODE
            }
        }
        Action::DefaultLayer(layer) => {
            if (layer as u16)
                <= QMKKeycodeRanges::QK_DEF_LAYER_MAX as u16 - QMKKeycodeRanges::QK_DEF_LAYER as u16
            {
                QMKKeycodeRanges::QK_DEF_LAYER as u16 + layer as u16
            } else {
                UNKNOWN_KEYCODE
            }
        }
        Action::HoldTap(_) => todo!(),
        Action::Custom(key) => match key {
            Keycode::Custom(id) => {
                if id as u16 <= 31 {
                    QMKKeycodeRanges::QK_KB as u16 + id as u16
                } else {
                    UNKNOWN_KEYCODE
                }
            }
            #[cfg(feature = "underglow")]
            Keycode::Underglow(command) => match command {
                crate::underglow::animations::UnderglowCommand::Toggle => {
                    QMKKeycodes::RGB_TOG as u16
                }
                crate::underglow::animations::UnderglowCommand::NextEffect => {
                    QMKKeycodes::RGB_MODE_FORWARD as u16
                }
                crate::underglow::animations::UnderglowCommand::PrevEffect => {
                    QMKKeycodes::RGB_MODE_REVERSE as u16
                }
                crate::underglow::animations::UnderglowCommand::SetEffect(effect) => match effect {
                    crate::underglow::animations::UnderglowEffect::Solid => {
                        QMKKeycodes::RGB_MODE_PLAIN as u16
                    }
                    crate::underglow::animations::UnderglowEffect::Breathing => {
                        QMKKeycodes::RGB_MODE_BREATHE as u16
                    }
                    crate::underglow::animations::UnderglowEffect::RainbowMood => {
                        QMKKeycodes::RGB_MODE_RAINBOW as u16
                    }
                    crate::underglow::animations::UnderglowEffect::RainbowSwirl => {
                        QMKKeycodes::RGB_MODE_SWIRL as u16
                    }
                    crate::underglow::animations::UnderglowEffect::Snake => {
                        QMKKeycodes::RGB_MODE_SNAKE as u16
                    }
                    crate::underglow::animations::UnderglowEffect::Knight => {
                        QMKKeycodes::RGB_MODE_KNIGHT as u16
                    }
                    crate::underglow::animations::UnderglowEffect::Christmas => {
                        QMKKeycodes::RGB_MODE_XMAS as u16
                    }
                    crate::underglow::animations::UnderglowEffect::StaticGradient => {
                        QMKKeycodes::RGB_MODE_GRADIENT as u16
                    }
                    crate::underglow::animations::UnderglowEffect::RGBTest => {
                        QMKKeycodes::RGB_MODE_RGBTEST as u16
                    }
                    crate::underglow::animations::UnderglowEffect::Twinkle => {
                        QMKKeycodes::RGB_MODE_TWINKLE as u16
                    }
                    _ => UNKNOWN_KEYCODE,
                },
                crate::underglow::animations::UnderglowCommand::AdjustHue(hue) => {
                    if hue.is_positive() {
                        return QMKKeycodes::RGB_HUI as u16;
                    }

                    if hue.is_negative() {
                        return QMKKeycodes::RGB_HUD as u16;
                    }

                    UNKNOWN_KEYCODE
                }
                crate::underglow::animations::UnderglowCommand::AdjustSaturation(sat) => {
                    if sat.is_positive() {
                        return QMKKeycodes::RGB_SAI as u16;
                    }

                    if sat.is_negative() {
                        return QMKKeycodes::RGB_SAD as u16;
                    }

                    UNKNOWN_KEYCODE
                }
                crate::underglow::animations::UnderglowCommand::AdjustValue(val) => {
                    if val.is_positive() {
                        return QMKKeycodes::RGB_VAI as u16;
                    }

                    if val.is_negative() {
                        return QMKKeycodes::RGB_VAD as u16;
                    }

                    UNKNOWN_KEYCODE
                }
                crate::underglow::animations::UnderglowCommand::AdjustSpeed(speed) => {
                    if speed.is_positive() {
                        return QMKKeycodes::RGB_SPI as u16;
                    }

                    if speed.is_negative() {
                        return QMKKeycodes::RGB_SPD as u16;
                    }

                    UNKNOWN_KEYCODE
                }
                _ => UNKNOWN_KEYCODE,
            },
            #[cfg(any(
                feature = "simple-backlight",
                feature = "simple-backlight-matrix",
                feature = "rgb-backlight-matrix"
            ))]
            Keycode::Backlight(command) => match command {
                crate::backlight::animations::BacklightCommand::TurnOn => {
                    QMKKeycodes::QK_BACKLIGHT_ON as u16
                }
                crate::backlight::animations::BacklightCommand::TurnOff => {
                    QMKKeycodes::QK_BACKLIGHT_OFF as u16
                }
                crate::backlight::animations::BacklightCommand::Toggle => {
                    QMKKeycodes::QK_BACKLIGHT_TOGGLE as u16
                }
                crate::backlight::animations::BacklightCommand::NextEffect => {
                    QMKKeycodes::QK_BACKLIGHT_STEP as u16
                }
                crate::backlight::animations::BacklightCommand::AdjustValue(val) => {
                    if val.is_positive() {
                        return QMKKeycodes::QK_BACKLIGHT_UP as u16;
                    }

                    if val.is_negative() {
                        return QMKKeycodes::QK_BACKLIGHT_DOWN as u16;
                    }

                    UNKNOWN_KEYCODE
                }
                crate::backlight::animations::BacklightCommand::AdjustSpeed(speed) => {
                    if speed.is_positive() {
                        return QMKKeycodes::RGB_SPI as u16;
                    }

                    if speed.is_negative() {
                        return QMKKeycodes::RGB_SPD as u16;
                    }

                    UNKNOWN_KEYCODE
                }
                // Note: AdjustHue and AdjustSaturation is not handled for RGB matrices. See the note on line 679
                _ => UNKNOWN_KEYCODE,
            },
            #[cfg(feature = "bluetooth")]
            Keycode::Bluetooth(command) => match command {
                #[cfg(feature = "usb")]
                crate::bluetooth::BluetoothCommand::OutputUSB => QMKKeycodes::QK_OUTPUT_USB as u16,
                #[cfg(feature = "usb")]
                crate::bluetooth::BluetoothCommand::OutputBluetooth => {
                    QMKKeycodes::QK_OUTPUT_BLUETOOTH as u16
                }
                _ => UNKNOWN_KEYCODE,
            },
        },
        _ => UNKNOWN_KEYCODE,
    }
}

pub(crate) fn convert_keycode_to_action(keycode: u16) -> Option<Action<Keycode>> {
    if keycode == QMKKeycodes::KC_NO as u16 {
        return Some(Action::NoOp);
    }

    if keycode == QMKKeycodes::KC_TRANSPARENT as u16 {
        return Some(Action::Trans);
    }

    if QMKKeycodeRanges::QK_BASIC as u16 <= keycode
        && keycode <= QMKKeycodeRanges::QK_BASIC_MAX as u16
    {
        // TODO: handle consumer-related keycodes in this range. They don't exist in keyberon's
        // enum, but do exist in QMK (see the enum above)
        return num::FromPrimitive::from_u16(keycode).map(Action::KeyCode);
    }

    if QMKKeycodeRanges::QK_MOMENTARY as u16 <= keycode
        && keycode <= QMKKeycodeRanges::QK_MOMENTARY_MAX as u16
    {
        return Some(Action::Layer(
            (keycode - QMKKeycodeRanges::QK_MOMENTARY as u16) as usize,
        ));
    }

    if QMKKeycodeRanges::QK_DEF_LAYER as u16 <= keycode
        && keycode <= QMKKeycodeRanges::QK_DEF_LAYER_MAX as u16
    {
        return Some(Action::DefaultLayer(
            (keycode - QMKKeycodeRanges::QK_DEF_LAYER as u16) as usize,
        ));
    }

    if QMKKeycodeRanges::QK_LIGHTING as u16 <= keycode
        && keycode <= QMKKeycodeRanges::QK_LIGHTING_MAX as u16
    {
        #[cfg(any(
            feature = "simple-backlight",
            feature = "simple-backlight-matrix",
            feature = "rgb-backlight-matrix"
        ))]
        {
            if keycode == QMKKeycodes::QK_BACKLIGHT_ON as u16 {
                return Some(Action::Custom(Keycode::Backlight(
                    crate::backlight::animations::BacklightCommand::TurnOff,
                )));
            }

            if keycode == QMKKeycodes::QK_BACKLIGHT_OFF as u16 {
                return Some(Action::Custom(Keycode::Backlight(
                    crate::backlight::animations::BacklightCommand::TurnOff,
                )));
            }

            if keycode == QMKKeycodes::QK_BACKLIGHT_TOGGLE as u16 {
                return Some(Action::Custom(Keycode::Backlight(
                    crate::backlight::animations::BacklightCommand::Toggle,
                )));
            }

            if keycode == QMKKeycodes::QK_BACKLIGHT_DOWN as u16 {
                return Some(Action::Custom(Keycode::Backlight(
                    crate::backlight::animations::BacklightCommand::AdjustValue(-17),
                )));
            }

            if keycode == QMKKeycodes::QK_BACKLIGHT_UP as u16 {
                return Some(Action::Custom(Keycode::Backlight(
                    crate::backlight::animations::BacklightCommand::AdjustValue(17),
                )));
            }

            if keycode == QMKKeycodes::QK_BACKLIGHT_STEP as u16 {
                return Some(Action::Custom(Keycode::Backlight(
                    crate::backlight::animations::BacklightCommand::NextEffect,
                )));
            }
        }

        #[cfg(feature = "underglow")]
        {
            if keycode == QMKKeycodes::RGB_TOG as u16 {
                return Some(Action::Custom(Keycode::Underglow(
                    crate::underglow::animations::UnderglowCommand::Toggle,
                )));
            }

            if keycode == QMKKeycodes::RGB_MODE_FORWARD as u16 {
                return Some(Action::Custom(Keycode::Underglow(
                    crate::underglow::animations::UnderglowCommand::NextEffect,
                )));
            }

            if keycode == QMKKeycodes::RGB_MODE_REVERSE as u16 {
                return Some(Action::Custom(Keycode::Underglow(
                    crate::underglow::animations::UnderglowCommand::PrevEffect,
                )));
            }

            if keycode == QMKKeycodes::RGB_HUI as u16 {
                return Some(Action::Custom(Keycode::Underglow(
                    crate::underglow::animations::UnderglowCommand::AdjustHue(17),
                )));
            }

            if keycode == QMKKeycodes::RGB_HUD as u16 {
                return Some(Action::Custom(Keycode::Underglow(
                    crate::underglow::animations::UnderglowCommand::AdjustHue(-17),
                )));
            }

            if keycode == QMKKeycodes::RGB_SAI as u16 {
                return Some(Action::Custom(Keycode::Underglow(
                    crate::underglow::animations::UnderglowCommand::AdjustSaturation(17),
                )));
            }

            if keycode == QMKKeycodes::RGB_SAD as u16 {
                return Some(Action::Custom(Keycode::Underglow(
                    crate::underglow::animations::UnderglowCommand::AdjustSaturation(-17),
                )));
            }

            if keycode == QMKKeycodes::RGB_VAI as u16 {
                return Some(Action::Custom(Keycode::Underglow(
                    crate::underglow::animations::UnderglowCommand::AdjustValue(17),
                )));
            }

            if keycode == QMKKeycodes::RGB_VAD as u16 {
                return Some(Action::Custom(Keycode::Underglow(
                    crate::underglow::animations::UnderglowCommand::AdjustValue(-17),
                )));
            }

            if keycode == QMKKeycodes::RGB_SPI as u16 {
                return Some(Action::Custom(Keycode::Underglow(
                    crate::underglow::animations::UnderglowCommand::AdjustSpeed(17),
                )));
            }

            if keycode == QMKKeycodes::RGB_SPD as u16 {
                return Some(Action::Custom(Keycode::Underglow(
                    crate::underglow::animations::UnderglowCommand::AdjustSpeed(-17),
                )));
            }

            if keycode == QMKKeycodes::RGB_MODE_PLAIN as u16 {
                return Some(Action::Custom(Keycode::Underglow(
                    crate::underglow::animations::UnderglowCommand::SetEffect(
                        crate::underglow::animations::UnderglowEffect::Solid,
                    ),
                )));
            }

            if keycode == QMKKeycodes::RGB_MODE_BREATHE as u16 {
                return Some(Action::Custom(Keycode::Underglow(
                    crate::underglow::animations::UnderglowCommand::SetEffect(
                        crate::underglow::animations::UnderglowEffect::Breathing,
                    ),
                )));
            }

            if keycode == QMKKeycodes::RGB_MODE_RAINBOW as u16 {
                return Some(Action::Custom(Keycode::Underglow(
                    crate::underglow::animations::UnderglowCommand::SetEffect(
                        crate::underglow::animations::UnderglowEffect::RainbowMood,
                    ),
                )));
            }

            if keycode == QMKKeycodes::RGB_MODE_SWIRL as u16 {
                return Some(Action::Custom(Keycode::Underglow(
                    crate::underglow::animations::UnderglowCommand::SetEffect(
                        crate::underglow::animations::UnderglowEffect::RainbowSwirl,
                    ),
                )));
            }

            if keycode == QMKKeycodes::RGB_MODE_SNAKE as u16 {
                return Some(Action::Custom(Keycode::Underglow(
                    crate::underglow::animations::UnderglowCommand::SetEffect(
                        crate::underglow::animations::UnderglowEffect::Snake,
                    ),
                )));
            }

            if keycode == QMKKeycodes::RGB_MODE_KNIGHT as u16 {
                return Some(Action::Custom(Keycode::Underglow(
                    crate::underglow::animations::UnderglowCommand::SetEffect(
                        crate::underglow::animations::UnderglowEffect::Knight,
                    ),
                )));
            }

            if keycode == QMKKeycodes::RGB_MODE_XMAS as u16 {
                return Some(Action::Custom(Keycode::Underglow(
                    crate::underglow::animations::UnderglowCommand::SetEffect(
                        crate::underglow::animations::UnderglowEffect::Christmas,
                    ),
                )));
            }

            if keycode == QMKKeycodes::RGB_MODE_GRADIENT as u16 {
                return Some(Action::Custom(Keycode::Underglow(
                    crate::underglow::animations::UnderglowCommand::SetEffect(
                        crate::underglow::animations::UnderglowEffect::StaticGradient,
                    ),
                )));
            }

            if keycode == QMKKeycodes::RGB_MODE_RGBTEST as u16 {
                return Some(Action::Custom(Keycode::Underglow(
                    crate::underglow::animations::UnderglowCommand::SetEffect(
                        crate::underglow::animations::UnderglowEffect::RGBTest,
                    ),
                )));
            }

            if keycode == QMKKeycodes::RGB_MODE_TWINKLE as u16 {
                return Some(Action::Custom(Keycode::Underglow(
                    crate::underglow::animations::UnderglowCommand::SetEffect(
                        crate::underglow::animations::UnderglowEffect::Twinkle,
                    ),
                )));
            }
        }
    }

    if QMKKeycodeRanges::QK_QUANTUM as u16 <= keycode
        && keycode <= QMKKeycodeRanges::QK_QUANTUM as u16
    {
        #[cfg(all(feature = "usb", feature = "bluetooth"))]
        if keycode == QMKKeycodes::QK_OUTPUT_USB as u16 {
            return Some(Action::Custom(Keycode::Bluetooth(
                crate::bluetooth::BluetoothCommand::OutputUSB,
            )));
        }

        #[cfg(all(feature = "usb", feature = "bluetooth"))]
        if keycode == QMKKeycodes::QK_OUTPUT_BLUETOOTH as u16 {
            return Some(Action::Custom(Keycode::Bluetooth(
                crate::bluetooth::BluetoothCommand::OutputBluetooth,
            )));
        }
    }

    if QMKKeycodeRanges::QK_KB as u16 <= keycode && keycode <= QMKKeycodeRanges::QK_KB_MAX as u16 {
        return Some(Action::Custom(Keycode::Custom(
            (keycode - QMKKeycodeRanges::QK_KB as u16) as u8,
        )));
    }

    None
}
