use defmt::warn;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;
use keyberon::action::Action;
use keyberon::key_code::KeyCode;

use crate::keyboard::Keycode;

use super::ViaKeyboard;

pub fn get_protocol_version(version: u16, data: &mut [u8]) {
    data[0..=1].copy_from_slice(&version.to_be_bytes());
}

pub fn get_uptime(data: &mut [u8]) {
    data[0..=3].copy_from_slice(&((embassy_time::Instant::now().as_millis() as u32).to_be_bytes()));
}

pub async fn get_switch_matrix_state<K: ViaKeyboard>(
    matrix_state: &[u8; (K::LAYOUT_COLS + u8::BITS as usize - 1) / u8::BITS as usize
         * K::LAYOUT_ROWS],
    data: &mut [u8],
) {
    // see [`crate::via::protocol::background_task`] to see how `matrix_state` is created.
    data[..(matrix_state.len())].copy_from_slice(matrix_state)
}

pub fn get_firmware_version<K: ViaKeyboard>(data: &mut [u8]) {
    data[0..=3].copy_from_slice(&K::VIA_FIRMWARE_VERSION.to_be_bytes());
}

pub async fn get_layout_options<K: ViaKeyboard>(layout_options: &u32, data: &mut [u8]) {
    data[(4 - K::VIA_EEPROM_LAYOUT_OPTIONS_SIZE)..=3]
        .copy_from_slice(&layout_options.to_be_bytes()[(4 - K::VIA_EEPROM_LAYOUT_OPTIONS_SIZE)..=3])
}

pub async fn set_layout_options<K: ViaKeyboard>(layout_options: &mut u32, data: &[u8]) {
    let mut bytes = [0; 4];
    bytes[(4 - K::VIA_EEPROM_LAYOUT_OPTIONS_SIZE)..]
        .copy_from_slice(&data[2..(2 + K::VIA_EEPROM_LAYOUT_OPTIONS_SIZE)]);
    *layout_options = u32::from_be_bytes(bytes);
    K::handle_set_layout_options(*layout_options);

    #[cfg(feature = "storage")]
    super::storage::update_data(
        super::storage::ViaStorageKeys::LayoutOptions,
        0,
        &data[0..K::VIA_EEPROM_LAYOUT_OPTIONS_SIZE],
    )
    .await;
}

pub async fn device_indication() {
    #[cfg(feature = "simple-backlight")]
    crate::backlight::simple_backlight::BACKLIGHT_COMMAND_CHANNEL
        .send(crate::backlight::simple_backlight::animations::BacklightCommand::Toggle)
        .await;

    #[cfg(feature = "simple-backlight-matrix")]
    crate::backlight::simple_backlight_matrix::BACKLIGHT_COMMAND_CHANNEL
        .send(crate::backlight::simple_backlight_matrix::animations::BacklightCommand::Toggle)
        .await;

    #[cfg(feature = "rgb-backlight-matrix")]
    crate::backlight::rgb_backlight_matrix::BACKLIGHT_COMMAND_CHANNEL
        .send(crate::backlight::rgb_backlight_matrix::animations::BacklightCommand::Toggle)
        .await;

    #[cfg(feature = "underglow")]
    crate::underglow::UNDERGLOW_COMMAND_CHANNEL
        .send(crate::underglow::animations::UnderglowCommand::Toggle)
        .await;
}

pub async fn eeprom_reset() {
    #[cfg(feature = "storage")]
    super::storage::reset_data().await;
}

pub(super) static BOOTLOADER_JUMP_SIGNAL: Signal<ThreadModeRawMutex, ()> = Signal::new();

pub fn bootloader_jump() {
    BOOTLOADER_JUMP_SIGNAL.signal(());
}

pub fn dynamic_keymap_macro_get_count<K: ViaKeyboard>(data: &mut [u8]) {
    data[0] = K::DYNAMIC_KEYMAP_MACRO_COUNT;
}

