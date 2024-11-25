//! Rumcake driver implementations for [`nrf-softdevice`].
//!
//! This driver provides implementations for
//! [`CentralDeviceDriver`](`crate::split::central::CentralDeviceDriver`), and
//! [`PeripheralDeviceDriver`](`crate::split::peripheral::PeripheralDeviceDriver`).
//!
//! To use this driver for split keyboards, central devices must pass the Bluetooth addresses of
//! the peripherals to [`nrf_ble_central_task`], and peripheral devices must implement pass the
//! Bluetooth address of the central device to [`nrf_ble_peripheral_task`].
//! [`central::NRFBLECentralDriver`] and [`peripheral::NRFBLEPeripheralDriver`] then need to be
//! passed to the [`central_task`] and [`peripheral_task`] respectively.

#[cfg(feature = "split-central")]
/// nrf-softdevice central device driver implementations
pub mod central {
    use defmt::{assert, debug, error, info, warn, Debug2Format};
    use embassy_futures::select::{select, select_array, Either};
    use embassy_sync::channel::Channel;
    use embassy_sync::mutex::Mutex;
    use embassy_sync::pubsub::{PubSubChannel, Publisher};
    use heapless::Vec;
    use nrf_softdevice::ble::central::{connect, ConnectError};
    use nrf_softdevice::ble::gatt_client::{self, discover};
    use nrf_softdevice::ble::{central, Address, AddressType};
    use nrf_softdevice::{RawError, Softdevice};

    use crate::hw::platform::RawMutex;
    use crate::split::central::{CentralDeviceDriver, CentralDeviceError};
    use crate::split::{
        MessageToCentral, MessageToPeripheral, MESSAGE_TO_CENTRAL_BUFFER_SIZE,
        MESSAGE_TO_PERIPHERAL_BUFFER_SIZE,
    };

    pub use rumcake_macros::setup_nrf_ble_split_central;

