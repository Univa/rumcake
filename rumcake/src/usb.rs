use defmt::{debug, error, info, Debug2Format};
use embassy_futures::select::{self, select};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::pubsub::PubSubChannel;
use embassy_usb::class::hid::{Config, HidWriter, State};
use embassy_usb::driver::Driver;
use embassy_usb::{Builder, UsbDevice};
use packed_struct::PackedStruct;
use static_cell::StaticCell;
use usbd_human_interface_device::device::keyboard::{
    NKROBootKeyboardReport, NKRO_BOOT_KEYBOARD_REPORT_DESCRIPTOR,
};

use crate::keyboard::{Keyboard, KeyboardLayout, KEYBOARD_REPORT_HID_SEND_CHANNEL};
use crate::StaticArray;

pub static USB_STATE: PubSubChannel<ThreadModeRawMutex, bool, 2, 2, 2> = PubSubChannel::new();

pub trait USBKeyboard: Keyboard + KeyboardLayout {
    // USB Configuration
    const USB_VID: u16;
    const USB_PID: u16;
}

pub fn setup_usb_hid_nkro_writer(
    b: &mut Builder<'static, impl Driver<'static>>,
) -> HidWriter<
    'static,
    impl Driver<'static>,
    { <<NKROBootKeyboardReport as PackedStruct>::ByteArray as StaticArray>::LEN },
> {
    // Keyboard HID setup
    static KB_STATE: StaticCell<State> = StaticCell::new();
    let kb_state = KB_STATE.init(State::new());
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

#[rumcake_macros::task]
pub async fn start_usb(mut usb: UsbDevice<'static, impl Driver<'static>>) {
    loop {
        info!("[USB] USB started");
        usb.run_until_suspend().await;
        info!("[USB] USB suspended");
        usb.wait_resume().await;
    }
}

// TODO: media keys
#[rumcake_macros::task]
pub async fn usb_hid_kb_write_task(
    mut hid: HidWriter<
        'static,
        impl Driver<'static>,
        { <<NKROBootKeyboardReport as PackedStruct>::ByteArray as StaticArray>::LEN },
    >,
) {
    let mut usb_reports_enabled = cfg!(not(feature = "bluetooth"));
    let mut subscriber = USB_STATE.subscriber().unwrap();

    loop {
        if usb_reports_enabled {
            match select(
                subscriber.next_message_pure(),
                KEYBOARD_REPORT_HID_SEND_CHANNEL.receive(),
            )
            .await
            {
                select::Either::First(new_state) => {
                    usb_reports_enabled = new_state;
                    info!("[USB] USB HID reports enabled = {}", usb_reports_enabled);
                }
                select::Either::Second(report) => {
                    debug!(
                        "[USB] Writing HID keyboard report to USB: {:?}",
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
            usb_reports_enabled = subscriber.next_message_pure().await;
            info!("[USB] USB HID reports enabled = {}", usb_reports_enabled);

            // Ignore any unprocessed reports due to lack of a connection
            while KEYBOARD_REPORT_HID_SEND_CHANNEL.try_receive().is_ok() {}
        }
    }
}