pub fn dynamic_keymap_macro_reset<K: ViaKeyboard + 'static>()
where
    [(); K::DYNAMIC_KEYMAP_MACRO_BUFFER_SIZE as usize]:,
    [(); K::DYNAMIC_KEYMAP_MACRO_COUNT as usize]:,
{
    if let Some(macro_data) = K::get_macro_buffer() {
        macro_data.buffer.fill(0)
    };
}

pub fn dynamic_keymap_macro_get_buffer_size<K: ViaKeyboard>(data: &mut [u8]) {
    data[0..=1].copy_from_slice(&K::DYNAMIC_KEYMAP_MACRO_BUFFER_SIZE.to_be_bytes());
}

pub async fn dynamic_keymap_macro_get_buffer<K: ViaKeyboard + 'static>(
    offset: u16,
    size: u8,
    data: &mut [u8],
) where
    [(); K::DYNAMIC_KEYMAP_MACRO_BUFFER_SIZE as usize]:,
    [(); K::DYNAMIC_KEYMAP_MACRO_COUNT as usize]:,
{
    let len = if offset as usize + size as usize > K::DYNAMIC_KEYMAP_MACRO_BUFFER_SIZE as usize {
        (K::DYNAMIC_KEYMAP_MACRO_BUFFER_SIZE as usize).saturating_sub(offset as usize)
    } else {
        size as usize
    };

    if let Some(macro_data) = K::get_macro_buffer() {
        data[..len].copy_from_slice(&macro_data.buffer[(offset as usize)..(offset as usize + len)]);
    };
}

pub async fn dynamic_keymap_macro_set_buffer<K: ViaKeyboard + 'static>(
    offset: u16,
    size: u8,
    data: &[u8],
) where
    [(); K::DYNAMIC_KEYMAP_MACRO_BUFFER_SIZE as usize]:,
    [(); K::DYNAMIC_KEYMAP_MACRO_COUNT as usize]:,
{
    let len = if offset as usize + size as usize > K::DYNAMIC_KEYMAP_MACRO_BUFFER_SIZE as usize {
        (K::DYNAMIC_KEYMAP_MACRO_BUFFER_SIZE as usize).saturating_sub(offset as usize)
    } else {
        size as usize
    };

    if let Some(macro_data) = K::get_macro_buffer() {
        macro_data.update_buffer(offset as usize, &data[..len]);
    }

    #[cfg(feature = "storage")]
    super::storage::update_data(
        super::storage::ViaStorageKeys::DynamicKeymapMacro,
        offset as usize,
        &data[..len],
    )
    .await;
}

pub fn dynamic_keymap_get_layer_count<K: ViaKeyboard>(data: &mut [u8]) {
    data[0] = K::DYNAMIC_KEYMAP_LAYER_COUNT as u8;
}

pub async fn dynamic_keymap_get_keycode<K: ViaKeyboard + 'static>(
    layer: u8,
    row: u8,
    col: u8,
    data: &mut [u8],
    convert_action_to_keycode: impl Fn(Action<Keycode>) -> u16,
) where
    [(); K::LAYERS]:,
    [(); K::LAYOUT_ROWS]:,
    [(); K::LAYOUT_COLS]:,
{
    let keycodes_bytes = &mut data[0..=1];

    if !(layer as usize >= K::DYNAMIC_KEYMAP_LAYER_COUNT
        || row as usize >= K::LAYOUT_ROWS
        || col as usize >= K::LAYOUT_COLS)
    {
        if let Some(action) = K::get_layout()
            .lock()
            .await
            .get_action((row, col), layer as usize)
        {
            keycodes_bytes.copy_from_slice(&convert_action_to_keycode(action).to_be_bytes())
        };
    } else {
        warn!("[VIA] Requested a dynamic keymap keycode that is out of bounds.")
    }
}

