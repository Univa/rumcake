use core::cell::{Cell, RefCell};

use defmt::{debug, error, info, warn, Debug2Format};
use embassy_futures::select::{self, select3, select4};
use heapless::Vec;
use nrf_softdevice::ble::gatt_server::builder::ServiceBuilder;
use nrf_softdevice::ble::gatt_server::characteristic::{Attribute, Metadata, Properties};
use nrf_softdevice::ble::gatt_server::{
    self, get_sys_attrs, run, set_sys_attrs, GetValueError, NotifyValueError, RegisterError,
    Service, SetValueError,
};
use nrf_softdevice::ble::peripheral::{advertise_pairable, ConnectableAdvertisement};
use nrf_softdevice::ble::security::{IoCapabilities, SecurityHandler};
use nrf_softdevice::ble::{
    Connection, EncryptionInfo, GattValue, IdentityKey, MasterId, SecurityMode, Uuid,
};
use nrf_softdevice::Softdevice;
use packed_struct::prelude::{PackedStruct, PrimitiveEnum};
use static_cell::StaticCell;
use usbd_human_interface_device::device::consumer::MultipleConsumerReport;
use usbd_human_interface_device::device::keyboard::NKROBootKeyboardReport;

use crate::hw::platform::BLUETOOTH_ADVERTISING_MUTEX;
use crate::hw::{HIDOutput, BATTERY_LEVEL_STATE, CURRENT_OUTPUT_STATE};

use crate::bluetooth::{
    BluetoothKeyboard, BATTERY_LEVEL_LISTENER, BLUETOOTH_CONNECTED_STATE,
    CURRENT_OUTPUT_STATE_LISTENER,
};

#[derive(Clone, Copy)]
struct Peer {
    master_id: MasterId,
    key: EncryptionInfo,
    peer_id: IdentityKey,
}

pub struct Bonder {
    peer: Cell<Option<Peer>>,
    sys_attrs: RefCell<Vec<u8, 62>>,
}

impl Default for Bonder {
    fn default() -> Self {
        Bonder {
            peer: Cell::new(None),
            sys_attrs: Default::default(),
        }
    }
}

impl SecurityHandler for Bonder {
    fn io_capabilities(&self) -> IoCapabilities {
        IoCapabilities::None
    }

    fn can_bond(&self, _conn: &Connection) -> bool {
        true
    }

    // fn display_passkey(&self, passkey: &[u8; 6]) {
    //     info!("[BT_HID] Passkey: {}", Debug2Format(passkey));
    // }

    // fn enter_passkey(&self, _reply: nrf_softdevice::ble::PasskeyReply) {}

    fn on_security_update(&self, _conn: &Connection, security_mode: SecurityMode) {
        debug!(
            "[BT_HID] new security mode: {}",
            Debug2Format(&security_mode)
        );
    }

    fn on_bonded(
        &self,
        _conn: &Connection,
        master_id: MasterId,
        key: EncryptionInfo,
        peer_id: IdentityKey,
    ) {
        // First time
        debug!("[BT_HID] storing bond for: id: {}, key: {}", master_id, key);

        // TODO: save keys
        self.sys_attrs.borrow_mut().clear();
        self.peer.set(Some(Peer {
            master_id,
            key,
            peer_id,
        }))
    }

    fn get_key(&self, _conn: &Connection, master_id: MasterId) -> Option<EncryptionInfo> {
        // Reconnecting with an existing bond
        debug!("[BT_HID] getting bond for: id: {}", master_id);

        self.peer
            .get()
            .and_then(|peer| (master_id == peer.master_id).then_some(peer.key))
    }

    fn save_sys_attrs(&self, conn: &Connection) {
        // On disconnect usually
        debug!(
            "[BT_HID] saving system attributes for: {}",
            conn.peer_address()
        );

        if let Some(peer) = self.peer.get() {
            if peer.peer_id.is_match(conn.peer_address()) {
                let mut sys_attrs = self.sys_attrs.borrow_mut();
                let capacity = sys_attrs.capacity();
                sys_attrs.resize(capacity, 0).unwrap();
                let len = get_sys_attrs(conn, &mut sys_attrs).unwrap() as u16;
                sys_attrs.truncate(len as usize);
                // TODO: save sys_attrs for peer
            }
        }
    }

