#![no_std]
#![no_main]

use core::mem::MaybeUninit;

use cortex_m_rt as _;
#[cfg(feature = "defmt")]
use defmt_rtt as _;
use embassy_futures::select::{Either, select};
use embassy_stm32::{
    SharedData, bind_interrupts,
    gpio::{Input, Level, Output, Pull, Speed},
    peripherals,
    usart::{self, BufferedUart, Config},
};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_time::{Delay, Duration};
use giga_r1::{
    ble::BleResources,
    led::{Color, RgbLed},
};
use panic_halt as _;
use trouble_host::prelude::*;

bind_interrupts!(struct Irqs {
    UART7 => usart::BufferedInterruptHandler<peripherals::UART7>;
});

#[allow(unsafe_code)]
#[unsafe(link_section = ".shared_data")]
static SHARED_DATA: MaybeUninit<SharedData> = MaybeUninit::uninit();

#[embassy_executor::main]
async fn main(_spawner: embassy_executor::Spawner) {
    let p = embassy_stm32::init_primary(embassy_stm32::Config::default(), &SHARED_DATA);
    let mut led = RgbLed::new(
        Output::new(p.PI12, Level::High, Speed::Low),
        Output::new(p.PJ13, Level::High, Speed::Low),
        Output::new(p.PE3, Level::High, Speed::Low),
    )
    .unwrap();
    led.set(Color::Yellow).unwrap();
    #[cfg(feature = "defmt")]
    defmt::info!("BLE: configuring UART7");

    let mut uart_config = Config::default();
    uart_config.baudrate = giga_r1::ble::INITIAL_BAUD;
    let mut tx_buffer = [0_u8; 512];
    let mut rx_buffer = [0_u8; 512];
    let uart = BufferedUart::new_with_rtscts(
        p.UART7,
        p.PA8,
        p.PF7,
        p.PF8,
        p.PF9,
        Irqs,
        &mut tx_buffer,
        &mut rx_buffer,
        uart_config,
    )
    .unwrap();

    #[cfg(feature = "defmt")]
    defmt::info!("BLE: starting CYW4343W controller");
    let power = Output::new(p.PA10, Level::Low, Speed::Low);
    let host_wake = Input::new(p.PG3, Pull::None);
    let device_wake = Output::new(p.PH7, Level::High, Speed::Low);
    let mut delay = Delay;
    let controller = BleResources::new(uart, power, host_wake, device_wake)
        .start_async(&mut delay)
        .await
        .unwrap();
    #[cfg(feature = "defmt")]
    defmt::info!("BLE: power sequence complete");
    let controller = controller.initialize_hci(&mut delay).await.unwrap();
    #[cfg(feature = "defmt")]
    defmt::info!("BLE: Cypress patchram initialized");

    let parts = controller.into_parts();
    let (tx, rx) = parts.uart.split();
    let _control_lines = (parts.power, parts.host_wake, parts.device_wake);

    let transport: SerialTransport<NoopRawMutex, _, _> = SerialTransport::new(rx, tx);
    let controller: ExternalController<_, 4> = ExternalController::new(transport);
    let address = Address::random([0x47, 0x49, 0x47, 0x41, 0x02, 0xc0]);
    let mut resources = HostResources::<_, DefaultPacketPool, 0, 0>::new();
    let stack = trouble_host::new(controller, &mut resources)
        .set_random_address(address)
        .build();
    #[cfg(feature = "defmt")]
    defmt::info!("BLE: host stack built; starting runner");
    let mut runner = stack.runner();
    let mut peripheral = stack.peripheral();

    let mut advertisement = [0_u8; 31];
    let service_data = [
        0xaa, 0xfe, // Eddystone UUID, little endian
        0x00, // UID frame
        0xee, // calibrated RSSI at 0 m
        0x47, 0x49, 0x47, 0x41, 0x2d, 0x52, 0x31, 0x2d, 0x42, 0x4c, // namespace
        0x45, 0x2d, 0x30, 0x30, 0x30, 0x32, // instance
        0x00, 0x00, // reserved
    ];
    let length = AdStructure::encode_slice(
        &[
            AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
            AdStructure::ServiceData16 {
                uuid: [0xaa, 0xfe],
                data: &service_data[2..],
            },
        ],
        &mut advertisement,
    )
    .unwrap();

    let beacon = async {
        let parameters = AdvertisementParameters {
            interval_min: Duration::from_millis(100),
            interval_max: Duration::from_millis(150),
            ..Default::default()
        };
        let _advertiser = peripheral
            .advertise(
                &parameters,
                Advertisement::NonconnectableNonscannableUndirected {
                    adv_data: &advertisement[..length],
                },
            )
            .await
            .unwrap();
        #[cfg(feature = "defmt")]
        defmt::info!("BLE: advertising active");
        led.set(Color::Green).unwrap();
        core::future::pending::<()>().await;
    };

    if let Either::First(_runner_result) = select(runner.run(), beacon).await {
        #[cfg(feature = "defmt")]
        defmt::error!("BLE host runner stopped: {:?}", _runner_result);
        led.set(Color::Red).unwrap();
        core::future::pending::<()>().await;
    }
}
