//! USB host communication.
//!
//! To use USB host communication, keyboards must implement [`USBKeyboard`].

use defmt::{error, info, Debug2Format};
use embassy_futures::select::{self, select};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;
use embassy_usb::class::hid::{Config, HidWriter, State as UsbState};
use embassy_usb::driver::Driver;
use embassy_usb::{Builder, UsbDevice};
use packed_struct::PackedStruct;
use static_cell::StaticCell;
use usbd_human_interface_device::device::consumer::{
    MultipleConsumerReport, MULTIPLE_CODE_REPORT_DESCRIPTOR,
};
use usbd_human_interface_device::device::keyboard::{
    NKROBootKeyboardReport, NKRO_BOOT_KEYBOARD_REPORT_DESCRIPTOR,
};

use crate::hw::{HIDOutput, OutputMode, CURRENT_OUTPUT_STATE};
use crate::keyboard::{
    Keyboard, KeyboardLayout, CONSUMER_REPORT_HID_SEND_CHANNEL, KEYBOARD_REPORT_HID_SEND_CHANNEL,
};
use crate::{State, StaticArray};

pub(crate) static CURRENT_OUTPUT_STATE_LISTENER: Signal<ThreadModeRawMutex, ()> = Signal::new();

pub(crate) static USB_RUNNING_STATE: State<bool> =
    State::new(false, &[&crate::hw::USB_RUNNING_STATE_LISTENER]);

/// A trait that keyboards must implement to communicate with host devices over USB.
pub trait USBKeyboard: Keyboard + KeyboardLayout {
    /// Vendor ID for the keyboard.
    const USB_VID: u16;

    /// Product ID for the keyboard.
    const USB_PID: u16;
}

/// Configure the HID report writer, using boot-specification-compatible NKRO keyboard reports.
///
/// The HID writer produced should be passed to [`usb_hid_kb_write_task`].
pub fn setup_usb_hid_nkro_writer(
    b: &mut Builder<'static, impl Driver<'static>>,
) -> HidWriter<
    'static,
    impl Driver<'static>,
    { <<NKROBootKeyboardReport as PackedStruct>::ByteArray as StaticArray>::LEN },
> {
    // Keyboard HID setup
    static KB_STATE: StaticCell<UsbState> = StaticCell::new();
    let kb_state = KB_STATE.init(UsbState::new());
    let kb_hid_config = Config {
        request_handler: None,
        report_descriptor: NKRO_BOOT_KEYBOARD_REPORT_DESCRIPTOR,
        poll_ms: 1,
        max_packet_size: 64,
    };
    HidWriter::<_, { <<NKROBootKeyboardReport as PackedStruct>::ByteArray as StaticArray>::LEN }>::new(
        b,
        kb_state,
        kb_hid_config,
    )
}

/// Configure the HID report writer, for consumer commands.
///
/// The HID writer produced should be passed to [`usb_hid_consumer_write_task`].
pub fn setup_usb_hid_consumer_writer(
    b: &mut Builder<'static, impl Driver<'static>>,
) -> HidWriter<
    'static,
    impl Driver<'static>,
    { <<MultipleConsumerReport as PackedStruct>::ByteArray as StaticArray>::LEN },
> {
    // Keyboard HID setup
    static CONSUMER_STATE: StaticCell<UsbState> = StaticCell::new();
    let consumer_state = CONSUMER_STATE.init(UsbState::new());
    let consumer_hid_config = Config {
        request_handler: None,
        report_descriptor: MULTIPLE_CODE_REPORT_DESCRIPTOR,
        poll_ms: 1,
        max_packet_size: 64,
    };
    HidWriter::<_, { <<MultipleConsumerReport as PackedStruct>::ByteArray as StaticArray>::LEN }>::new(
        b,
        consumer_state,
        consumer_hid_config,
    )
}

#[rumcake_macros::task]
pub async fn start_usb(mut usb: UsbDevice<'static, impl Driver<'static>>) {
    loop {
        info!("[USB] USB started");
        USB_RUNNING_STATE.set(true).await;
        usb.run_until_suspend().await;
        info!("[USB] USB suspended");
        USB_RUNNING_STATE.set(false).await;
        usb.wait_resume().await;
    }
}

#[rumcake_macros::task]
pub async fn usb_hid_kb_write_task(
    mut hid: HidWriter<
        'static,
        impl Driver<'static>,
        { <<NKROBootKeyboardReport as PackedStruct>::ByteArray as StaticArray>::LEN },
    >,
) {
    loop {
        if matches!(CURRENT_OUTPUT_STATE.get().await, Some(HIDOutput::Usb)) {
            match select(
                CURRENT_OUTPUT_STATE_LISTENER.wait(),
                KEYBOARD_REPORT_HID_SEND_CHANNEL.receive(),
            )
            .await
            {
                select::Either::First(()) => {}
                select::Either::Second(report) => {
                    info!(
                        "[USB] Writing NKRO HID keyboard report to USB: {:?}",
                        Debug2Format(&report)
                    );
                    if let Err(err) = hid.write(&report.pack().unwrap()).await {
                        error!(
                            "[USB] Couldn't write HID keyboard report: {:?}",
                            Debug2Format(&err)
                        );
                    };
                }
            }
        } else {
            CURRENT_OUTPUT_STATE_LISTENER.wait().await;

            // Ignore any unprocessed reports due to lack of a connection
            while KEYBOARD_REPORT_HID_SEND_CHANNEL.try_receive().is_ok() {}
        }
    }
}

#[rumcake_macros::task]
pub async fn usb_hid_consumer_write_task(
    mut hid: HidWriter<
        'static,
        impl Driver<'static>,
        { <<MultipleConsumerReport as PackedStruct>::ByteArray as StaticArray>::LEN },
    >,
) {
    loop {
        if matches!(CURRENT_OUTPUT_STATE.get().await, Some(HIDOutput::Usb)) {
            match select(
                CURRENT_OUTPUT_STATE_LISTENER.wait(),
                CONSUMER_REPORT_HID_SEND_CHANNEL.receive(),
            )
            .await
            {
                select::Either::First(()) => {}
                select::Either::Second(report) => {
                    info!(
                        "[USB] Writing consumer HID report to USB: {:?}",
                        Debug2Format(&report)
                    );
                    if let Err(err) = hid.write(&report.pack().unwrap()).await {
                        error!(
                            "[USB] Couldn't write consumer HID report: {:?}",
                            Debug2Format(&err)
                        );
                    };
                }
            }
        } else {
            CURRENT_OUTPUT_STATE_LISTENER.wait().await;

            // Ignore any unprocessed reports due to lack of a connection
            while CONSUMER_REPORT_HID_SEND_CHANNEL.try_receive().is_ok() {}
        }
    }
}
