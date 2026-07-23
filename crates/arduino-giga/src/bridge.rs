//! Lock-free Cortex-M7/Cortex-M4 shared-memory bridge.

use core::sync::atomic::{AtomicU32, Ordering};

use cortex_m::peripheral::MPU;

pub const BRIDGE_MAGIC: u32 = 0x4749_4741;
pub const PING_XOR: u32 = 0x5049_4e47;
pub const RESPONSE_XOR: u32 = 0x4d34_4f4b;

/// Configure the M7 MPU so both cores observe D3 SRAM accesses directly.
///
/// This matches the memory attributes used by Arduino's factory RPC support:
/// region 15 covers the complete 64 KiB D3 SRAM window at `0x3800_0000` and is
/// normal, non-cacheable, non-bufferable memory with full access.
///
/// Call this on the M7 before either the Embassy [`SharedData`] area or a
/// [`BridgeMailbox`] in D3 SRAM is accessed. The Cortex-M4 has no data cache and
/// does not require this configuration.
///
/// [`SharedData`]: https://docs.rs/embassy-stm32/latest/embassy_stm32/struct.SharedData.html
pub fn configure_m7_shared_sram(mpu: MPU) {
    const REGION_NUMBER: u32 = 15;
    const D3_SRAM_BASE: u32 = 0x3800_0000;
    const FULL_ACCESS: u32 = 0b011 << 24;
    const NORMAL_NON_CACHEABLE: u32 = 0b001 << 19;
    const REGION_SIZE_64_KIB: u32 = 15 << 1;
    const REGION_ENABLE: u32 = 1;
    const MPU_ENABLE_WITH_PRIVILEGED_DEFAULT_MAP: u32 = 0b101;

    cortex_m::asm::dmb();

    // Volatile MPU register writes are safe here because ownership of the
    // Cortex-M MPU singleton guarantees exclusive configuration access.
    #[allow(unsafe_code)]
    unsafe {
        mpu.ctrl.write(0);
        cortex_m::asm::dsb();
        cortex_m::asm::isb();

        mpu.rnr.write(REGION_NUMBER);
        mpu.rbar.write(D3_SRAM_BASE);
        mpu.rasr
            .write(FULL_ACCESS | NORMAL_NON_CACHEABLE | REGION_SIZE_64_KIB | REGION_ENABLE);
        mpu.ctrl.write(MPU_ENABLE_WITH_PRIVILEGED_DEFAULT_MAP);
    }

    cortex_m::asm::dsb();
    cortex_m::asm::isb();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct BridgeSnapshot {
    pub m4_heartbeat: u32,
    pub command_sequence: u32,
    pub command: u32,
    pub response_sequence: u32,
    pub response: u32,
}

/// Shared mailbox. Place one instance at the same non-cacheable SRAM address
/// in both core images.
#[repr(C, align(32))]
pub struct BridgeMailbox {
    magic: AtomicU32,
    m4_heartbeat: AtomicU32,
    command_sequence: AtomicU32,
    command: AtomicU32,
    response_sequence: AtomicU32,
    response: AtomicU32,
}

impl BridgeMailbox {
    pub const fn new() -> Self {
        Self {
            magic: AtomicU32::new(0),
            m4_heartbeat: AtomicU32::new(0),
            command_sequence: AtomicU32::new(0),
            command: AtomicU32::new(0),
            response_sequence: AtomicU32::new(0),
            response: AtomicU32::new(0),
        }
    }

    /// Reset the protocol before the M7 releases the M4 from reset.
    pub fn initialize_primary(&self) {
        self.magic.store(0, Ordering::SeqCst);
        self.m4_heartbeat.store(0, Ordering::Relaxed);
        self.command_sequence.store(0, Ordering::Relaxed);
        self.command.store(0, Ordering::Relaxed);
        self.response_sequence.store(0, Ordering::Relaxed);
        self.response.store(0, Ordering::Relaxed);
        self.magic.store(BRIDGE_MAGIC, Ordering::Release);
    }

    pub fn is_initialized(&self) -> bool {
        self.magic.load(Ordering::Acquire) == BRIDGE_MAGIC
    }

    pub fn publish_command(&self, sequence: u32, command: u32) {
        self.command.store(command, Ordering::Relaxed);
        self.command_sequence.store(sequence, Ordering::Release);
    }

    pub fn poll_command(&self, previous_sequence: u32) -> Option<(u32, u32)> {
        let sequence = self.command_sequence.load(Ordering::Acquire);
        if sequence == previous_sequence {
            None
        } else {
            Some((sequence, self.command.load(Ordering::Relaxed)))
        }
    }

    pub fn publish_response(&self, sequence: u32, response: u32) {
        self.response.store(response, Ordering::Relaxed);
        self.response_sequence.store(sequence, Ordering::Release);
    }

    pub fn response(&self, sequence: u32) -> Option<u32> {
        if self.response_sequence.load(Ordering::Acquire) == sequence {
            Some(self.response.load(Ordering::Relaxed))
        } else {
            None
        }
    }

    pub fn increment_m4_heartbeat(&self) {
        self.m4_heartbeat.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> BridgeSnapshot {
        BridgeSnapshot {
            m4_heartbeat: self.m4_heartbeat.load(Ordering::Relaxed),
            command_sequence: self.command_sequence.load(Ordering::Acquire),
            command: self.command.load(Ordering::Relaxed),
            response_sequence: self.response_sequence.load(Ordering::Acquire),
            response: self.response.load(Ordering::Relaxed),
        }
    }
}

impl Default for BridgeMailbox {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for BridgeMailbox {
    fn format(&self, formatter: defmt::Formatter) {
        let snapshot = self.snapshot();
        defmt::write!(formatter, "BridgeMailbox({})", snapshot);
    }
}