pub async fn dynamic_keymap_set_keycode<K: ViaKeyboard + 'static>(
    layer: u8,
    row: u8,
    col: u8,
    data: &[u8],
    convert_keycode_to_action: impl Fn(u16) -> Option<Action<Keycode>>,
) where
    [(); K::LAYERS]:,
    [(); K::LAYOUT_ROWS]:,
    [(); K::LAYOUT_COLS]:,
{
    let keycode = &data[0..=1];

    if !(layer as usize >= K::DYNAMIC_KEYMAP_LAYER_COUNT
        || row as usize >= K::LAYOUT_ROWS
        || col as usize >= K::LAYOUT_COLS)
    {
        {
            let mut layout = K::get_layout().lock().await;
            if let Some(action) =
                convert_keycode_to_action(u16::from_be_bytes(keycode.try_into().unwrap()))
            {
                layout
                    .change_action((row, col), layer as usize, action)
                    .unwrap();
            }
        }

        #[cfg(feature = "storage")]
        {
            let keycode_offset = ((layer * K::LAYOUT_ROWS as u8 * K::LAYOUT_COLS as u8 * 2)
                + (row * K::LAYOUT_COLS as u8 * 2)
                + (col * 2)) as usize;

            super::storage::update_data(
                super::storage::ViaStorageKeys::DynamicKeymap,
                keycode_offset,
                keycode,
            )
            .await;
        }
    } else {
        warn!("[VIA] Requested a dynamic keymap keycode that is out of bounds.")
    }
}

pub async fn dynamic_keymap_get_encoder<K: ViaKeyboard>(
    layer: u8,
    encoder_id: u8,
    clockwise: bool,
    data: &mut [u8],
) {
    let keycode = &mut data[0..=1];

    let keycode_offset = ((layer * K::NUM_ENCODERS as u8 * 2 * 2)
        + (encoder_id * 2 * 2)
        + if clockwise { 0 } else { 2 }) as usize;

    if !(layer as usize >= K::DYNAMIC_KEYMAP_LAYER_COUNT || encoder_id as usize >= K::NUM_ENCODERS)
    {
        //TODO: encoder support
    } else {
        warn!("[VIA] Requested a dynamic keymap encoder that is out of bounds.")
    }
}

pub async fn dynamic_keymap_set_encoder<K: ViaKeyboard>(
    layer: u8,
    encoder_id: u8,
    clockwise: bool,
    data: &[u8],
) {
    let keycode = &data[0..=1];

    let keycode_offset = ((layer * K::NUM_ENCODERS as u8 * 2 * 2)
        + (encoder_id * 2 * 2)
        + if clockwise { 0 } else { 2 }) as usize;

    if !(layer as usize >= K::DYNAMIC_KEYMAP_LAYER_COUNT || encoder_id as usize >= K::NUM_ENCODERS)
    {
        //TODO: encoder support
        #[cfg(feature = "storage")]
        super::storage::update_data(
            super::storage::ViaStorageKeys::DynamicKeymapEncoder,
            keycode_offset,
            keycode,
        )
        .await;
    } else {
        warn!("[VIA] Attempted to set a dynamic keymap encoder out of bounds.")
    }
}

pub async fn dynamic_keymap_get_buffer<K: ViaKeyboard + 'static>(
    offset: u16,
    size: u8,
    data: &mut [u8],
    convert_action_to_keycode: impl Fn(Action<Keycode>) -> u16,
) where
    [(); K::LAYERS]:,
    [(); K::LAYOUT_ROWS]:,
    [(); K::LAYOUT_COLS]:,
{
    let buffer_size = K::DYNAMIC_KEYMAP_LAYER_COUNT * K::LAYOUT_ROWS * K::LAYOUT_COLS * 2;

    let len = if offset as usize + size as usize > buffer_size {
        buffer_size.saturating_sub(offset as usize)
    } else {
        size as usize
    };

    let mut layout = K::get_layout().lock().await;

    // We make the assumption that Via will never request for a buffer that requires us to send
    // part a 2-byte keycode (so a partial keycode). In other words, we assume that `offset` and
    // `size` will always be even.
    // https://github.com/the-via/app/blob/ee4443bbdcad79a9568d43488e5097a9c6d96bbe/src/utils/keyboard-api.ts#L249
    for byte in ((offset as usize)..(offset as usize + len)).step_by(2) {
        let layer = byte / (K::LAYOUT_ROWS * K::LAYOUT_COLS * 2);
        let row = (byte / (K::LAYOUT_COLS * 2)) % K::LAYOUT_ROWS;
        let col = (byte / 2) % K::LAYOUT_COLS;

        data[(byte - offset as usize)..(byte - offset as usize + 2)].copy_from_slice(
            &convert_action_to_keycode(layout.get_action((row as u8, col as u8), layer).unwrap())
                .to_be_bytes(),
        );
    }
}