    pub struct NRFBLECentralDriver<'a> {
        publisher: Publisher<'a, RawMutex, MessageToPeripheral, 4, 4, 1>,
    }

    pub static BLE_MESSAGES_FROM_PERIPHERALS: Channel<RawMutex, MessageToCentral, 4> =
        Channel::new();

    pub static BLE_MESSAGES_TO_PERIPHERALS: PubSubChannel<RawMutex, MessageToPeripheral, 4, 4, 1> =
        PubSubChannel::new();

    pub static BLUETOOTH_CONNECTION_MUTEX: Mutex<RawMutex, ()> = Mutex::new(());

    /// Create an instance of the nRF bluetooth central device driver.
    pub fn setup_driver() -> NRFBLECentralDriver<'static> {
        NRFBLECentralDriver {
            publisher: BLE_MESSAGES_TO_PERIPHERALS.publisher().unwrap(),
        }
    }

    impl CentralDeviceDriver for NRFBLECentralDriver<'static> {
        type DriverError = ();

        async fn receive_message_from_peripherals(
            &mut self,
        ) -> Result<MessageToCentral, CentralDeviceError<Self::DriverError>> {
            let message = BLE_MESSAGES_FROM_PERIPHERALS.receive().await;

            Ok(message)
        }

        async fn broadcast_message_to_peripherals(
            &mut self,
            message: MessageToPeripheral,
        ) -> Result<(), CentralDeviceError<Self::DriverError>> {
            self.publisher.publish(message).await;

            Ok(())
        }
    }

    #[nrf_softdevice::gatt_client(uuid = "51a97f95-3492-4269-b5fd-32ac8dc72590")]
    struct SplitServiceClient {
        #[characteristic(uuid = "e35e4d4e-33f3-41e9-a526-edd36084dc0d", read, notify)]
        message_to_central: [u8; MESSAGE_TO_CENTRAL_BUFFER_SIZE],

        #[characteristic(uuid = "38668033-1c59-4877-8841-8eecf6d521f7", write)]
        message_to_peripheral: [u8; MESSAGE_TO_PERIPHERAL_BUFFER_SIZE],
    }

    pub async fn nrf_ble_central_task<const P: usize>(
        peripheral_addresses: &[[u8; 6]; P],
        sd: &'static Softdevice,
    ) {
        assert!(
            peripheral_addresses.len() <= 4,
            "You can not have more than 4 peripherals."
        );

        info!("[SPLIT_BT_DRIVER] Bluetooth services started");

        let peripheral_fut = |peripheral_addr: [u8; 6]| {
            async move {
                loop {
                    let whitelist = [&Address::new(AddressType::RandomStatic, peripheral_addr)];
                    let mut subscriber = BLE_MESSAGES_TO_PERIPHERALS.subscriber().unwrap();

                    let mut config = central::ConnectConfig::default();
                    config.scan_config.whitelist = Some(&whitelist);
                    config.conn_params.min_conn_interval = 6;
                    config.conn_params.max_conn_interval = 6;

                    let connection = {
                        let _lock = BLUETOOTH_CONNECTION_MUTEX.lock().await;
                        match connect(sd, &config).await {
                            Ok(connection) => {
                                info!("[SPLIT_BT_DRIVER] Connection established with peripheral");
                                connection
                            }
                            Err(error) => {
                                let ConnectError::Raw(RawError::BleGapWhitelistInUse) = error
                                else {
                                    warn!(
                                    "[SPLIT_BT_DRIVER] BLE connection error, disconnecting and retrying in 5 seconds: {}",
                                    Debug2Format(&error)
                                );
                                    continue;
                                };

                                // We don't log whitelist errors
                                continue;
                            }
                        }
                    };

                    let client: SplitServiceClient = match discover(&connection).await {
                        Ok(client) => client,
                        Err(error) => {
                            warn!(
                                "[SPLIT_BT_DRIVER] BLE GATT discovery error, retrying: {}",
                                Debug2Format(&error)
                            );
                            continue;
                        }
                    };

                    let client_fut = async {
                        // Enable notifications from the peripherals
                        client.message_to_central_cccd_write(true).await.unwrap();

                        gatt_client::run(&connection, &client, |event| match event {
                            SplitServiceClientEvent::MessageToCentralNotification(mut message) => {
                                let message = postcard::from_bytes_cobs(&mut message).unwrap();

                                match BLE_MESSAGES_FROM_PERIPHERALS.try_send(message) {
                                    Ok(()) => {
                                        debug!(
                                            "[SPLIT_BT_DRIVER] Consumed notification from peripheral: {:?}",
                                            Debug2Format(&message)
                                        )
                                    }
                                    Err(err) => {
                                        error!(
                                            "[SPLIT_BT_DRIVER] Could not consume notification from peripheral. data: {:?} error: {:?}",
                                            Debug2Format(&message),
                                            Debug2Format(&err)
                                        )
                                    }
                                };
                            }
                        }).await
                    };

                    let subscriber_fut = async {
                        // Discard any reports that haven't been processed due to lack of a connection
                        while subscriber.try_next_message_pure().is_some() {}

                        loop {
                            let message = subscriber.next_message_pure().await;

                            let mut buf = [0; MESSAGE_TO_PERIPHERAL_BUFFER_SIZE];
                            postcard::to_slice_cobs(&message, &mut buf).unwrap();

                            debug!(
                        "[SPLIT_BT_DRIVER] Notifying split keyboard message to peripheral: {:?}",
                        Debug2Format(&message)
                    );

                            if let Err(err) = client
                                .message_to_peripheral_write_without_response(&buf)
                                .await
                            {
                                error!(
                                    "[SPLIT_BT_DRIVER] Couldn't write message to peripheral: {:?}",
                                    Debug2Format(&err)
                                );
                            }
                        }
                    };

                    match select(client_fut, subscriber_fut).await {
                        Either::First(error) => {
                            warn!(
                        "[SPLIT_BT_DRIVER] Connection to peripheral lost. Attempting to reconnect. Error: {}",
                        Debug2Format(&error)
                    );
                        }
                        Either::Second(_) => {
                            error!(
                                "[SPLIT_BT_DRIVER] Subscriber task failed. This should not happen."
                            )
                        }
                    }
                }
            }
        };

        let futures = if let Ok(futures) = peripheral_addresses
            .iter()
            .map(|addr| peripheral_fut(*addr))
            .collect::<Vec<_, P>>()
            .into_array::<P>()
        {
            futures
        } else {
            panic!("Could not start nrf_ble_central_task");
        };

        select_array(futures).await;

        error!(
            "[SPLIT_BT_DRIVER] A peripheral connection task has completed. This should not happen."
        );
    }
}

#[cfg(feature = "split-peripheral")]
/// nrf-softdevice peripheral device driver implementations
pub mod peripheral {
    use defmt::{debug, error, info, warn, Debug2Format};
    use embassy_futures::select::{select, Either};
    use embassy_sync::channel::Channel;
    use nrf_softdevice::ble::gatt_server::{run, set_sys_attrs};
    use nrf_softdevice::ble::peripheral::{advertise_connectable, ConnectableAdvertisement};
    use nrf_softdevice::ble::{Address, AddressType};
    use nrf_softdevice::Softdevice;

    use crate::hw::platform::{RawMutex, BLUETOOTH_ADVERTISING_MUTEX};
    use crate::split::peripheral::{PeripheralDeviceDriver, PeripheralDeviceError};
    use crate::split::{
        MessageToCentral, MessageToPeripheral, MESSAGE_TO_CENTRAL_BUFFER_SIZE,
        MESSAGE_TO_PERIPHERAL_BUFFER_SIZE,
    };

    pub use rumcake_macros::setup_nrf_ble_split_peripheral;

    pub struct NRFBLEPeripheralDriver {}

    pub static BLE_MESSAGES_TO_CENTRAL: Channel<RawMutex, MessageToCentral, 4> = Channel::new();