    fn load_sys_attrs(&self, conn: &Connection) {
        let addr = conn.peer_address();
        debug!("[BT_HID] loading system attributes for: {}", addr);

        let attrs = self.sys_attrs.borrow();

        // TODO: search stored peers
        let attrs = if self
            .peer
            .get()
            .map(|peer| peer.peer_id.is_match(addr))
            .unwrap_or(false)
        {
            (!attrs.is_empty()).then_some(attrs.as_slice())
        } else {
            None
        };

        if let Err(err) = set_sys_attrs(conn, attrs) {
            warn!(
                "[BT_HID] SecurityHandler failed to set sys attrs: {:?}",
                err
            );
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, PrimitiveEnum, Default)]
pub enum VidSource {
    #[default]
    BluetoothSIG = 1,
    UsbIF = 2,
}

#[derive(Clone, Copy, PackedStruct, Default)]
#[packed_struct(endian = "lsb", bit_numbering = "msb0")]
pub struct PnPID {
    #[packed_field(bytes = "0", ty = "enum")]
    pub vid_source: VidSource,
    #[packed_field()]
    pub vendor_id: u16,
    #[packed_field()]
    pub product_id: u16,
    #[packed_field()]
    pub product_version: u16,
}

pub struct DeviceInformationService {
    model_number_value_handle: u16,
    serial_number_value_handle: u16,
    firmware_revision_value_handle: u16,
    hardware_revision_value_handle: u16,
    manufacturer_name_value_handle: u16,
    pnp_id_value_handle: u16,
}

pub enum DeviceInformationServiceEvent {}

impl DeviceInformationService {
    pub fn new(sd: &mut Softdevice) -> Result<Self, RegisterError> {
        let mut sb = ServiceBuilder::new(sd, Uuid::new_16(0x180a)).unwrap();

        let model_number_handles = sb
            .add_characteristic(
                Uuid::new_16(0x2a24),
                Attribute::new("")
                    .variable_len(32)
                    .read_security(SecurityMode::JustWorks),
                Metadata::new(Properties::new().read()),
            )
            .unwrap()
            .build();

        let serial_number_handles = sb
            .add_characteristic(
                Uuid::new_16(0x2a25),
                Attribute::new("")
                    .variable_len(32)
                    .read_security(SecurityMode::JustWorks),
                Metadata::new(Properties::new().read()),
            )
            .unwrap()
            .build();

        let firmware_revision_handles = sb
            .add_characteristic(
                Uuid::new_16(0x2a26),
                Attribute::new("")
                    .variable_len(32)
                    .read_security(SecurityMode::JustWorks),
                Metadata::new(Properties::new().read()),
            )
            .unwrap()
            .build();

        let hardware_revision_handles = sb
            .add_characteristic(
                Uuid::new_16(0x2a27),
                Attribute::new("")
                    .variable_len(32)
                    .read_security(SecurityMode::JustWorks),
                Metadata::new(Properties::new().read()),
            )
            .unwrap()
            .build();

        let manufacturer_name_handles = sb
            .add_characteristic(
                Uuid::new_16(0x2a29),
                Attribute::new("")
                    .variable_len(32)
                    .read_security(SecurityMode::JustWorks),
                Metadata::new(Properties::new().read()),
            )
            .unwrap()
            .build();

        let pnp_id_handles = sb
            .add_characteristic(
                Uuid::new_16(0x2a50),
                Attribute::new(PnPID::default().pack().unwrap())
                    .read_security(SecurityMode::JustWorks),
                Metadata::new(Properties::new().read()),
            )
            .unwrap()
            .build();

        sb.build();

        Ok(Self {
            model_number_value_handle: model_number_handles.value_handle,
            serial_number_value_handle: serial_number_handles.value_handle,
            firmware_revision_value_handle: firmware_revision_handles.value_handle,
            hardware_revision_value_handle: hardware_revision_handles.value_handle,
            manufacturer_name_value_handle: manufacturer_name_handles.value_handle,
            pnp_id_value_handle: pnp_id_handles.value_handle,
        })
    }

