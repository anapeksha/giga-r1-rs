#![no_std]
#![no_main]

use core::mem::MaybeUninit;

#[cfg(feature = "defmt")]
use defmt_rtt as _;
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
use panic_halt as _;

bind_interrupts!(struct Irqs {
    SDMMC1 => sdmmc::InterruptHandler<peripherals::SDMMC1>;
});

#[allow(unsafe_code)]
#[unsafe(link_section = ".shared_data")]
static SHARED_DATA: MaybeUninit<SharedData> = MaybeUninit::uninit();

#[embassy_executor::main]
async fn main(_spawner: embassy_executor::Spawner) {
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
    let mut blue = Output::new(p.PE3, Level::Low, Speed::Low);

    // WL_REG_ON must start low so every run probes from a known radio state.
    let mut wifi_power = Output::new(p.PB10, Level::Low, Speed::Low);
    cortex_m::asm::delay(100_000_000);
    wifi_power.set_high();
    cortex_m::asm::delay(100_000_000);

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

    // Failure stages: 1=SDIO init, 2=CCCR read, 3=capability read,
    // 4=unexpected CCCR contents.
    let (passed, cccr_revision, card_capability, failure_stage) =
        match SerialDataInterface::new(&mut sdmmc, Hertz::khz(400)).await {
            Ok(mut sdio) => {
                let revision = sdio.cmd52(0).await;
                let capability = sdio.cmd52(0x08 << 9).await;
                match (revision, capability) {
                    (Ok(revision), Ok(capability)) => {
                        const R5_ERROR_MASK: u16 = 0xcb00;
                        let revision_byte = revision as u8;
                        let valid_revision = revision & R5_ERROR_MASK == 0
                            && (1..=4).contains(&(revision_byte & 0x0f));
                        let valid_capability = capability & R5_ERROR_MASK == 0;
                        (
                            valid_revision && valid_capability,
                            revision_byte,
                            capability as u8,
                            if valid_revision && valid_capability {
                                0
                            } else {
                                4
                            },
                        )
                    }
                    (Err(_), _) => (false, 0, 0, 2),
                    (_, Err(_)) => (false, 0, 0, 3),
                }
            }
            Err(_) => (false, 0, 0, 1),
        };

    #[cfg(feature = "defmt")]
    if passed {
        defmt::info!(
            "Wi-Fi SDIO probe passed: CCCR={=u8:#x}, capability={=u8:#x}",
            cccr_revision,
            card_capability
        );
    } else {
        defmt::error!(
            "Wi-Fi SDIO probe failed: stage={=u8}, CCCR={=u8:#x}, capability={=u8:#x}",
            failure_stage,
            cccr_revision,
            card_capability
        );
    }
    #[cfg(not(feature = "defmt"))]
    let _ = (cccr_revision, card_capability, failure_stage);

    loop {
        #[cfg(feature = "defmt")]
        if !passed {
            defmt::error!(
                "Wi-Fi SDIO probe failure stage={=u8}, CCCR={=u8:#x}, capability={=u8:#x}",
                failure_stage,
                cccr_revision,
                card_capability
            );
        }
        blue.set_high();
        if passed {
            red.set_high();
            green.set_low();
        } else {
            green.set_high();
            red.set_low();
        }
        cortex_m::asm::delay(200_000_000);

        red.set_high();
        green.set_high();
        blue.set_low();
        cortex_m::asm::delay(100_000_000);
    }
}