pub async fn dynamic_keymap_set_buffer<K: ViaKeyboard + 'static>(
    offset: u16,
    size: u8,
    data: &[u8],
    convert_keycode_to_action: impl Fn(u16) -> Option<Action<Keycode>>,
) where
    [(); K::LAYERS]:,
    [(); K::LAYOUT_ROWS]:,
    [(); K::LAYOUT_COLS]:,
{
    let buffer_size = K::DYNAMIC_KEYMAP_LAYER_COUNT * K::LAYOUT_ROWS * K::LAYOUT_COLS * 2;

    let len = if offset as usize + size as usize > buffer_size {
        buffer_size.saturating_sub(offset as usize)
    } else {
        size as usize
    };

    {
        let mut layout = K::get_layout().lock().await;

        // We make the assumption that VIA will never write a buffer that contains part a 2-byte
        // keycode (so a partial keycode). In other words, we assume that `offset` and `size` will
        // always be even.
        // https://github.com/the-via/app/blob/ee4443bbdcad79a9568d43488e5097a9c6d96bbe/src/utils/keyboard-api.ts#L295
        for byte in ((offset as usize)..(offset as usize + len)).step_by(2) {
            if let Some(action) = convert_keycode_to_action(u16::from_be_bytes(
                data[(byte - offset as usize)..(byte - offset as usize + 2)]
                    .try_into()
                    .unwrap(),
            )) {
                let layer = byte / (K::LAYOUT_ROWS * K::LAYOUT_COLS * 2);
                let row = (byte / (K::LAYOUT_COLS * 2)) % K::LAYOUT_ROWS;
                let col = (byte / 2) % K::LAYOUT_COLS;

                layout
                    .change_action((row as u8, col as u8), layer, action)
                    .unwrap();
            }
        }
    }

    #[cfg(feature = "storage")]
    {
        super::storage::update_data(
            super::storage::ViaStorageKeys::DynamicKeymap,
            offset as usize,
            &data[..len],
        )
        .await;
    }
}

pub async fn dynamic_keymap_reset<K: ViaKeyboard + 'static>()
where
    [(); K::LAYERS]:,
    [(); K::LAYOUT_ROWS]:,
    [(); K::LAYOUT_COLS]:,
{
    let mut layout = K::get_layout().lock().await;
    let original = K::get_original_layout();

    for (layer_idx, layer) in original.iter().enumerate() {
        for (row_idx, row) in layer.iter().enumerate() {
            for (col_idx, action) in row.iter().enumerate() {
                layout
                    .change_action((row_idx as u8, col_idx as u8), layer_idx, *action)
                    .unwrap();
            }
        }
    }
}

#[cfg(feature = "simple-backlight")]
pub async fn simple_backlight_get_enabled(data: &mut [u8]) {
    data[0] = crate::backlight::simple_backlight::BACKLIGHT_CONFIG_STATE
        .get()
        .await
        .enabled as u8
}

#[cfg(feature = "simple-backlight-matrix")]
pub async fn simple_backlight_matrix_get_enabled(data: &mut [u8]) {
    data[0] = crate::backlight::simple_backlight_matrix::BACKLIGHT_CONFIG_STATE
        .get()
        .await
        .enabled as u8
}

