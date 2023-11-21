use core::cell::{Cell, RefCell};

use defmt::{debug, error, info, warn, Debug2Format};
use embassy_futures::join;
use embassy_futures::select::{self, select, select3};
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
use usbd_human_interface_device::device::consumer::{
    MultipleConsumerReport, MULTIPLE_CODE_REPORT_DESCRIPTOR,
};
use usbd_human_interface_device::device::keyboard::{
    NKROBootKeyboardReport, NKRO_BOOT_KEYBOARD_REPORT_DESCRIPTOR,
};

use crate::hw::mcu::BLUETOOTH_ADVERTISING_MUTEX;
use crate::hw::BATTERY_LEVEL_STATE;
use crate::keyboard::KEYBOARD_REPORT_HID_SEND_CHANNEL;

#[cfg(feature = "usb")]
use crate::usb::USB_STATE;

use crate::bluetooth::{
    BluetoothCommand, BluetoothKeyboard, BATTERY_LEVEL_LISTENER, BLUETOOTH_COMMAND_CHANNEL,
    USB_STATE_LISTENER,
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

// TODO: Via/vial
pub struct HIDService {
    keyboard_report_value_handle: u16,
    keyboard_report_cccd_handle: u16,
    consumer_report_value_handle: u16,
    consumer_report_cccd_handle: u16,
    hid_control_value_handle: u16,
}

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

        let hid_descriptors: Vec<u8, { 63 + 23 }> = NKRO_BOOT_KEYBOARD_REPORT_DESCRIPTOR
            .iter()
            .cloned()
            .chain(MULTIPLE_CODE_REPORT_DESCRIPTOR.iter().cloned())
            .collect();

        let _report_map_builder = sb.add_characteristic(
            Uuid::new_16(0x2a4b),
            Attribute::new(&hid_descriptors).read_security(SecurityMode::JustWorks),
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
                    0x00, // ID
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
                    0x01, // ID
                    0x01, // Input
                ])
                .security(SecurityMode::JustWorks),
            )
            .unwrap();
        let consumer_report_handles = consumer_report_builder.build();

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

#[rumcake_macros::task]
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

    let connection_fut = async {
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
                // Discard any reports that haven't been processed due to lack of a connection
                while KEYBOARD_REPORT_HID_SEND_CHANNEL.try_receive().is_ok() {}

                #[cfg(feature = "usb")]
                {
                    loop {
                        if !USB_STATE.get().await {
                            match select(
                                USB_STATE_LISTENER.wait(),
                                KEYBOARD_REPORT_HID_SEND_CHANNEL.receive(),
                            )
                            .await
                            {
                                select::Either::First(()) => {
                                    info!(
                                        "[BT_HID] Bluetooth HID reports enabled = {}",
                                        !USB_STATE.get().await
                                    );
                                }
                                select::Either::Second(report) => {
                                    // TODO: media keys
                                    info!(
                                        "[BT_HID] Writing HID keyboard report to bluetooth: {:?}",
                                        Debug2Format(&report)
                                    );

                                    if let Err(err) =
                                        server.hids.keyboard_report_notify(&connection, report)
                                    {
                                        error!(
                                            "[BT_HID] Couldn't write HID keyboard report: {:?}",
                                            Debug2Format(&err)
                                        );
                                    };
                                }
                            };
                        } else {
                            USB_STATE_LISTENER.wait().await;
                            info!(
                                "[BT_HID] Bluetooth HID reports enabled = {}",
                                !USB_STATE.get().await
                            );
                        }
                    }
                }

                #[cfg(not(feature = "usb"))]
                {
                    loop {
                        let report = KEYBOARD_REPORT_HID_SEND_CHANNEL.receive().await;

                        // TODO: media keys
                        info!(
                            "[BT_HID] Writing HID keyboard report to bluetooth: {:?}",
                            Debug2Format(&report)
                        );

                        if let Err(err) = server.hids.keyboard_report_notify(&connection, report) {
                            error!(
                                "[BT_HID] Couldn't write HID keyboard report: {:?}",
                                Debug2Format(&err)
                            );
                        };
                    }
                }
            };

            match select3(conn_fut, adc_fut, hid_fut).await {
                select::Either3::First(error) => {
                    warn!(
                        "[BT_HID] Connection has been lost: {}",
                        Debug2Format(&error)
                    )
                }
                select::Either3::Second(_) => {
                    error!("[BT_HID] Battery task failed. This should not happen.");
                }
                select::Either3::Third(_) => {
                    error!("[BT_HID] HID task failed. This should not happen.");
                }
            };
        }
    };

    let command_fut = async {
        loop {
            let command = BLUETOOTH_COMMAND_CHANNEL.receive().await;
            match command {
                #[cfg(feature = "usb")]
                BluetoothCommand::ToggleOutput => {
                    USB_STATE.set(!USB_STATE.get().await).await;
                }
                #[cfg(feature = "usb")]
                BluetoothCommand::OutputUSB => {
                    USB_STATE.set(true).await;
                }
                #[cfg(feature = "usb")]
                BluetoothCommand::OutputBluetooth => {
                    USB_STATE.set(false).await;
                }
            }
        }
    };

    join::join(command_fut, connection_fut).await;
}
