use defmt::warn;
use embassy_sync::signal::Signal;
use keyberon::action::Action;

use crate::hw::platform::RawMutex;
use crate::keyboard::{KeyboardLayout, Keycode};
use crate::storage::{FlashStorage, StorageDevice, StorageKey};

use super::ViaKeyboard;

pub fn get_protocol_version(version: u16, data: &mut [u8]) {
    data[0..=1].copy_from_slice(&version.to_be_bytes());
}

pub fn get_uptime(data: &mut [u8]) {
    data[0..=3].copy_from_slice(&((embassy_time::Instant::now().as_millis() as u32).to_be_bytes()));
}

pub async fn get_switch_matrix_state<K: ViaKeyboard>(
    matrix_state: &[u8; (<K::Layout as KeyboardLayout>::LAYOUT_COLS + u8::BITS as usize - 1)
         / u8::BITS as usize
         * <K::Layout as KeyboardLayout>::LAYOUT_ROWS],
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

pub async fn set_layout_options<K: ViaKeyboard + 'static>(layout_options: &mut u32, data: &[u8])
where
    [(); <<K::StorageType as StorageDevice>::FlashStorageType as FlashStorage>::ERASE_SIZE]:,
{
    let mut bytes = [0; 4];
    bytes[(4 - K::VIA_EEPROM_LAYOUT_OPTIONS_SIZE)..]
        .copy_from_slice(&data[2..(2 + K::VIA_EEPROM_LAYOUT_OPTIONS_SIZE)]);
    *layout_options = u32::from_be_bytes(bytes);
    K::handle_set_layout_options(*layout_options);

    if let Some(database) = K::get_storage_service() {
        // Update data
        // For layout options, we just overwrite all of the old data
        if let Err(()) = database
            .write_raw(
                StorageKey::LayoutOptions,
                &data[..K::VIA_EEPROM_LAYOUT_OPTIONS_SIZE],
            )
            .await
        {
            warn!("[VIA] Could not write layout options.")
        };
    }
}

pub async fn device_indication<K: ViaKeyboard>() {
    #[cfg(feature = "simple-backlight")]
    if let Some(channel) = <<K::Layout as KeyboardLayout>::SimpleBacklightDeviceType as crate::lighting::simple_backlight::private::MaybeSimpleBacklightDevice>::get_command_channel() {
        channel
            .send(crate::lighting::simple_backlight::SimpleBacklightCommand::Toggle)
            .await;
    }

    #[cfg(feature = "simple-backlight-matrix")]
    if let Some(channel) = <<K::Layout as KeyboardLayout>::SimpleBacklightMatrixDeviceType as crate::lighting::simple_backlight_matrix::private::MaybeSimpleBacklightMatrixDevice>::get_command_channel() {
        channel
            .send(crate::lighting::simple_backlight_matrix::SimpleBacklightMatrixCommand::Toggle)
            .await;
    }

    #[cfg(feature = "rgb-backlight-matrix")]
    if let Some(channel) = <<K::Layout as KeyboardLayout>::RGBBacklightMatrixDeviceType as crate::lighting::rgb_backlight_matrix::private::MaybeRGBBacklightMatrixDevice>::get_command_channel() {
        channel
            .send(crate::lighting::rgb_backlight_matrix::RGBBacklightMatrixCommand::Toggle)
            .await;
    }

    #[cfg(feature = "underglow")]
    if let Some(channel) = <<K::Layout as KeyboardLayout>::UnderglowDeviceType as crate::lighting::underglow::private::MaybeUnderglowDevice>::get_command_channel() {
        channel
            .send(crate::lighting::underglow::UnderglowCommand::Toggle)
            .await;
    }
}

pub async fn eeprom_reset<K: ViaKeyboard + 'static>()
where
    [(); <<K::StorageType as StorageDevice>::FlashStorageType as FlashStorage>::ERASE_SIZE]:,
{
    if let Some(database) = K::get_storage_service() {
        let _ = database.delete(StorageKey::LayoutOptions).await;
        let _ = database.delete(StorageKey::DynamicKeymap).await;
        let _ = database.delete(StorageKey::DynamicKeymapMacro).await;
        let _ = database.delete(StorageKey::DynamicKeymapEncoder).await;
    }
}

pub(super) static BOOTLOADER_JUMP_SIGNAL: Signal<RawMutex, ()> = Signal::new();

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
    [(); <<K::StorageType as StorageDevice>::FlashStorageType as FlashStorage>::ERASE_SIZE]:,
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

    if let Some(database) = K::get_storage_service() {
        let offset = offset as usize;
        let mut buf = [0; K::DYNAMIC_KEYMAP_MACRO_BUFFER_SIZE as usize];

        // Read data
        let stored_len = match database.read_raw(StorageKey::DynamicKeymapMacro).await {
            Ok((stored_data, stored_len)) => {
                buf[..stored_len].copy_from_slice(stored_data);
                stored_len
            }
            Err(()) => {
                warn!("[VIA] Could not read dynamic keymap macro buffer.");
                0 // Assume that there is no data yet
            }
        };

        // Update data
        buf[offset..(offset + len)].copy_from_slice(&data[..len]);

        let new_length = stored_len.max(offset + len);

        if let Err(()) = database
            .write_raw(StorageKey::DynamicKeymapMacro, &buf[..new_length])
            .await
        {
            warn!("[VIA] Could not write dynamic keymap macro buffer.")
        };
    }
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
    [(); <K::Layout as KeyboardLayout>::LAYERS]:,
    [(); <K::Layout as KeyboardLayout>::LAYOUT_ROWS]:,
    [(); <K::Layout as KeyboardLayout>::LAYOUT_COLS]:,
{
    let keycodes_bytes = &mut data[0..=1];

    if !(layer as usize >= K::DYNAMIC_KEYMAP_LAYER_COUNT
        || row as usize >= <K::Layout as KeyboardLayout>::LAYOUT_ROWS
        || col as usize >= <K::Layout as KeyboardLayout>::LAYOUT_COLS)
    {
        if let Some(action) = <K::Layout as KeyboardLayout>::get_layout()
            .layout
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
    [(); <<K::StorageType as StorageDevice>::FlashStorageType as FlashStorage>::ERASE_SIZE]:,
    [(); K::DYNAMIC_KEYMAP_LAYER_COUNT * K::Layout::LAYOUT_COLS * K::Layout::LAYOUT_ROWS * 2]:,
    [(); <K::Layout as KeyboardLayout>::LAYERS]:,
    [(); <K::Layout as KeyboardLayout>::LAYOUT_ROWS]:,
    [(); <K::Layout as KeyboardLayout>::LAYOUT_COLS]:,
{
    let keycode = &data[0..=1];

    if !(layer as usize >= K::DYNAMIC_KEYMAP_LAYER_COUNT
        || row as usize >= <K::Layout as KeyboardLayout>::LAYOUT_ROWS
        || col as usize >= <K::Layout as KeyboardLayout>::LAYOUT_COLS)
    {
        {
            let mut layout = <K::Layout as KeyboardLayout>::get_layout()
                .layout
                .lock()
                .await;
            if let Some(action) =
                convert_keycode_to_action(u16::from_be_bytes(keycode.try_into().unwrap()))
            {
                layout
                    .change_action((row, col), layer as usize, action)
                    .unwrap();
            }
        }

        if let Some(database) = K::get_storage_service() {
            let offset = ((layer
                * <K::Layout as KeyboardLayout>::LAYOUT_ROWS as u8
                * K::Layout::LAYOUT_COLS as u8
                * 2)
                + (row * <K::Layout as KeyboardLayout>::LAYOUT_COLS as u8 * 2)
                + (col * 2)) as usize;

            let mut buf = [0; K::DYNAMIC_KEYMAP_LAYER_COUNT
                * K::Layout::LAYOUT_COLS
                * K::Layout::LAYOUT_ROWS
                * 2];

            // Read data
            match database.read_raw(StorageKey::DynamicKeymap).await {
                Ok((stored_data, stored_len)) => {
                    buf[..stored_len].copy_from_slice(stored_data);
                }
                Err(()) => {
                    warn!("[VIA] Could not read dynamic keymap buffer.");
                }
            };

            // Update data
            buf[offset..(offset + 2)].copy_from_slice(keycode);

            if let Err(()) = database.write_raw(StorageKey::DynamicKeymap, &buf).await {
                warn!("[VIA] Could not write dynamic keymap buffer.",)
            };
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

    let offset = ((layer * <K::Layout as KeyboardLayout>::NUM_ENCODERS as u8 * 2 * 2)
        + (encoder_id * 2 * 2)
        + if clockwise { 0 } else { 2 }) as usize;

    if !(layer as usize >= K::DYNAMIC_KEYMAP_LAYER_COUNT
        || encoder_id as usize >= <K::Layout as KeyboardLayout>::NUM_ENCODERS)
    {
        //TODO: encoder support
    } else {
        warn!("[VIA] Requested a dynamic keymap encoder that is out of bounds.")
    }
}

pub async fn dynamic_keymap_set_encoder<K: ViaKeyboard + 'static>(
    layer: u8,
    encoder_id: u8,
    clockwise: bool,
    data: &[u8],
) where
    [(); <<K::StorageType as StorageDevice>::FlashStorageType as FlashStorage>::ERASE_SIZE]:,
    [(); K::DYNAMIC_KEYMAP_LAYER_COUNT * K::Layout::NUM_ENCODERS * 2 * 2]:,
{
    let keycode = &data[0..=1];

    let offset = ((layer * <K::Layout as KeyboardLayout>::NUM_ENCODERS as u8 * 2 * 2)
        + (encoder_id * 2 * 2)
        + if clockwise { 0 } else { 2 }) as usize;

    if !(layer as usize >= K::DYNAMIC_KEYMAP_LAYER_COUNT
        || encoder_id as usize >= <K::Layout as KeyboardLayout>::NUM_ENCODERS)
    {
        //TODO: encoder support

        if let Some(database) = K::get_storage_service() {
            let mut buf = [0; K::DYNAMIC_KEYMAP_LAYER_COUNT * K::Layout::NUM_ENCODERS * 2 * 2];

            // Read data
            match database.read_raw(StorageKey::DynamicKeymapEncoder).await {
                Ok((stored_data, stored_len)) => {
                    buf[..stored_len].copy_from_slice(stored_data);
                }
                Err(()) => {
                    warn!("[VIA] Could not read dynamic keymap encoder.");
                }
            };

            // Update data
            buf[offset..(offset + 2)].copy_from_slice(keycode);

            if let Err(()) = database
                .write_raw(StorageKey::DynamicKeymapEncoder, &buf)
                .await
            {
                warn!("[VIA] Could not write dynamic keymap encoder.")
            };
        }
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
    [(); <K::Layout as KeyboardLayout>::LAYERS]:,
    [(); <K::Layout as KeyboardLayout>::LAYOUT_ROWS]:,
    [(); <K::Layout as KeyboardLayout>::LAYOUT_COLS]:,
{
    let buffer_size = K::DYNAMIC_KEYMAP_LAYER_COUNT
        * <K::Layout as KeyboardLayout>::LAYOUT_ROWS
        * K::Layout::LAYOUT_COLS
        * 2;

    let len = if offset as usize + size as usize > buffer_size {
        buffer_size.saturating_sub(offset as usize)
    } else {
        size as usize
    };

    let mut layout = <K::Layout as KeyboardLayout>::get_layout()
        .layout
        .lock()
        .await;

    // We make the assumption that Via will never request for a buffer that requires us to send
    // part a 2-byte keycode (so a partial keycode). In other words, we assume that `offset` and
    // `size` will always be even.
    // https://github.com/the-via/app/blob/ee4443bbdcad79a9568d43488e5097a9c6d96bbe/src/utils/keyboard-api.ts#L249
    for byte in ((offset as usize)..(offset as usize + len)).step_by(2) {
        let layer =
            byte / (<K::Layout as KeyboardLayout>::LAYOUT_ROWS * K::Layout::LAYOUT_COLS * 2);
        let row =
            (byte / (<K::Layout as KeyboardLayout>::LAYOUT_COLS * 2)) % K::Layout::LAYOUT_ROWS;
        let col = (byte / 2) % <K::Layout as KeyboardLayout>::LAYOUT_COLS;

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
    [(); <<K::StorageType as StorageDevice>::FlashStorageType as FlashStorage>::ERASE_SIZE]:,
    [(); K::DYNAMIC_KEYMAP_LAYER_COUNT * K::Layout::LAYOUT_COLS * K::Layout::LAYOUT_ROWS * 2]:,
    [(); <K::Layout as KeyboardLayout>::LAYERS]:,
    [(); <K::Layout as KeyboardLayout>::LAYOUT_ROWS]:,
    [(); <K::Layout as KeyboardLayout>::LAYOUT_COLS]:,
{
    let buffer_size = K::DYNAMIC_KEYMAP_LAYER_COUNT
        * <K::Layout as KeyboardLayout>::LAYOUT_ROWS
        * K::Layout::LAYOUT_COLS
        * 2;

    let len = if offset as usize + size as usize > buffer_size {
        buffer_size.saturating_sub(offset as usize)
    } else {
        size as usize
    };

    {
        let mut layout = <K::Layout as KeyboardLayout>::get_layout()
            .layout
            .lock()
            .await;

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
                let layer = byte
                    / (<K::Layout as KeyboardLayout>::LAYOUT_ROWS * K::Layout::LAYOUT_COLS * 2);
                let row = (byte / (<K::Layout as KeyboardLayout>::LAYOUT_COLS * 2))
                    % K::Layout::LAYOUT_ROWS;
                let col = (byte / 2) % <K::Layout as KeyboardLayout>::LAYOUT_COLS;

                layout
                    .change_action((row as u8, col as u8), layer, action)
                    .unwrap();
            }
        }
    }

    if let Some(database) = K::get_storage_service() {
        let offset = offset as usize;
        let mut buf = [0; K::DYNAMIC_KEYMAP_LAYER_COUNT
            * K::Layout::LAYOUT_COLS
            * K::Layout::LAYOUT_ROWS
            * 2];

        // Read data
        match database.read_raw(StorageKey::DynamicKeymap).await {
            Ok((stored_data, stored_len)) => {
                buf[..stored_len].copy_from_slice(stored_data);
            }
            Err(()) => {
                warn!("[VIA] Could not read dynamic keymap buffer.");
            }
        };

        // Update data
        buf[offset..(offset + 2)].copy_from_slice(&data[..len]);

        if let Err(()) = database.write_raw(StorageKey::DynamicKeymap, &buf).await {
            warn!("[VIA] Could not write dynamic keymap buffer.",)
        };
    }
}

pub async fn dynamic_keymap_reset<K: ViaKeyboard + 'static>()
where
    [(); <K::Layout as KeyboardLayout>::LAYERS]:,
    [(); <K::Layout as KeyboardLayout>::LAYOUT_ROWS]:,
    [(); <K::Layout as KeyboardLayout>::LAYOUT_COLS]:,
{
    let mut layout = <K::Layout as KeyboardLayout>::get_layout()
        .layout
        .lock()
        .await;
    let original = <K::Layout as KeyboardLayout>::get_original_layout();

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

#[cfg(feature = "underglow")]
pub async fn underglow_get_enabled<K: ViaKeyboard>(data: &mut [u8]) {
    if let Some(state) =
        <<K::Layout as KeyboardLayout>::UnderglowDeviceType as crate::lighting::underglow::private::MaybeUnderglowDevice>::get_state()
    {
        data[0] = state.get().await.enabled as u8
    }
}

#[cfg(feature = "simple-backlight")]
pub async fn simple_backlight_get_enabled<K: ViaKeyboard>(data: &mut [u8]) {
    if let Some(state) = <<K::Layout as KeyboardLayout>::SimpleBacklightDeviceType as crate::lighting::simple_backlight::private::MaybeSimpleBacklightDevice>::get_state() {
        data[0] = state.get().await.enabled as u8
    }
}

#[cfg(feature = "simple-backlight-matrix")]
pub async fn simple_backlight_matrix_get_enabled<K: ViaKeyboard>(data: &mut [u8]) {
    if let Some(state) = <<K::Layout as KeyboardLayout>::SimpleBacklightMatrixDeviceType as crate::lighting::simple_backlight_matrix::private::MaybeSimpleBacklightMatrixDevice>::get_state() {
        data[0] = state.get().await.enabled as u8
    }
}

#[cfg(feature = "rgb-backlight-matrix")]
pub async fn rgb_backlight_matrix_get_enabled<K: ViaKeyboard>(data: &mut [u8]) {
    if let Some(state) = <<K::Layout as KeyboardLayout>::RGBBacklightMatrixDeviceType as crate::lighting::rgb_backlight_matrix::private::MaybeRGBBacklightMatrixDevice>::get_state() {
        data[0] = state.get().await.enabled as u8
    }
}

#[cfg(feature = "underglow")]
pub async fn underglow_set_enabled<K: ViaKeyboard>(data: &[u8]) {
    if let Some(channel) = <<K::Layout as KeyboardLayout>::UnderglowDeviceType as crate::lighting::underglow::private::MaybeUnderglowDevice>::get_command_channel() {
        let command = if data[0] == 1 {
            crate::lighting::underglow::UnderglowCommand::TurnOn
        } else {
            crate::lighting::underglow::UnderglowCommand::TurnOff
        };

        channel.send(command).await;
    }
}

#[cfg(feature = "simple-backlight")]
pub async fn simple_backlight_set_enabled<K: ViaKeyboard>(data: &[u8]) {
    if let Some(channel) = <<K::Layout as KeyboardLayout>::SimpleBacklightDeviceType as crate::lighting::simple_backlight::private::MaybeSimpleBacklightDevice>::get_command_channel() {
        let command = if data[0] == 1 {
            crate::lighting::simple_backlight::SimpleBacklightCommand::TurnOn
        } else {
            crate::lighting::simple_backlight::SimpleBacklightCommand::TurnOff
        };

        channel.send(command).await;
    }
}

#[cfg(feature = "simple-backlight-matrix")]
pub async fn simple_backlight_matrix_set_enabled<K: ViaKeyboard>(data: &[u8]) {
    if let Some(channel) = <<K::Layout as KeyboardLayout>::SimpleBacklightMatrixDeviceType as crate::lighting::simple_backlight_matrix::private::MaybeSimpleBacklightMatrixDevice>::get_command_channel() {
        let command = if data[0] == 1 {
            crate::lighting::simple_backlight_matrix::SimpleBacklightMatrixCommand::TurnOn
        } else {
            crate::lighting::simple_backlight_matrix::SimpleBacklightMatrixCommand::TurnOff
        };

        channel.send(command).await;
    }
}

#[cfg(feature = "rgb-backlight-matrix")]
pub async fn rgb_backlight_matrix_set_enabled<K: ViaKeyboard>(data: &[u8]) {
    if let Some(channel) = <<K::Layout as KeyboardLayout>::RGBBacklightMatrixDeviceType as crate::lighting::rgb_backlight_matrix::private::MaybeRGBBacklightMatrixDevice>::get_command_channel() {
        let command = if data[0] == 1 {
            crate::lighting::rgb_backlight_matrix::RGBBacklightMatrixCommand::TurnOn
        } else {
            crate::lighting::rgb_backlight_matrix::RGBBacklightMatrixCommand::TurnOff
        };

        channel.send(command).await;
    }
}

#[cfg(feature = "underglow")]
pub async fn underglow_get_brightness<K: ViaKeyboard>(data: &mut [u8]) {
    if let Some(state) =
        <<K::Layout as KeyboardLayout>::UnderglowDeviceType as crate::lighting::underglow::private::MaybeUnderglowDevice>::get_state()
    {
        data[0] = state.get().await.val;
    }
}

#[cfg(feature = "simple-backlight")]
pub async fn simple_backlight_get_brightness<K: ViaKeyboard>(data: &mut [u8]) {
    if let Some(state) = <<K::Layout as KeyboardLayout>::SimpleBacklightDeviceType as crate::lighting::simple_backlight::private::MaybeSimpleBacklightDevice>::get_state() {
        data[0] = state.get().await.val
    }
}

#[cfg(feature = "simple-backlight-matrix")]
pub async fn simple_backlight_matrix_get_brightness<K: ViaKeyboard>(data: &mut [u8]) {
    if let Some(state) = <<K::Layout as KeyboardLayout>::SimpleBacklightMatrixDeviceType as crate::lighting::simple_backlight_matrix::private::MaybeSimpleBacklightMatrixDevice>::get_state() {
        data[0] = state.get().await.val
    }
}

#[cfg(feature = "rgb-backlight-matrix")]
pub async fn rgb_backlight_matrix_get_brightness<K: ViaKeyboard>(data: &mut [u8]) {
    if let Some(state) = <<K::Layout as KeyboardLayout>::RGBBacklightMatrixDeviceType as crate::lighting::rgb_backlight_matrix::private::MaybeRGBBacklightMatrixDevice>::get_state() {
        data[0] = state.get().await.val
    }
}

#[cfg(feature = "underglow")]
pub async fn underglow_set_brightness<K: ViaKeyboard>(data: &[u8]) {
    if let Some(channel) = <<K::Layout as KeyboardLayout>::UnderglowDeviceType as crate::lighting::underglow::private::MaybeUnderglowDevice>::get_command_channel() {
        channel
            .send(crate::lighting::underglow::UnderglowCommand::SetValue(
                data[0],
            ))
            .await;
    }
}

#[cfg(feature = "simple-backlight")]
pub async fn simple_backlight_set_brightness<K: ViaKeyboard>(data: &[u8]) {
    if let Some(channel) = <<K::Layout as KeyboardLayout>::SimpleBacklightDeviceType as crate::lighting::simple_backlight::private::MaybeSimpleBacklightDevice>::get_command_channel() {
        channel
            .send(crate::lighting::simple_backlight::SimpleBacklightCommand::SetValue(data[0]))
            .await;
    }
}

#[cfg(feature = "simple-backlight-matrix")]
pub async fn simple_backlight_matrix_set_brightness<K: ViaKeyboard>(data: &[u8]) {
    if let Some(channel) = <<K::Layout as KeyboardLayout>::SimpleBacklightMatrixDeviceType as crate::lighting::simple_backlight_matrix::private::MaybeSimpleBacklightMatrixDevice>::get_command_channel() {
        channel
            .send(
                crate::lighting::simple_backlight_matrix::SimpleBacklightMatrixCommand::SetValue(
                    data[0],
                ),
            )
            .await;
    }
}

#[cfg(feature = "rgb-backlight-matrix")]
pub async fn rgb_backlight_matrix_set_brightness<K: ViaKeyboard>(data: &[u8]) {
    if let Some(channel) = <<K::Layout as KeyboardLayout>::RGBBacklightMatrixDeviceType as crate::lighting::rgb_backlight_matrix::private::MaybeRGBBacklightMatrixDevice>::get_command_channel() {
        channel
            .send(
                crate::lighting::rgb_backlight_matrix::RGBBacklightMatrixCommand::SetValue(data[0]),
            )
            .await;
    }
}

#[cfg(feature = "underglow")]
pub async fn underglow_get_effect<K: ViaKeyboard>(
    data: &mut [u8],
    convert_effect_to_qmk_id: impl Fn(crate::lighting::underglow::UnderglowConfig) -> u8,
) {
    if let Some(state) =
        <<K::Layout as KeyboardLayout>::UnderglowDeviceType as crate::lighting::underglow::private::MaybeUnderglowDevice>::get_state()
    {
        data[0] = convert_effect_to_qmk_id(state.get().await);
    }
}

#[cfg(feature = "simple-backlight")]
pub async fn simple_backlight_get_effect<K: ViaKeyboard>(
    data: &mut [u8],
    convert_effect_to_qmk_id: impl Fn(crate::lighting::simple_backlight::SimpleBacklightEffect) -> u8,
) {
    if let Some(state) = <<K::Layout as KeyboardLayout>::SimpleBacklightDeviceType as crate::lighting::simple_backlight::private::MaybeSimpleBacklightDevice>::get_state() {
        data[0] = convert_effect_to_qmk_id(state.get().await.effect)
    }
}

#[cfg(feature = "simple-backlight-matrix")]
pub async fn simple_backlight_matrix_get_effect<K: ViaKeyboard>(
    data: &mut [u8],
    convert_effect_to_qmk_id: impl Fn(
        crate::lighting::simple_backlight_matrix::SimpleBacklightMatrixEffect,
    ) -> u8,
) {
    if let Some(state) = <<K::Layout as KeyboardLayout>::SimpleBacklightMatrixDeviceType as crate::lighting::simple_backlight_matrix::private::MaybeSimpleBacklightMatrixDevice>::get_state() {
        data[0] = convert_effect_to_qmk_id(state.get().await.effect)
    }
}

#[cfg(feature = "rgb-backlight-matrix")]
pub async fn rgb_backlight_matrix_get_effect<K: ViaKeyboard>(
    data: &mut [u8],
    convert_effect_to_qmk_id: impl Fn(
        crate::lighting::rgb_backlight_matrix::RGBBacklightMatrixEffect,
    ) -> u8,
) {
    if let Some(state) = <<K::Layout as KeyboardLayout>::RGBBacklightMatrixDeviceType as crate::lighting::rgb_backlight_matrix::private::MaybeRGBBacklightMatrixDevice>::get_state() {
        data[0] = convert_effect_to_qmk_id(state.get().await.effect)
    }
}

#[cfg(feature = "underglow")]
/// `convert_qmk_id_to_effect` must return Option<(UnderglowEffect, Option<u8>)> because some QMK
/// underglow effect IDs represent the same overall effect, but differ only by speed or direction.
/// e.g. RainbowSwirl to RainbowSwirl6. Option<u8> is used to set the speed for the effect to
/// handle these cases. This only really applies to Vial, since it uses an older protocol. In the
/// new Via protocol, we never set the speed, and instead use a custom UI to control speed.
pub async fn underglow_set_effect<K: ViaKeyboard>(
    data: &[u8],
    convert_qmk_id_to_effect: impl Fn(
        u8,
    ) -> Option<(
        crate::lighting::underglow::UnderglowEffect,
        Option<u8>,
    )>,
) {
    if let Some(channel) = <<K::Layout as KeyboardLayout>::UnderglowDeviceType as crate::lighting::underglow::private::MaybeUnderglowDevice>::get_command_channel() {
        if let Some((effect, speed)) = convert_qmk_id_to_effect(data[0]) {
            channel
                .send(crate::lighting::underglow::UnderglowCommand::SetEffect(
                    effect,
                ))
                .await;

            if let Some(speed) = speed {
                channel
                    .send(crate::lighting::underglow::UnderglowCommand::SetSpeed(
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
}

#[cfg(feature = "simple-backlight")]
pub async fn simple_backlight_set_effect<K: ViaKeyboard>(
    data: &[u8],
    convert_qmk_id_to_effect: impl Fn(
        u8,
    ) -> Option<
        crate::lighting::simple_backlight::SimpleBacklightEffect,
    >,
) {
    if let Some(channel) = <<K::Layout as KeyboardLayout>::SimpleBacklightDeviceType as crate::lighting::simple_backlight::private::MaybeSimpleBacklightDevice>::get_command_channel() {
        if let Some(effect) = convert_qmk_id_to_effect(data[0]) {
            channel
                .send(crate::lighting::simple_backlight::SimpleBacklightCommand::SetEffect(effect))
                .await;
        } else {
            warn!(
                "[VIA] Tried to set an unknown backlight effect: {:?}",
                data[0]
            )
        }
    }
}

#[cfg(feature = "simple-backlight-matrix")]
pub async fn simple_backlight_matrix_set_effect<K: ViaKeyboard>(
    data: &[u8],
    convert_qmk_id_to_effect: impl Fn(
        u8,
    ) -> Option<
        crate::lighting::simple_backlight_matrix::SimpleBacklightMatrixEffect,
    >,
) {
    if let Some(channel) = <<K::Layout as KeyboardLayout>::SimpleBacklightMatrixDeviceType as crate::lighting::simple_backlight_matrix::private::MaybeSimpleBacklightMatrixDevice>::get_command_channel() {
        if let Some(effect) = convert_qmk_id_to_effect(data[0]) {
            channel
            .send(
                crate::lighting::simple_backlight_matrix::SimpleBacklightMatrixCommand::SetEffect(
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
}

#[cfg(feature = "rgb-backlight-matrix")]
pub async fn rgb_backlight_matrix_set_effect<K: ViaKeyboard>(
    data: &[u8],
    convert_qmk_id_to_effect: impl Fn(
        u8,
    ) -> Option<
        crate::lighting::rgb_backlight_matrix::RGBBacklightMatrixEffect,
    >,
) {
    if let Some(channel) = <<K::Layout as KeyboardLayout>::RGBBacklightMatrixDeviceType as crate::lighting::rgb_backlight_matrix::private::MaybeRGBBacklightMatrixDevice>::get_command_channel() {
        if let Some(effect) = convert_qmk_id_to_effect(data[0]) {
            channel
                .send(
                    crate::lighting::rgb_backlight_matrix::RGBBacklightMatrixCommand::SetEffect(
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
}

#[cfg(feature = "underglow")]
pub async fn underglow_get_speed<K: ViaKeyboard>(data: &mut [u8]) {
    if let Some(state) =
        <<K::Layout as KeyboardLayout>::UnderglowDeviceType as crate::lighting::underglow::private::MaybeUnderglowDevice>::get_state()
    {
        data[0] = state.get().await.speed;
    }
}

#[cfg(feature = "simple-backlight")]
pub async fn simple_backlight_get_speed<K: ViaKeyboard>(data: &mut [u8]) {
    if let Some(state) = <<K::Layout as KeyboardLayout>::SimpleBacklightDeviceType as crate::lighting::simple_backlight::private::MaybeSimpleBacklightDevice>::get_state() {
        data[0] = state.get().await.speed
    }
}

#[cfg(feature = "simple-backlight-matrix")]
pub async fn simple_backlight_matrix_get_speed<K: ViaKeyboard>(data: &mut [u8]) {
    if let Some(state) = <<K::Layout as KeyboardLayout>::SimpleBacklightMatrixDeviceType as crate::lighting::simple_backlight_matrix::private::MaybeSimpleBacklightMatrixDevice>::get_state() {
        data[0] = state.get().await.speed
    }
}

#[cfg(feature = "rgb-backlight-matrix")]
pub async fn rgb_backlight_matrix_get_speed<K: ViaKeyboard>(data: &mut [u8]) {
    if let Some(state) = <<K::Layout as KeyboardLayout>::RGBBacklightMatrixDeviceType as crate::lighting::rgb_backlight_matrix::private::MaybeRGBBacklightMatrixDevice>::get_state() {
        data[0] = state.get().await.speed
    }
}

#[cfg(feature = "underglow")]
pub async fn underglow_set_speed<K: ViaKeyboard>(data: &[u8]) {
    if let Some(channel) = <<K::Layout as KeyboardLayout>::UnderglowDeviceType as crate::lighting::underglow::private::MaybeUnderglowDevice>::get_command_channel() {
        channel
            .send(crate::lighting::underglow::UnderglowCommand::SetSpeed(
                data[0],
            ))
            .await;
    }
}

#[cfg(feature = "simple-backlight")]
pub async fn simple_backlight_set_speed<K: ViaKeyboard>(data: &[u8]) {
    if let Some(channel) = <<K::Layout as KeyboardLayout>::SimpleBacklightDeviceType as crate::lighting::simple_backlight::private::MaybeSimpleBacklightDevice>::get_command_channel() {
        channel
            .send(crate::lighting::simple_backlight::SimpleBacklightCommand::SetSpeed(data[0]))
            .await;
    }
}

#[cfg(feature = "simple-backlight-matrix")]
pub async fn simple_backlight_matrix_set_speed<K: ViaKeyboard>(data: &[u8]) {
    if let Some(channel) = <<K::Layout as KeyboardLayout>::SimpleBacklightMatrixDeviceType as crate::lighting::simple_backlight_matrix::private::MaybeSimpleBacklightMatrixDevice>::get_command_channel() {
        channel
            .send(
                crate::lighting::simple_backlight_matrix::SimpleBacklightMatrixCommand::SetSpeed(
                    data[0],
                ),
            )
            .await;
    }
}

#[cfg(feature = "rgb-backlight-matrix")]
pub async fn rgb_backlight_matrix_set_speed<K: ViaKeyboard>(data: &[u8]) {
    if let Some(channel) = <<K::Layout as KeyboardLayout>::RGBBacklightMatrixDeviceType as crate::lighting::rgb_backlight_matrix::private::MaybeRGBBacklightMatrixDevice>::get_command_channel() {
        channel
            .send(
                crate::lighting::rgb_backlight_matrix::RGBBacklightMatrixCommand::SetSpeed(data[0]),
            )
            .await;
    }
}

#[cfg(feature = "underglow")]
pub async fn underglow_get_color<K: ViaKeyboard>(data: &mut [u8]) {
    if let Some(state) =
        <<K::Layout as KeyboardLayout>::UnderglowDeviceType as crate::lighting::underglow::private::MaybeUnderglowDevice>::get_state()
    {
        let config = state.get().await;
        data[0] = config.hue;
        data[1] = config.sat;
    }
}

#[cfg(feature = "rgb-backlight-matrix")]
pub async fn rgb_backlight_matrix_get_color<K: ViaKeyboard>(data: &mut [u8]) {
    if let Some(state) = <<K::Layout as KeyboardLayout>::RGBBacklightMatrixDeviceType as crate::lighting::rgb_backlight_matrix::private::MaybeRGBBacklightMatrixDevice>::get_state() {
        // Color only available on RGB matrices
        let config = state.get().await;
        data[0] = config.hue;
        data[1] = config.sat;
    }
}

#[cfg(feature = "underglow")]
pub async fn underglow_set_color<K: ViaKeyboard>(data: &[u8]) {
    if let Some(channel) = <<K::Layout as KeyboardLayout>::UnderglowDeviceType as crate::lighting::underglow::private::MaybeUnderglowDevice>::get_command_channel() {
        channel
            .send(crate::lighting::underglow::UnderglowCommand::SetHue(
                data[0],
            ))
            .await;
        channel
            .send(crate::lighting::underglow::UnderglowCommand::SetSaturation(
                data[1],
            ))
            .await;
    }
}

#[cfg(feature = "rgb-backlight-matrix")]
pub async fn rgb_backlight_matrix_set_color<K: ViaKeyboard>(data: &[u8]) {
    if let Some(channel) = <<K::Layout as KeyboardLayout>::RGBBacklightMatrixDeviceType as crate::lighting::rgb_backlight_matrix::private::MaybeRGBBacklightMatrixDevice>::get_command_channel() {
        // Color only available on RGB matrices
        channel
            .send(crate::lighting::rgb_backlight_matrix::RGBBacklightMatrixCommand::SetHue(data[0]))
            .await;
        channel
            .send(
                crate::lighting::rgb_backlight_matrix::RGBBacklightMatrixCommand::SetSaturation(
                    data[1],
                ),
            )
            .await;
    }
}

pub async fn underglow_save<K: ViaKeyboard>() {
    #[cfg(feature = "underglow")]
    if let Some(channel) = <<K::Layout as KeyboardLayout>::UnderglowDeviceType as crate::lighting::underglow::private::MaybeUnderglowDevice>::get_command_channel() {
        channel
            .send(crate::lighting::underglow::UnderglowCommand::SaveConfig)
            .await;
    }
}

pub async fn simple_backlight_save<K: ViaKeyboard>() {
    #[cfg(feature = "simple-backlight")]
    if let Some(channel) = <<K::Layout as KeyboardLayout>::SimpleBacklightDeviceType as crate::lighting::simple_backlight::private::MaybeSimpleBacklightDevice>::get_command_channel() {
        channel
            .send(crate::lighting::simple_backlight::SimpleBacklightCommand::SaveConfig)
            .await;
    }
}

pub async fn simple_backlight_matrix_save<K: ViaKeyboard>() {
    #[cfg(feature = "simple-backlight-matrix")]
    if let Some(channel) = <<K::Layout as KeyboardLayout>::SimpleBacklightMatrixDeviceType as crate::lighting::simple_backlight_matrix::private::MaybeSimpleBacklightMatrixDevice>::get_command_channel() {
        channel
            .send(
                crate::lighting::simple_backlight_matrix::SimpleBacklightMatrixCommand::SaveConfig,
            )
            .await;
    }
}

pub async fn rgb_backlight_matrix_save<K: ViaKeyboard>() {
    #[cfg(feature = "rgb-backlight-matrix")]
    if let Some(channel) = <<K::Layout as KeyboardLayout>::RGBBacklightMatrixDeviceType as crate::lighting::rgb_backlight_matrix::private::MaybeRGBBacklightMatrixDevice>::get_command_channel() {
        channel
            .send(crate::lighting::rgb_backlight_matrix::RGBBacklightMatrixCommand::SaveConfig)
            .await;
    }
}
