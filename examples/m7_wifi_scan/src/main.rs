#![no_std]
#![no_main]

use core::mem::MaybeUninit;

use cortex_m_rt as _;
#[cfg(feature = "defmt")]
use defmt_rtt as _;
use embassy_futures::join::join;
use embassy_stm32::{
    SharedData, bind_interrupts,
    gpio::{Level, Output, Speed},
    peripherals,
    rcc::{
        AHBPrescaler, APBPrescaler, HSIPrescaler, Pll, PllDiv, PllMul, PllPreDiv, PllSource,
        Sysclk, VoltageScale,
    },
    sdmmc::{self, Sdmmc, sdio::SerialDataInterface},
    time::Hertz,
};
use embassy_time::{Delay, Duration, Timer};
use giga_r1::{
    led::{Color, RgbLed},
    wifi::{State, Wifi},
};
use panic_halt as _;
use static_cell::StaticCell;

bind_interrupts!(struct Irqs {
    SDMMC1 => sdmmc::InterruptHandler<peripherals::SDMMC1>;
});

#[allow(unsafe_code)]
#[unsafe(link_section = ".shared_data")]
static SHARED_DATA: MaybeUninit<SharedData> = MaybeUninit::uninit();
static CYW43_STATE: StaticCell<State> = StaticCell::new();

#[embassy_executor::main]
async fn main(_spawner: embassy_executor::Spawner) {
    // Arduino's MCUboot leaves D-cache enabled. SDMMC1 IDMA buffers are not
    // cache coherent, so disable it before constructing the SDIO transport.
    let mut core = cortex_m::Peripherals::take().unwrap();
    core.SCB.disable_dcache(&mut core.CPUID);

    let mut config = embassy_stm32::Config::default();
    config.rcc.hsi = Some(HSIPrescaler::DIV1);
    config.rcc.csi = true;
    config.rcc.pll1 = Some(Pll {
        source: PllSource::HSI,
        prediv: PllPreDiv::DIV4,
        mul: PllMul::MUL50,
        divp: Some(PllDiv::DIV2),
        divq: Some(PllDiv::DIV4),
        divr: None,
    });
    config.rcc.sys = Sysclk::PLL1_P;
    config.rcc.d1c_pre = AHBPrescaler::DIV1;
    config.rcc.ahb_pre = AHBPrescaler::DIV2;
    config.rcc.apb1_pre = APBPrescaler::DIV2;
    config.rcc.apb2_pre = APBPrescaler::DIV2;
    config.rcc.apb3_pre = APBPrescaler::DIV2;
    config.rcc.apb4_pre = APBPrescaler::DIV2;
    config.rcc.voltage_scale = VoltageScale::Scale1;
    let p = embassy_stm32::init_primary(config, &SHARED_DATA);

    let mut led = RgbLed::new(
        Output::new(p.PI12, Level::High, Speed::Low),
        Output::new(p.PJ13, Level::High, Speed::Low),
        Output::new(p.PE3, Level::High, Speed::Low),
    )
    .unwrap();
    led.set(Color::Yellow).unwrap();

    let mut sdmmc = Sdmmc::new_4bit(
        p.SDMMC1,
        Irqs,
        p.PC12,
        p.PD2,
        p.PC8,
        p.PC9,
        p.PC10,
        p.PC11,
        Default::default(),
    );
    let power = Output::new(p.PB10, Level::Low, Speed::Low);
    let wifi = Wifi::new(power).unwrap();
    let mut delay = Delay;
    let sdmmc = &mut sdmmc;
    let mut parts = wifi
        .start_async_with(CYW43_STATE.init(State::new()), &mut delay, move || {
            SerialDataInterface::new(sdmmc, Hertz::khz(400))
        })
        .await
        .unwrap();
    let runner = parts.take_runner().unwrap();

    let scan = async {
        parts.initialize().await;
        let mut scanner = parts.control.scan(Default::default()).await;
        let mut networks = 0_u16;
        while let Some(_network) = scanner.next().await {
            networks = networks.saturating_add(1);
            #[cfg(feature = "defmt")]
            defmt::info!(
                "Wi-Fi AP: SSID={=[u8]}, RSSI={=i16} dBm, channel={=u8}",
                &_network.ssid[..usize::from(_network.ssid_len)],
                _network.rssi,
                _network.ctl_ch
            );
        }

        #[cfg(feature = "defmt")]
        defmt::info!("Wi-Fi scan complete: {=u16} network(s)", networks);

        loop {
            led.set(if networks > 0 {
                Color::Green
            } else {
                Color::Red
            })
            .unwrap();
            Timer::after(Duration::from_millis(600)).await;
            led.set(Color::Blue).unwrap();
            Timer::after(Duration::from_millis(300)).await;
        }
    };

    join(runner.run(), scan).await;
}
