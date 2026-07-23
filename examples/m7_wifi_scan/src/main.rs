#![no_std]
#![no_main]

use core::mem::MaybeUninit;

use aligned::{A4, Aligned};
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
use embassy_time::{Duration, Timer};
use panic_halt as _;
use static_cell::StaticCell;

const FACTORY_WIFI_FIRMWARE_LEN: usize = 421_098;
const FACTORY_WIFI_FIRMWARE_CRC32: u32 = 0xeafe_5f02;
#[allow(unsafe_code)]
#[unsafe(link_section = ".data.wifi_firmware")]
static FACTORY_WIFI_FIRMWARE: Aligned<A4, [u8; FACTORY_WIFI_FIRMWARE_LEN]> =
    Aligned(*include_bytes!("../../../firmware/4343WA1.bin"));
static FACTORY_WIFI_CLM: &[u8; 7_222] = include_bytes!("../../../firmware/4343WA1.clm_blob");

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & (0_u32.wrapping_sub(crc & 1)));
        }
    }
    !crc
}

// Arduino's GIGA NVRAM data for the onboard Murata Type 1DX (CYW4343W).
// The two MAC entries intentionally use a locally administered address.
static NVRAM: Aligned<A4, [u8; 562]> = Aligned(
    *b"manfid=0x2d0\0\
prodid=0x0726\0\
vendid=0x14e4\0\
devid=0x43e2\0\
boardtype=0x0726\0\
boardrev=0x1202\0\
boardnum=22\0\
macaddr=02:00:00:00:47:01\0\
sromrev=11\0\
boardflags=0x00404201\0\
boardflags3=0x04000000\0\
xtalfreq=37400\0\
nocrc=1\0\
ag0=0\0\
aa2g=1\0\
ccode=ALL\0\
extpagain2g=0\0\
pa2ga0=-145,6667,-751\0\
AvVmid_c0=0x0,0xc8\0\
cckpwroffset0=2\0\
maxp2ga0=74\0\
cckbw202gpo=0\0\
legofdmbw202gpo=0x88888888\0\
mcsbw202gpo=0xaaaaaaaa\0\
propbw202gpo=0xdd\0\
ofdmdigfilttype=18\0\
ofdmdigfilttypebe=18\0\
papdmode=1\0\
papdvalidtest=1\0\
pacalidx2g=48\0\
papdepsoffset=-22\0\
papdendidx=58\0\
il0macaddr=02:00:00:00:47:01\0\
wl0id=0x431b\0\
muxenab=0x10\0\0\0",
);

bind_interrupts!(struct Irqs {
    SDMMC1 => sdmmc::InterruptHandler<peripherals::SDMMC1>;
});

#[allow(unsafe_code)]
#[unsafe(link_section = ".shared_data")]
static SHARED_DATA: MaybeUninit<SharedData> = MaybeUninit::uninit();
static CYW43_STATE: StaticCell<cyw43::State> = StaticCell::new();

#[embassy_executor::main]
async fn main(_spawner: embassy_executor::Spawner) {
    // Arduino's MCUboot leaves the Cortex-M7 D-cache enabled. SDMMC1 uses IDMA,
    // and the current Embassy SDIO adapter does not maintain that cache around
    // DMA buffers. Disable D-cache before touching those buffers so firmware,
    // control, and scan transfers are coherent.
    let mut core = cortex_m::Peripherals::take().unwrap();
    core.SCB.disable_dcache(&mut core.CPUID);

    let mut mcu_config = embassy_stm32::Config::default();
    mcu_config.rcc.hsi = Some(HSIPrescaler::DIV1);
    mcu_config.rcc.csi = true;
    mcu_config.rcc.pll1 = Some(Pll {
        source: PllSource::HSI,
        prediv: PllPreDiv::DIV4,
        mul: PllMul::MUL50,
        divp: Some(PllDiv::DIV2),
        divq: Some(PllDiv::DIV4),
        divr: None,
    });
    mcu_config.rcc.sys = Sysclk::PLL1_P;
    mcu_config.rcc.d1c_pre = AHBPrescaler::DIV1;
    mcu_config.rcc.ahb_pre = AHBPrescaler::DIV2;
    mcu_config.rcc.apb1_pre = APBPrescaler::DIV2;
    mcu_config.rcc.apb2_pre = APBPrescaler::DIV2;
    mcu_config.rcc.apb3_pre = APBPrescaler::DIV2;
    mcu_config.rcc.apb4_pre = APBPrescaler::DIV2;
    mcu_config.rcc.voltage_scale = VoltageScale::Scale1;
    let p = embassy_stm32::init_primary(mcu_config, &SHARED_DATA);

    let mut red = Output::new(p.PI12, Level::High, Speed::Low);
    let mut green = Output::new(p.PJ13, Level::High, Speed::Low);
    let mut blue = Output::new(p.PE3, Level::High, Speed::Low);

    let firmware_crc32 = crc32(&FACTORY_WIFI_FIRMWARE[..]);
    #[cfg(feature = "defmt")]
    defmt::info!(
        "factory Wi-Fi firmware CRC32: {=u32:08x} (expected {=u32:08x})",
        firmware_crc32,
        FACTORY_WIFI_FIRMWARE_CRC32
    );
    if firmware_crc32 != FACTORY_WIFI_FIRMWARE_CRC32 {
        // Solid red means the bundled image failed its build-time integrity contract.
        red.set_low();
        loop {
            Timer::after(Duration::from_millis(250)).await;
        }
    }

    // Solid yellow means that the factory firmware was found and driver
    // initialization is in progress.
    red.set_low();
    green.set_low();

    let mut wifi_power = Output::new(p.PB10, Level::Low, Speed::Low);
    Timer::after(Duration::from_millis(250)).await;
    wifi_power.set_high();
    Timer::after(Duration::from_millis(500)).await;

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
    let sdio = SerialDataInterface::new(&mut sdmmc, Hertz::khz(400))
        .await
        .unwrap();
    let (_device, mut control, runner) = cyw43::new_sdio(
        CYW43_STATE.init(cyw43::State::new()),
        sdio,
        &FACTORY_WIFI_FIRMWARE,
        &NVRAM,
    )
    .await;

    let scan = async {
        control.init(FACTORY_WIFI_CLM).await;

        let mut scanner = control.scan(Default::default()).await;
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
            blue.set_high();
            if networks > 0 {
                red.set_high();
                green.set_low();
            } else {
                green.set_high();
                red.set_low();
            }
            Timer::after(Duration::from_millis(600)).await;

            red.set_high();
            green.set_high();
            blue.set_low();
            Timer::after(Duration::from_millis(300)).await;
        }
    };

    join(runner.run(), scan).await;
}