#[cfg(feature = "rgb-backlight-matrix")]
pub async fn rgb_backlight_matrix_get_enabled(data: &mut [u8]) {
    data[0] = crate::backlight::rgb_backlight_matrix::BACKLIGHT_CONFIG_STATE
        .get()
        .await
        .enabled as u8
}

#[cfg(feature = "simple-backlight")]
pub async fn simple_backlight_set_enabled(data: &[u8]) {
    let command = if data[0] == 1 {
        crate::backlight::simple_backlight::animations::BacklightCommand::TurnOn
    } else {
        crate::backlight::simple_backlight::animations::BacklightCommand::TurnOff
    };

    crate::backlight::simple_backlight::BACKLIGHT_COMMAND_CHANNEL
        .send(command)
        .await;
}

#[cfg(feature = "simple-backlight-matrix")]
pub async fn simple_backlight_matrix_set_enabled(data: &[u8]) {
    let command = if data[0] == 1 {
        crate::backlight::simple_backlight_matrix::animations::BacklightCommand::TurnOn
    } else {
        crate::backlight::simple_backlight_matrix::animations::BacklightCommand::TurnOff
    };

    crate::backlight::simple_backlight_matrix::BACKLIGHT_COMMAND_CHANNEL
        .send(command)
        .await;
}

#[cfg(feature = "rgb-backlight-matrix")]
pub async fn rgb_backlight_matrix_set_enabled(data: &[u8]) {
    let command = if data[0] == 1 {
        crate::backlight::rgb_backlight_matrix::animations::BacklightCommand::TurnOn
    } else {
        crate::backlight::rgb_backlight_matrix::animations::BacklightCommand::TurnOff
    };

    crate::backlight::rgb_backlight_matrix::BACKLIGHT_COMMAND_CHANNEL
        .send(command)
        .await;
}

#[cfg(feature = "simple-backlight")]
pub async fn simple_backlight_get_brightness(data: &mut [u8]) {
    data[0] = crate::backlight::simple_backlight::BACKLIGHT_CONFIG_STATE
        .get()
        .await
        .val
}

#[cfg(feature = "simple-backlight-matrix")]
pub async fn simple_backlight_matrix_get_brightness(data: &mut [u8]) {
    data[0] = crate::backlight::simple_backlight_matrix::BACKLIGHT_CONFIG_STATE
        .get()
        .await
        .val
}

#[cfg(feature = "rgb-backlight-matrix")]
pub async fn rgb_backlight_matrix_get_brightness(data: &mut [u8]) {
    data[0] = crate::backlight::rgb_backlight_matrix::BACKLIGHT_CONFIG_STATE
        .get()
        .await
        .val
}

#[cfg(feature = "simple-backlight")]
pub async fn simple_backlight_set_brightness(data: &[u8]) {
    crate::backlight::simple_backlight::BACKLIGHT_COMMAND_CHANNEL
        .send(crate::backlight::simple_backlight::animations::BacklightCommand::SetValue(data[0]))
        .await;
}

#[cfg(feature = "simple-backlight-matrix")]
pub async fn simple_backlight_matrix_set_brightness(data: &[u8]) {
    crate::backlight::simple_backlight_matrix::BACKLIGHT_COMMAND_CHANNEL
        .send(
            crate::backlight::simple_backlight_matrix::animations::BacklightCommand::SetValue(
                data[0],
            ),
        )
        .await;
}

#[cfg(feature = "rgb-backlight-matrix")]
pub async fn rgb_backlight_matrix_set_brightness(data: &[u8]) {
    crate::backlight::rgb_backlight_matrix::BACKLIGHT_COMMAND_CHANNEL
        .send(
            crate::backlight::rgb_backlight_matrix::animations::BacklightCommand::SetValue(data[0]),
        )
        .await;
}

#[cfg(feature = "simple-backlight")]
pub async fn simple_backlight_get_effect(
    data: &mut [u8],
    convert_effect_to_qmk_id: impl Fn(
        crate::backlight::simple_backlight::animations::BacklightEffect,
    ) -> u8,
) {
    data[0] = convert_effect_to_qmk_id(
        crate::backlight::simple_backlight::BACKLIGHT_CONFIG_STATE
            .get()
            .await
            .effect,
    )
}