    pub fn model_number_set(
        &self,
        sd: &Softdevice,
        str: &'static str,
    ) -> Result<(), SetValueError> {
        gatt_server::set_value(sd, self.model_number_value_handle, str.as_bytes())?;
        Ok(())
    }

    pub fn serial_number_set(
        &self,
        sd: &Softdevice,
        str: &'static str,
    ) -> Result<(), SetValueError> {
        gatt_server::set_value(sd, self.serial_number_value_handle, str.as_bytes())?;
        Ok(())
    }

    pub fn firmware_revision_set(
        &self,
        sd: &Softdevice,
        str: &'static str,
    ) -> Result<(), SetValueError> {
        gatt_server::set_value(sd, self.firmware_revision_value_handle, str.as_bytes())?;
        Ok(())
    }

    pub fn hardware_revision_set(
        &self,
        sd: &Softdevice,
        str: &'static str,
    ) -> Result<(), SetValueError> {
        gatt_server::set_value(sd, self.hardware_revision_value_handle, str.as_bytes())?;
        Ok(())
    }

    pub fn manufacturer_name_set(
        &self,
        sd: &Softdevice,
        str: &'static str,
    ) -> Result<(), SetValueError> {
        gatt_server::set_value(sd, self.manufacturer_name_value_handle, str.as_bytes())?;
        Ok(())
    }

    pub fn pnp_id_set(&self, sd: &Softdevice, pnp: &PnPID) -> Result<(), SetValueError> {
        gatt_server::set_value(sd, self.pnp_id_value_handle, &pnp.pack().unwrap())?;
        Ok(())
    }
}

impl Service for DeviceInformationService {
    type Event = DeviceInformationServiceEvent;