    pub static BLE_MESSAGES_FROM_CENTRAL: Channel<RawMutex, MessageToPeripheral, 4> =
        Channel::new();

    /// Create an instance of the nRF bluetooth central device driver.
    pub fn setup_driver() -> NRFBLEPeripheralDriver {
        NRFBLEPeripheralDriver {}
    }

    impl PeripheralDeviceDriver for NRFBLEPeripheralDriver {
        type DriverError = ();

        async fn send_message_to_central(
            &mut self,
            message: MessageToCentral,
        ) -> Result<(), PeripheralDeviceError<Self::DriverError>> {
            BLE_MESSAGES_TO_CENTRAL.send(message).await;

            Ok(())
        }

        async fn receive_message_from_central(
            &mut self,
        ) -> Result<MessageToPeripheral, PeripheralDeviceError<Self::DriverError>> {
            let message = BLE_MESSAGES_FROM_CENTRAL.receive().await;

            Ok(message)
        }
    }

    #[nrf_softdevice::gatt_service(uuid = "51a97f95-3492-4269-b5fd-32ac8dc72590")]
    pub struct SplitService {
        #[characteristic(uuid = "e35e4d4e-33f3-41e9-a526-edd36084dc0d", read, notify)]
        message_to_central: [u8; MESSAGE_TO_CENTRAL_BUFFER_SIZE],

        #[characteristic(uuid = "38668033-1c59-4877-8841-8eecf6d521f7", write_without_response)]
        message_to_peripheral: [u8; MESSAGE_TO_PERIPHERAL_BUFFER_SIZE],
    }

    #[nrf_softdevice::gatt_server]
    pub struct PeripheralDeviceServer {
        split: SplitService,
    }

    pub async fn nrf_ble_peripheral_task(
        central_address: [u8; 6],
        sd: &'static Softdevice,
        server: PeripheralDeviceServer,
    ) {
        info!("[SPLIT_BT_DRIVER] Bluetooth services started");

        loop {
            let advertisement = ConnectableAdvertisement::NonscannableDirected {
                peer: Address::new(AddressType::RandomStatic, central_address),
            };
            let connection = {
                let _lock = BLUETOOTH_ADVERTISING_MUTEX.lock().await;
                match advertise_connectable(sd, advertisement, &Default::default()).await {
                    Ok(connection) => {
                        info!("[SPLIT_BT_DRIVER] Connection established with central");
                        connection
                    }
                    Err(error) => {
                        warn!(
                            "[SPLIT_BT_DRIVER] BLE advertising error: {}",
                            Debug2Format(&error)
                        );
                        continue;
                    }
                }
            };

            set_sys_attrs(&connection, None).unwrap();

            let server_fut = run(&connection, &server, |event| match event {
                PeripheralDeviceServerEvent::Split(split_event) => match split_event {
                    SplitServiceEvent::MessageToCentralCccdWrite { notifications } => {
                        debug!(
                            "[SPLIT_BT_DRIVER] Split value CCCD updated: {}",
                            notifications
                        );
                    }
                    SplitServiceEvent::MessageToPeripheralWrite(mut message) => {
                        let message = postcard::from_bytes_cobs(&mut message).unwrap();

                        match BLE_MESSAGES_FROM_CENTRAL.try_send(message) {
                            Ok(()) => {
                                debug!(
                                    "[SPLIT_BT_DRIVER] Consumed notification from central: {:?}",
                                    Debug2Format(&message)
                                );
                            }
                            Err(err) => {
                                error!(
                                    "[SPLIT_BT_DRIVER] Could not consume notification from peripheral. data: {:?} error: {:?}",
                                    Debug2Format(&message),
                                    Debug2Format(&err)
                                );
                            }
                        };
                    }
                },
            });

            let message_fut = async {
                // Discard any reports that haven't been processed due to lack of a connection
                while BLE_MESSAGES_TO_CENTRAL.try_receive().is_ok() {}

                loop {
                    let message = BLE_MESSAGES_TO_CENTRAL.receive().await;

                    let mut buf = [0; MESSAGE_TO_CENTRAL_BUFFER_SIZE];
                    postcard::to_slice_cobs(&message, &mut buf).unwrap();

                    debug!(
                        "[SPLIT_BT_DRIVER] Notifying split keyboard message to central: {:?}",
                        Debug2Format(&message)
                    );

                    if let Err(err) = server.split.message_to_central_notify(&connection, &buf) {
                        error!(
                            "[SPLIT_BT_DRIVER] Couldn't notify message to central: {:?}",
                            Debug2Format(&err)
                        );
                    }
                }
            };

            match select(server_fut, message_fut).await {
                Either::First(error) => {
                    warn!(
                        "[SPLIT_BT_DRIVER] Connection to central has been lost. Attempting to reconnect. Error: {}",
                        Debug2Format(&error)
                    )
                }
                Either::Second(_) => {
                    error!("[SPLIT_BT_DRIVER] Split message task failed. This should not happen.");
                }
            }
        }
    }
}