#[cfg(feature = "simple-backlight-matrix")]
pub async fn simple_backlight_matrix_get_effect(
    data: &mut [u8],
    convert_effect_to_qmk_id: impl Fn(
        crate::backlight::simple_backlight_matrix::animations::BacklightEffect,
    ) -> u8,
) {
    data[0] = convert_effect_to_qmk_id(
        crate::backlight::simple_backlight_matrix::BACKLIGHT_CONFIG_STATE
            .get()
            .await
            .effect,
    )
}

#[cfg(feature = "rgb-backlight-matrix")]
pub async fn rgb_backlight_matrix_get_effect(
    data: &mut [u8],
    convert_effect_to_qmk_id: impl Fn(
        crate::backlight::rgb_backlight_matrix::animations::BacklightEffect,
    ) -> u8,
) {
    data[0] = convert_effect_to_qmk_id(
        crate::backlight::rgb_backlight_matrix::BACKLIGHT_CONFIG_STATE
            .get()
            .await
            .effect,
    )
}

#[cfg(feature = "simple-backlight")]
pub async fn simple_backlight_set_effect(
    data: &[u8],
    convert_qmk_id_to_effect: impl Fn(
        u8,
    ) -> Option<
        crate::backlight::simple_backlight::animations::BacklightEffect,
    >,
) {
    if let Some(effect) = convert_qmk_id_to_effect(data[0]) {
        crate::backlight::simple_backlight::BACKLIGHT_COMMAND_CHANNEL
            .send(
                crate::backlight::simple_backlight::animations::BacklightCommand::SetEffect(effect),
            )
            .await;
    } else {
        warn!(
            "[VIA] Tried to set an unknown backlight effect: {:?}",
            data[0]
        )
    }
}

#[cfg(feature = "simple-backlight-matrix")]
pub async fn simple_backlight_matrix_set_effect(
    data: &[u8],
    convert_qmk_id_to_effect: impl Fn(
        u8,
    ) -> Option<
        crate::backlight::simple_backlight_matrix::animations::BacklightEffect,
    >,
) {
    if let Some(effect) = convert_qmk_id_to_effect(data[0]) {
        crate::backlight::simple_backlight_matrix::BACKLIGHT_COMMAND_CHANNEL
            .send(
                crate::backlight::simple_backlight_matrix::animations::BacklightCommand::SetEffect(
                    effect,
                ),
            )
            .await;
    } else {
        warn!(
            "[VIA] Tried to set an unknown backlight effect: {:?}",
            data[0]
        )
    }
}

#[cfg(feature = "rgb-backlight-matrix")]
pub async fn rgb_backlight_matrix_set_effect(
    data: &[u8],
    convert_qmk_id_to_effect: impl Fn(
        u8,
    ) -> Option<
        crate::backlight::rgb_backlight_matrix::animations::BacklightEffect,
    >,
) {
    if let Some(effect) = convert_qmk_id_to_effect(data[0]) {
        crate::backlight::rgb_backlight_matrix::BACKLIGHT_COMMAND_CHANNEL
            .send(
                crate::backlight::rgb_backlight_matrix::animations::BacklightCommand::SetEffect(
                    effect,
                ),
            )
            .await;
    } else {
        warn!(
            "[VIA] Tried to set an unknown backlight effect: {:?}",
            data[0]
        )
    }
}

#[cfg(feature = "simple-backlight")]
pub async fn simple_backlight_get_speed(data: &mut [u8]) {
    data[0] = crate::backlight::simple_backlight::BACKLIGHT_CONFIG_STATE
        .get()
        .await
        .speed
}

#[cfg(feature = "simple-backlight-matrix")]
pub async fn simple_backlight_matrix_get_speed(data: &mut [u8]) {
    data[0] = crate::backlight::simple_backlight_matrix::BACKLIGHT_CONFIG_STATE
        .get()
        .await
        .speed
}

