#![no_std]
#![no_main]

use core::mem::MaybeUninit;

#[cfg(feature = "defmt")]
use defmt_rtt as _;
use embassy_futures::join::join;
use embassy_stm32::{
    SharedData, bind_interrupts,
    gpio::{Level, Output, Speed},
    peripherals,
    rcc::{Hsi48Config, mux::Usbsel},
    usb::{self, Driver, Instance},
};
use embassy_usb::{
    Builder,
    class::cdc_acm::{CdcAcmClass, State},
    driver::EndpointError,
};
use panic_halt as _;

bind_interrupts!(struct Irqs {
    OTG_FS => usb::InterruptHandler<peripherals::USB_OTG_FS>;
});

#[allow(unsafe_code)]
#[unsafe(link_section = ".shared_data")]
static SHARED_DATA: MaybeUninit<SharedData> = MaybeUninit::uninit();

#[embassy_executor::main]
async fn main(_spawner: embassy_executor::Spawner) {
    let mut hal_config = embassy_stm32::Config::default();
    hal_config.rcc.hsi48 = Some(Hsi48Config {
        sync_from_usb: true,
    });
    hal_config.rcc.mux.usbsel = Usbsel::HSI48;
    let p = embassy_stm32::init_primary(hal_config, &SHARED_DATA);

    let mut red = Output::new(p.PI12, Level::High, Speed::Low);
    let mut green = Output::new(p.PJ13, Level::High, Speed::Low);
    let mut blue = Output::new(p.PE3, Level::Low, Speed::Low);

    let mut ep_out_buffer = [0_u8; 256];
    let mut driver_config = usb::Config::default();
    driver_config.vbus_detection = false;
    let driver = Driver::new_fs(
        p.USB_OTG_FS,
        Irqs,
        p.PA12,
        p.PA11,
        &mut ep_out_buffer,
        driver_config,
    );

    let mut usb_config = embassy_usb::Config::new(0x2341, 0x0066);
    usb_config.manufacturer = Some("Arduino GIGA Rust");
    usb_config.product = Some("GIGA R1 CDC echo");
    usb_config.serial_number = Some("GIGA-M7");

    let mut config_descriptor = [0_u8; 256];
    let mut bos_descriptor = [0_u8; 256];
    let mut control_buffer = [0_u8; 64];
    let mut cdc_state = State::new();
    let mut builder = Builder::new(
        driver,
        usb_config,
        &mut config_descriptor,
        &mut bos_descriptor,
        &mut [],
        &mut control_buffer,
    );
    let mut cdc = CdcAcmClass::new(&mut builder, &mut cdc_state, 64);
    let mut usb = builder.build();

    let usb_task = usb.run();
    let serial_task = async {
        loop {
            red.set_high();
            green.set_high();
            blue.set_low();
            cdc.wait_connection().await;

            blue.set_high();
            green.set_low();
            #[cfg(feature = "defmt")]
            defmt::info!("USB CDC configured");

            if echo_until_disconnect(&mut cdc).await.is_err() {
                #[cfg(feature = "defmt")]
                defmt::info!("USB CDC disconnected");
            }
        }
    };

    join(usb_task, serial_task).await;
}

async fn echo_until_disconnect<'d, T: Instance + 'd>(
    cdc: &mut CdcAcmClass<'d, Driver<'d, T>>,
) -> Result<(), EndpointError> {
    let mut buffer = [0_u8; 64];
    loop {
        let count = cdc.read_packet(&mut buffer).await?;
        cdc.write_packet(&buffer[..count]).await?;
    }
}