    fn on_write(&self, _handle: u16, _data: &[u8]) -> Option<Self::Event> {
        None
    }
}

pub struct HIDService {
    keyboard_report_value_handle: u16,
    keyboard_report_cccd_handle: u16,
    consumer_report_value_handle: u16,
    consumer_report_cccd_handle: u16,
    via_input_report_value_handle: u16,
    via_input_report_cccd_handle: u16,
    via_output_report_value_handle: u16,
    hid_control_value_handle: u16,
}

/// Report descriptor with NKRO, consumer control and Via functionality. This is basically a
/// combination of
/// [`usbd_human_interface_device::device::keyboard::NKRO_BOOT_KEYBOARD_REPORT_DESCRIPTOR`],
/// [`usbd_human_interface_device::device::consumer::MULTIPLE_CODE_REPORT_DESCRIPTOR`], and
/// [`crate::via::VIA_REPORT_DESCRIPTOR`], with report IDs included. Without report IDs, some
/// functionality doesn't seem to work as expected. In testing, exclusion of a report ID seems to
/// prevent Via output reports from being received. Potentially related:
/// https://devzone.nordicsemi.com/f/nordic-q-a/24486/hid-get-report-from-a-mac-not-as-expected
pub(crate) const REPORT_MAP: &[u8] = &[
    // NKRO reports
    0x05, 0x01, // Usage Page (Generic Desktop),
    0x09, 0x06, // Usage (Keyboard),
    0xA1, 0x01, // Collection (Application),
    // bitmap of modifiers
    0x85, 0x01, //   Report ID (1)
    0x75, 0x01, //   Report Size (1),
    0x95, 0x08, //   Report Count (8),
    0x05, 0x07, //   Usage Page (Key Codes),
    0x19, 0xE0, //   Usage Minimum (224),
    0x29, 0xE7, //   Usage Maximum (231),
    0x15, 0x00, //   Logical Minimum (0),
    0x25, 0x01, //   Logical Maximum (1),
    0x81, 0x02, //   Input (Data, Variable, Absolute), ;Modifier byte
    // 7 bytes of padding
    0x75, 0x38, //   Report Size (0x38),
    0x95, 0x01, //   Report Count (1),
    0x81, 0x01, //   Input (Constant), ;Reserved byte
    // LED output report
    0x95, 0x05, //   Report Count (5),
    0x75, 0x01, //   Report Size (1),
    0x05, 0x08, //   Usage Page (LEDs),
    0x19, 0x01, //   Usage Minimum (1),
    0x29, 0x05, //   Usage Maximum (5),
    0x91, 0x02, //   Output (Data, Variable, Absolute),
    0x95, 0x01, //   Report Count (1),
    0x75, 0x03, //   Report Size (3),
    0x91, 0x03, //   Output (Constant),
    // bitmap of keys
    0x95, 0x88, //   Report Count () - (REPORT_BYTES-1)*8
    0x75, 0x01, //   Report Size (1),
    0x15, 0x00, //   Logical Minimum (0),
    0x25, 0x01, //   Logical Maximum(1),
    0x05, 0x07, //   Usage Page (Key Codes),
    0x19, 0x00, //   Usage Minimum (0),
    0x29, 0x87, //   Usage Maximum (), - (REPORT_BYTES-1)*8-1
    0x81, 0x02, //   Input (Data, Variable, Absolute),
    0xc0, // End Collection
    // Consumer reports
    0x05, 0x0C, // Usage Page (Consumer),
    0x09, 0x01, // Usage (Consumer Control),
    0xA1, 0x01, // Collection (Application),
    0x85, 0x02, //   Report ID (2)
    0x75, 0x10, //     Report Size(16)
    0x95, 0x04, //     Report Count(4)
    0x15, 0x00, //     Logical Minimum(0)
    0x26, 0x9C, 0x02, //     Logical Maximum(0x029C)
    0x19, 0x00, //     Usage Minimum(0)
    0x2A, 0x9C, 0x02, //     Usage Maximum(0x029C)
    0x81, 0x00, //     Input (Array, Data, Variable)
    0xC0, // End Collection
    // Via reports
    0x06, 0x60, 0xFF, // Usage Page (Vendor Defined)
    0x09, 0x61, // Usage (Vendor Defined)
    0xA1, 0x01, // Collection (Application)
    0x85, 0x03, //   Report ID (3)
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

impl HIDService {
    pub fn new(sd: &mut Softdevice) -> Result<Self, RegisterError> {
        let mut sb = ServiceBuilder::new(sd, Uuid::new_16(0x1812)).unwrap();

        #[rustfmt::skip]
        let _hid_info_builder = sb.add_characteristic(
            Uuid::new_16(0x2a4a),
            Attribute::new(&[
                0x00, 0x00, // Version
                0x00,   // Code
                0b01 | // Remote wake
                0b10, // Normally connectable
            ]).read_security(SecurityMode::JustWorks),
            Metadata::new(Properties::new().read()),
        );

        let _report_map_builder = sb.add_characteristic(
            Uuid::new_16(0x2a4b),
            Attribute::new(REPORT_MAP).read_security(SecurityMode::JustWorks),
            Metadata::new(Properties::new().read()),
        );

        let mut keyboard_report_builder = sb
            .add_characteristic(
                Uuid::new_16(0x2a4d),
                Attribute::new(NKROBootKeyboardReport::default().pack().unwrap())
                    .security(SecurityMode::JustWorks),
                Metadata::with_security(Properties::new().read().notify(), SecurityMode::JustWorks),
            )
            .unwrap();
        keyboard_report_builder
            .add_descriptor(
                Uuid::new_16(0x2908),
                Attribute::new(&[
                    0x01, // ID
                    0x01, // Input
                ])
                .security(SecurityMode::JustWorks),
            )
            .unwrap();
        let keyboard_report_handles = keyboard_report_builder.build();

        let mut consumer_report_builder = sb
            .add_characteristic(
                Uuid::new_16(0x2a4d),
                Attribute::new(MultipleConsumerReport::default().pack().unwrap())
                    .security(SecurityMode::JustWorks),
                Metadata::with_security(Properties::new().read().notify(), SecurityMode::JustWorks),
            )
            .unwrap();
        consumer_report_builder
            .add_descriptor(
                Uuid::new_16(0x2908),
                Attribute::new(&[
                    0x02, // ID
                    0x01, // Input
                ])
                .security(SecurityMode::JustWorks),
            )
            .unwrap();
        let consumer_report_handles = consumer_report_builder.build();

        let mut via_input_report_builder = sb
            .add_characteristic(
                Uuid::new_16(0x2a4d),
                Attribute::new([0; 32]).security(SecurityMode::JustWorks),
                Metadata::with_security(Properties::new().read().notify(), SecurityMode::JustWorks),
            )
            .unwrap();
        via_input_report_builder
            .add_descriptor(
                Uuid::new_16(0x2908),
                Attribute::new(&[
                    0x03, // ID
                    0x01, // Input
                ])
                .security(SecurityMode::JustWorks),
            )
            .unwrap();
        let via_input_report_handles = via_input_report_builder.build();

        let mut via_output_report_builder = sb
            .add_characteristic(
                Uuid::new_16(0x2a4d),
                Attribute::new([0; 32]).security(SecurityMode::JustWorks),
                Metadata::with_security(
                    Properties::new().read().write().write_without_response(),
                    SecurityMode::JustWorks,
                ),
            )
            .unwrap();
        via_output_report_builder
            .add_descriptor(
                Uuid::new_16(0x2908),
                Attribute::new(&[
                    0x03, // ID
                    0x02, // Output
                ])
                .security(SecurityMode::JustWorks),
            )
            .unwrap();
        let via_output_report_handles = via_output_report_builder.build();

        let hid_control_builder = sb
            .add_characteristic(
                Uuid::new_16(0x2a4c),
                Attribute::new(&[0]).write_security(SecurityMode::JustWorks),
                Metadata::new(Properties::new().write_without_response()),
            )
            .unwrap();
        let hid_control_handles = hid_control_builder.build();

        sb.build();

        Ok(Self {
            keyboard_report_value_handle: keyboard_report_handles.value_handle,
            keyboard_report_cccd_handle: keyboard_report_handles.cccd_handle,
            consumer_report_value_handle: consumer_report_handles.value_handle,
            consumer_report_cccd_handle: consumer_report_handles.cccd_handle,
            via_input_report_value_handle: via_input_report_handles.value_handle,
            via_input_report_cccd_handle: via_input_report_handles.cccd_handle,
            via_output_report_value_handle: via_output_report_handles.value_handle,
            hid_control_value_handle: hid_control_handles.value_handle,
        })
    }

    pub fn keyboard_report_notify(
        &self,
        connection: &Connection,
        report: NKROBootKeyboardReport,
    ) -> Result<(), NotifyValueError> {
        gatt_server::notify_value(
            connection,
            self.keyboard_report_value_handle,
            &report.pack().unwrap(),
        )?;
        Ok(())
    }

    pub fn consumer_report_notify(
        &self,
        connection: &Connection,
        report: MultipleConsumerReport,
    ) -> Result<(), NotifyValueError> {
        gatt_server::notify_value(
            connection,
            self.consumer_report_value_handle,
            &report.pack().unwrap(),
        )?;
        Ok(())
    }

    pub fn via_report_notify(
        &self,
        connection: &Connection,
        report: [u8; 32],
    ) -> Result<(), NotifyValueError> {
        gatt_server::notify_value(connection, self.via_input_report_value_handle, &report)?;
        Ok(())
    }

    pub fn unsafe_via_report_get(&self) -> Result<[u8; 32], GetValueError> {
        unsafe {
            let sd = nrf_softdevice::Softdevice::steal();
            let buf = &mut [0; 32];
            gatt_server::get_value(sd, self.via_output_report_value_handle, buf)?;
            Ok(*buf)
        }
    }

    pub fn unsafe_hid_control_get(&self) -> Result<u8, GetValueError> {
        unsafe {
            let sd = nrf_softdevice::Softdevice::steal();
            let buf = &mut [0];
            gatt_server::get_value(sd, self.hid_control_value_handle, buf)?;
            Ok(buf[0])
        }
    }
}

pub enum HIDServiceEvent {
    KeyboardReportCccdWrite { notifications: bool },
    ConsumerReportCccdWrite { notifications: bool },
    ViaReportCccdWrite { notifications: bool },
    ViaReportWrite([u8; 32]),
    HidControlWrite(u8),
}

impl Service for HIDService {
    type Event = HIDServiceEvent;

    fn on_write(&self, handle: u16, data: &[u8]) -> Option<Self::Event> {
        if handle == self.keyboard_report_cccd_handle && !data.is_empty() {
            match data[0] & 0x01 {
                0x00 => {
                    return Some(HIDServiceEvent::KeyboardReportCccdWrite {
                        notifications: false,
                    })
                }
                0x01 => {
                    return Some(HIDServiceEvent::KeyboardReportCccdWrite {
                        notifications: true,
                    })
                }
                _ => {}
            }
        }
        if handle == self.consumer_report_cccd_handle {
            match data[0] & 0x01 {
                0x00 => {
                    return Some(HIDServiceEvent::ConsumerReportCccdWrite {
                        notifications: false,
                    })
                }
                0x01 => {
                    return Some(HIDServiceEvent::ConsumerReportCccdWrite {
                        notifications: true,
                    })
                }
                _ => {}
            }
        }
        if handle == self.via_input_report_cccd_handle {
            match data[0] & 0x01 {
                0x00 => {
                    return Some(HIDServiceEvent::ViaReportCccdWrite {
                        notifications: false,
                    })
                }
                0x01 => {
                    return Some(HIDServiceEvent::ViaReportCccdWrite {
                        notifications: true,
                    })
                }
                _ => {}
            }
        }
        if handle == self.via_output_report_value_handle {
            if data.len() < <u8 as GattValue>::MIN_SIZE {
                return self
                    .unsafe_via_report_get()
                    .ok()
                    .map(HIDServiceEvent::ViaReportWrite);
            } else {
                return Some(HIDServiceEvent::ViaReportWrite(<[u8; 32]>::from_gatt(data)));
            }
        }
        if handle == self.hid_control_value_handle {
            if data.len() < <u8 as GattValue>::MIN_SIZE {
                return self
                    .unsafe_hid_control_get()
                    .ok()
                    .map(HIDServiceEvent::HidControlWrite);
            } else {
                return Some(HIDServiceEvent::HidControlWrite(u8::from_gatt(data)));
            }
        }
        None
    }
}

#[nrf_softdevice::gatt_service(uuid = "180f")]
pub struct BatteryService {
    #[characteristic(uuid = "2a19", read, notify, security = "justworks")]
    battery_level: u8,
}

#[nrf_softdevice::gatt_server]
pub struct Server {
    bas: BatteryService,
    dis: DeviceInformationService,
    hids: HIDService,
}

pub async fn nrf_ble_task<K: BluetoothKeyboard>(_k: K, sd: &'static Softdevice, server: Server)
where
    [(); K::PRODUCT.len() + 15]:,
{
    #[rustfmt::skip]
    let adv_data: Vec<u8, { K::PRODUCT.len() + 15 }> = [
        0x02, 0x01, nrf_softdevice::raw::BLE_GAP_ADV_FLAGS_LE_ONLY_GENERAL_DISC_MODE as u8,
        0x05, 0x03, 0x12, 0x18, 0x0F, 0x18, // Incomplete list of 16 bit services: HID service and battery service
        0x03, 0x19, 0xC1, 0x03, // Appearance: Keyboard
        (K::PRODUCT.len() + 1) as u8, 0x09, // Complete name: keyboard name
    ].iter().cloned().chain(K::PRODUCT.as_bytes().iter().cloned()).collect();

    #[rustfmt::skip]
    let scan_data = [
        0x05, 0x03, 0x12, 0x18, 0x0F, 0x18, // Incomplete list of 16 bit services: HID service and battery service
    ];

    // Setup device information service
    server.dis.model_number_set(sd, K::PRODUCT).unwrap();
    server.dis.serial_number_set(sd, K::SERIAL_NUMBER).unwrap();
    server
        .dis
        .manufacturer_name_set(sd, K::MANUFACTURER)
        .unwrap();
    server
        .dis
        .hardware_revision_set(sd, K::HARDWARE_REVISION)
        .unwrap();
    server
        .dis
        .firmware_revision_set(sd, K::FIRMWARE_REVISION)
        .unwrap();
    server
        .dis
        .pnp_id_set(
            sd,
            &PnPID {
                vid_source: VidSource::BluetoothSIG,
                product_id: K::BLE_PID,
                vendor_id: K::BLE_VID,
                product_version: 1,
            },
        )
        .unwrap();

    info!("[BT_HID] Bluetooth services started");

    static BONDER: StaticCell<Bonder> = StaticCell::new();
    let bonder = BONDER.init(Bonder::default());

    loop {
        let advertisement = ConnectableAdvertisement::ScannableUndirected {
            adv_data: &adv_data,
            scan_data: &scan_data,
        };

        let connection = {
            let _lock = BLUETOOTH_ADVERTISING_MUTEX.lock().await;
            match advertise_pairable(sd, advertisement, &Default::default(), bonder).await {
                Ok(connection) => {
                    info!("[BT_HID] Connection established with host device");
                    BLUETOOTH_CONNECTED_STATE.set(true).await;
                    connection
                }
                Err(error) => {
                    warn!("[BT_HID] BLE advertising error: {}", Debug2Format(&error));
                    continue;
                }
            }
        };

        let conn_fut = run(&connection, &server, |event| match event {
            ServerEvent::Bas(bas_event) => match bas_event {
                BatteryServiceEvent::BatteryLevelCccdWrite { notifications } => {
                    debug!("[BT_HID] Battery value CCCD updated: {}", notifications);
                }
            },
            ServerEvent::Dis(dis_event) => match dis_event {},
            ServerEvent::Hids(hids_event) => match hids_event {
                HIDServiceEvent::KeyboardReportCccdWrite { notifications } => {
                    debug!("[BT_HID] Keyboard report CCCD updated: {}", notifications);
                }
                HIDServiceEvent::ConsumerReportCccdWrite { notifications } => {
                    debug!("[BT_HID] Consumer report CCCD updated: {}", notifications);
                }
                HIDServiceEvent::ViaReportCccdWrite { notifications } => {
                    debug!("[BT_HID] Via report CCCD updated: {}", notifications);
                }
                HIDServiceEvent::ViaReportWrite(report) => {
                    #[cfg(feature = "via")]
                    {
                        let channel = K::get_via_hid_receive_channel();
                        match channel.try_send(report) {
                            Ok(()) => {
                                debug!("[BT_HID] Received Via report: {}", report);
                            }
                            Err(err) => {
                                error!(
                                    "[BT_HID] Could not consume Via report. data: {:?} error: {:?}",
                                    Debug2Format(&report),
                                    Debug2Format(&err)
                                );
                            }
                        }
                    }

                    #[cfg(not(feature = "via"))]
                    warn!("[BT_HID] Via is not enabled. Ignoring report: {}", report);
                }
                HIDServiceEvent::HidControlWrite(val) => {
                    debug!("[BT_HID] Received HID control value: {=u8}", val);
                }
            },
        });

        let adc_fut = async {
            loop {
                BATTERY_LEVEL_LISTENER.wait().await;
                let pct = BATTERY_LEVEL_STATE.get().await;

                match server.bas.battery_level_notify(&connection, &pct) {
                    Ok(_) => {
                        debug!(
                            "[BT_HID] Notified connection of new battery level: {=u8}",
                            pct
                        );
                    }
                    Err(error) => {
                        error!(
                            "[BT_HID] Could not notify connection of new battery level ({=u8}): {}",
                            pct,
                            Debug2Format(&error)
                        );
                    }
                }
            }
        };

        let hid_fut = async {
            let keyboard_report_channel = K::get_keyboard_report_send_channel();
            let consumer_report_channel = K::get_consumer_report_send_channel();

            // Discard any reports that haven't been processed due to lack of a connection
            while keyboard_report_channel.try_receive().is_ok() {}
            while consumer_report_channel.try_receive().is_ok() {}

            #[cfg(feature = "via")]
            let via_report_channel = K::get_via_hid_send_channel();

            #[cfg(feature = "via")]
            while via_report_channel.try_receive().is_ok() {}

            loop {
                if matches!(CURRENT_OUTPUT_STATE.get().await, Some(HIDOutput::Bluetooth)) {
                    #[cfg(feature = "via")]
                    match select4(
                        CURRENT_OUTPUT_STATE_LISTENER.wait(),
                        keyboard_report_channel.receive(),
                        consumer_report_channel.receive(),
                        via_report_channel.receive(),
                    )
                    .await
                    {
                        select::Either4::First(()) => {}
                        select::Either4::Second(report) => {
                            info!(
                                "[BT_HID] Writing NKRO HID report to bluetooth: {:?}",
                                Debug2Format(&report)
                            );

                            if let Err(err) =
                                server.hids.keyboard_report_notify(&connection, report)
                            {
                                error!(
                                    "[BT_HID] Couldn't write NKRO HID report: {:?}",
                                    Debug2Format(&err)
                                );
                            };
                        }
                        select::Either4::Third(report) => {
                            info!(
                                "[BT_HID] Writing consumer HID report to bluetooth: {:?}",
                                Debug2Format(&report)
                            );

                            if let Err(err) =
                                server.hids.consumer_report_notify(&connection, report)
                            {
                                error!(
                                    "[BT_HID] Couldn't write consumer HID report: {:?}",
                                    Debug2Format(&err)
                                );
                            };
                        }
                        select::Either4::Fourth(report) => {
                            info!(
                                "[BT_HID] Writing Via HID report to bluetooth: {:?}",
                                Debug2Format(&report)
                            );

                            if let Err(err) = server.hids.via_report_notify(&connection, report) {
                                error!(
                                    "[BT_HID] Couldn't write Via HID report: {:?}",
                                    Debug2Format(&err)
                                );
                            };
                        }
                    };

                    #[cfg(not(feature = "via"))]
                    match select3(
                        CURRENT_OUTPUT_STATE_LISTENER.wait(),
                        keyboard_report_channel.receive(),
                        consumer_report_channel.receive(),
                    )
                    .await
                    {
                        select::Either3::First(()) => {}
                        select::Either3::Second(report) => {
                            info!(
                                "[BT_HID] Writing NKRO HID report to bluetooth: {:?}",
                                Debug2Format(&report)
                            );

                            if let Err(err) =
                                server.hids.keyboard_report_notify(&connection, report)
                            {
                                error!(
                                    "[BT_HID] Couldn't write NKRO HID report: {:?}",
                                    Debug2Format(&err)
                                );
                            };
                        }
                        select::Either3::Third(report) => {
                            info!(
                                "[BT_HID] Writing consumer HID report to bluetooth: {:?}",
                                Debug2Format(&report)
                            );

                            if let Err(err) =
                                server.hids.consumer_report_notify(&connection, report)
                            {
                                error!(
                                    "[BT_HID] Couldn't write consumer HID report: {:?}",
                                    Debug2Format(&err)
                                );
                            };
                        }
                    };
                } else {
                    CURRENT_OUTPUT_STATE_LISTENER.wait().await;
                }
            }
        };

        match select3(conn_fut, adc_fut, hid_fut).await {
            select::Either3::First(error) => {
                warn!(
                    "[BT_HID] Connection has been lost: {}",
                    Debug2Format(&error)
                );
                BLUETOOTH_CONNECTED_STATE.set(false).await;
            }
            select::Either3::Second(_) => {
                error!("[BT_HID] Battery task failed. This should not happen.");
            }
            select::Either3::Third(_) => {
                error!("[BT_HID] HID task failed. This should not happen.");
            }
        };
    }
}