#[cfg(feature = "rgb-backlight-matrix")]
pub async fn rgb_backlight_matrix_get_speed(data: &mut [u8]) {
    data[0] = crate::backlight::rgb_backlight_matrix::BACKLIGHT_CONFIG_STATE
        .get()
        .await
        .speed
}

#[cfg(feature = "simple-backlight")]
pub async fn simple_backlight_set_speed(data: &[u8]) {
    crate::backlight::simple_backlight::BACKLIGHT_COMMAND_CHANNEL
        .send(crate::backlight::simple_backlight::animations::BacklightCommand::SetSpeed(data[0]))
        .await;
}

#[cfg(feature = "simple-backlight-matrix")]
pub async fn simple_backlight_matrix_set_speed(data: &[u8]) {
    crate::backlight::simple_backlight_matrix::BACKLIGHT_COMMAND_CHANNEL
        .send(
            crate::backlight::simple_backlight_matrix::animations::BacklightCommand::SetSpeed(
                data[0],
            ),
        )
        .await;
}

#[cfg(feature = "rgb-backlight-matrix")]
pub async fn rgb_backlight_matrix_set_speed(data: &[u8]) {
    crate::backlight::rgb_backlight_matrix::BACKLIGHT_COMMAND_CHANNEL
        .send(
            crate::backlight::rgb_backlight_matrix::animations::BacklightCommand::SetSpeed(data[0]),
        )
        .await;
}

#[cfg(feature = "rgb-backlight-matrix")]
pub async fn rgb_backlight_matrix_get_color(data: &mut [u8]) {
    // Color only available on RGB matrices
    let config = crate::backlight::rgb_backlight_matrix::BACKLIGHT_CONFIG_STATE
        .get()
        .await;
    data[0] = config.hue;
    data[1] = config.sat;
}

#[cfg(feature = "rgb-backlight-matrix")]
pub async fn rgb_backlight_matrix_set_color(data: &[u8]) {
    // Color only available on RGB matrices
    crate::backlight::rgb_backlight_matrix::BACKLIGHT_COMMAND_CHANNEL
        .send(crate::backlight::rgb_backlight_matrix::animations::BacklightCommand::SetHue(data[0]))
        .await;
    crate::backlight::rgb_backlight_matrix::BACKLIGHT_COMMAND_CHANNEL
        .send(
            crate::backlight::rgb_backlight_matrix::animations::BacklightCommand::SetSaturation(
                data[1],
            ),
        )
        .await;
}

pub async fn simple_backlight_save() {
    #[cfg(all(feature = "storage", feature = "simple-backlight"))]
    crate::backlight::simple_backlight::BACKLIGHT_COMMAND_CHANNEL
        .send(crate::backlight::simple_backlight::animations::BacklightCommand::SaveConfig)
        .await;
}

pub async fn simple_backlight_matrix_save() {
    #[cfg(all(feature = "storage", feature = "simple-backlight-matrix"))]
    crate::backlight::simple_backlight_matrix::BACKLIGHT_COMMAND_CHANNEL
        .send(crate::backlight::simple_backlight_matrix::animations::BacklightCommand::SaveConfig)
        .await;
}

pub async fn rgb_backlight_matrix_save() {
    #[cfg(all(feature = "storage", feature = "rgb-backlight-matrix"))]
    crate::backlight::rgb_backlight_matrix::BACKLIGHT_COMMAND_CHANNEL
        .send(crate::backlight::rgb_backlight_matrix::animations::BacklightCommand::SaveConfig)
        .await;
}

#[cfg(feature = "underglow")]
pub async fn underglow_get_enabled(data: &mut [u8]) {
    data[0] = crate::underglow::UNDERGLOW_CONFIG_STATE.get().await.enabled as u8
}

