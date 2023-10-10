pub mod central {
    use defmt::{debug, error, info, warn, Debug2Format};
    use embassy_futures::select::{select, select_slice, Either};
    use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
    use embassy_sync::channel::Channel;
    use embassy_sync::pubsub::{PubSubChannel, Publisher};
    use embassy_time::{Duration, Timer};
    use heapless::Vec;
    use nrf_softdevice::ble::central::{connect, ConnectError};
    use nrf_softdevice::ble::gatt_client::{self, discover};
    use nrf_softdevice::ble::{central, set_address, Address, AddressType};
    use nrf_softdevice::{RawError, Softdevice};

    use rumcake::split::drivers::{CentralDeviceDriver, CentralDeviceError};
    use rumcake::split::{MessageToCentral, MessageToPeripheral};

    pub struct NRFBLECentralDriver<'a> {
        publisher: Publisher<'a, ThreadModeRawMutex, MessageToPeripheral, 4, 4, 1>,
    }

    pub trait NRFBLECentralDevice<const N: usize = 1> {
        const NUM_PERIPHERALS: usize = 1;
        const BLUETOOTH_ADDRESS: [u8; 6];
        const PERIPHERAL_ADDRESSES: [[u8; 6]; N];
    }

    pub static BLE_MESSAGES_FROM_PERIPHERALS: Channel<ThreadModeRawMutex, MessageToCentral, 4> =
        Channel::new();

    pub static BLE_MESSAGES_TO_PERIPHERALS: PubSubChannel<
        ThreadModeRawMutex,
        MessageToPeripheral,
        4,
        4,
        1,
    > = PubSubChannel::new();

    pub fn setup_split_central_driver<const N: usize, K: NRFBLECentralDevice<N>>(
        _k: K,
    ) -> NRFBLECentralDriver<'static> {
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
        message_to_central: [u8; 7],

        #[characteristic(uuid = "38668033-1c59-4877-8841-8eecf6d521f7", write)]
        message_to_peripheral: [u8; 7],
    }

    #[rumcake_macros::task]
    pub async fn nrf_ble_central_task<const N: usize, K: NRFBLECentralDevice<N>>(
        _k: K,
        sd: &'static Softdevice,
    ) {
        if N > 4 {
            panic!("You can not have more than 4 peripherals.");
        }

        set_address(
            sd,
            &Address::new(AddressType::RandomStatic, K::BLUETOOTH_ADDRESS),
        );

        info!("[SPLIT_BT_DRIVER] Bluetooth services started");

        let peripheral_fut = |peripheral_addr: &'static [u8; 6]| {
            async {
                loop {
                    let whitelist = [&Address::new(AddressType::RandomStatic, *peripheral_addr)];
                    let mut subscriber = BLE_MESSAGES_TO_PERIPHERALS.subscriber().unwrap();

                    let mut config = central::ConnectConfig::default();
                    config.scan_config.whitelist = Some(&whitelist);
                    config.conn_params.min_conn_interval = 6;
                    config.conn_params.max_conn_interval = 6;

                    let connection = match connect(sd, &config).await {
                        Ok(connection) => {
                            info!("[SPLIT_BT_DRIVER] Connection established with peripheral");
                            connection
                        }
                        Err(error) => {
                            let ConnectError::Raw(RawError::BleGapWhitelistInUse) = error else {
                                warn!(
                                    "[SPLIT_BT_DRIVER] BLE connection error, disconnecting and retrying in 5 seconds: {}",
                                    Debug2Format(&error)
                                );
                                Timer::after(Duration::from_secs(5)).await;
                                continue;
                            };

                            // We ignore whitelist errors, immediately retry if that's the case
                            Timer::after(Duration::from_secs(5)).await;
                            continue;
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

                            let mut buf = [0; 7];
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

        select_slice(
            &mut K::PERIPHERAL_ADDRESSES
                .iter()
                .map(|addr| peripheral_fut(addr))
                .collect::<Vec<_, N>>(),
        )
        .await;

        error!(
            "[SPLIT_BT_DRIVER] A peripheral connection task has completed. This should not happen."
        );
    }
}

pub mod peripheral {
    use defmt::{debug, error, info, warn, Debug2Format};
    use embassy_futures::select::{select, Either};
    use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
    use embassy_sync::channel::Channel;
    use nrf_softdevice::ble::gatt_server::{run, set_sys_attrs};
    use nrf_softdevice::ble::peripheral::{advertise_connectable, ConnectableAdvertisement};
    use nrf_softdevice::ble::{set_address, Address, AddressType};
    use nrf_softdevice::Softdevice;
    use rumcake::split::drivers::{PeripheralDeviceDriver, PeripheralDeviceError};
    use rumcake::split::{MessageToCentral, MessageToPeripheral};

    pub trait NRFBLEPeripheralDevice {
        const BLUETOOTH_ADDRESS: [u8; 6];
        const CENTRAL_ADDRESS: [u8; 6];
    }

    pub struct NRFBLEPeripheralDriver {}

    pub static BLE_MESSAGES_TO_CENTRAL: Channel<ThreadModeRawMutex, MessageToCentral, 4> =
        Channel::new();

    pub static BLE_MESSAGES_FROM_CENTRAL: Channel<ThreadModeRawMutex, MessageToPeripheral, 4> =
        Channel::new();

    pub fn setup_split_peripheral_driver<K: NRFBLEPeripheralDevice>() -> NRFBLEPeripheralDriver {
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
        message_to_central: [u8; 7],

        #[characteristic(uuid = "38668033-1c59-4877-8841-8eecf6d521f7", write_without_response)]
        message_to_peripheral: [u8; 7],
    }

    #[nrf_softdevice::gatt_server]
    pub struct PeripheralDeviceServer {
        split: SplitService,
    }

    #[rumcake_macros::task]
    pub async fn nrf_ble_peripheral_task<K: NRFBLEPeripheralDevice>(
        sd: &'static Softdevice,
        server: PeripheralDeviceServer,
    ) {
        set_address(
            sd,
            &Address::new(AddressType::RandomStatic, K::BLUETOOTH_ADDRESS),
        );

        info!("[SPLIT_BT_DRIVER] Bluetooth services started");

        loop {
            let advertisement = ConnectableAdvertisement::NonscannableDirected {
                peer: Address::new(AddressType::RandomStatic, K::CENTRAL_ADDRESS),
            };
            let connection =
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

                    let mut buf = [0; 7];
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