#[cfg(feature = "underglow")]
pub async fn underglow_set_enabled(data: &[u8]) {
    let command = if data[0] == 1 {
        crate::underglow::animations::UnderglowCommand::TurnOn
    } else {
        crate::underglow::animations::UnderglowCommand::TurnOff
    };

    crate::underglow::UNDERGLOW_COMMAND_CHANNEL
        .send(command)
        .await;
}

#[cfg(feature = "underglow")]
pub async fn underglow_get_brightness(data: &mut [u8]) {
    data[0] = crate::underglow::UNDERGLOW_CONFIG_STATE.get().await.val;
}

#[cfg(feature = "underglow")]
pub async fn underglow_set_brightness(data: &[u8]) {
    crate::underglow::UNDERGLOW_COMMAND_CHANNEL
        .send(crate::underglow::animations::UnderglowCommand::SetValue(
            data[0],
        ))
        .await;
}

#[cfg(feature = "underglow")]
pub async fn underglow_get_effect(
    data: &mut [u8],
    convert_effect_to_qmk_id: impl Fn(crate::underglow::animations::UnderglowConfig) -> u8,
) {
    data[0] = convert_effect_to_qmk_id(crate::underglow::UNDERGLOW_CONFIG_STATE.get().await);
}

#[cfg(feature = "underglow")]
/// `convert_qmk_id_to_effect` must return Option<(UnderglowEffect, Option<u8>)> because some QMK
/// underglow effect IDs represent the same overall effect, but differ only by speed or direction.
/// e.g. RainbowSwirl to RainbowSwirl6. Option<u8> is used to set the speed for the effect to
/// handle these cases. This only really applies to Vial, since it uses an older protocol. In the
/// new Via protocol, we never set the speed, and instead use a custom UI to control speed.
pub async fn underglow_set_effect(
    data: &[u8],
    convert_qmk_id_to_effect: impl Fn(
        u8,
    ) -> Option<(
        crate::underglow::animations::UnderglowEffect,
        Option<u8>,
    )>,
) {
    if let Some((effect, speed)) = convert_qmk_id_to_effect(data[0]) {
        crate::underglow::UNDERGLOW_COMMAND_CHANNEL
            .send(crate::underglow::animations::UnderglowCommand::SetEffect(
                effect,
            ))
            .await;

        if let Some(speed) = speed {
            crate::underglow::UNDERGLOW_COMMAND_CHANNEL
                .send(crate::underglow::animations::UnderglowCommand::SetSpeed(
                    speed,
                ))
                .await;
        }
    } else {
        warn!(
            "[VIA] Tried to set an unknown underglow effect: {:?}",
            data[0]
        )
    }
}

#[cfg(feature = "underglow")]
pub async fn underglow_get_speed(data: &mut [u8]) {
    data[0] = crate::underglow::UNDERGLOW_CONFIG_STATE.get().await.speed;
}

#[cfg(feature = "underglow")]
pub async fn underglow_set_speed(data: &[u8]) {
    crate::underglow::UNDERGLOW_COMMAND_CHANNEL
        .send(crate::underglow::animations::UnderglowCommand::SetSpeed(
            data[0],
        ))
        .await;
}

#[cfg(feature = "underglow")]
pub async fn underglow_get_color(data: &mut [u8]) {
    let config = crate::underglow::UNDERGLOW_CONFIG_STATE.get().await;
    data[0] = config.hue;
    data[1] = config.sat;
}

#[cfg(feature = "underglow")]
pub async fn underglow_set_color(data: &[u8]) {
    crate::underglow::UNDERGLOW_COMMAND_CHANNEL
        .send(crate::underglow::animations::UnderglowCommand::SetHue(
            data[0],
        ))
        .await;
    crate::underglow::UNDERGLOW_COMMAND_CHANNEL
        .send(crate::underglow::animations::UnderglowCommand::SetSaturation(data[1]))
        .await;
}

pub async fn underglow_save() {
    #[cfg(all(feature = "storage", feature = "underglow"))]
    crate::underglow::UNDERGLOW_COMMAND_CHANNEL
        .send(crate::underglow::animations::UnderglowCommand::SaveConfig)
        .await;
}
