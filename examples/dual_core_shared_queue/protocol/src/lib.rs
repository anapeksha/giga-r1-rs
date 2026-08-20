#![no_std]

use core::mem::size_of;

use giga_r1::ipc::{QueueError, SharedQueue};

/// Capacity of one binary block in 32-bit words.
pub const BLOCK_WORDS: usize = 384;
/// Capacity of one binary block in bytes.
pub const BLOCK_BYTES: usize = BLOCK_WORDS * size_of::<u32>();
/// Number of outstanding blocks supported in each direction.
pub const SLOT_COUNT: usize = 4;
/// Size reserved by each image for the bulk mailbox.
pub const BULK_REGION_SIZE: usize = 16 * 1024;

/// Fixed-block SPSC queue used in each direction.
pub type BlockQueue = SharedQueue<BLOCK_WORDS, SLOT_COUNT>;

/// Bidirectional mailbox shared at the same address by both core images.
#[repr(C, align(32))]
pub struct BulkMailbox {
    pub m7_to_m4: BlockQueue,
    pub m4_to_m7: BlockQueue,
}

impl BulkMailbox {
    /// Construct uninitialized queue storage for the shared linker section.
    pub const fn new() -> Self {
        Self {
            m7_to_m4: BlockQueue::new(),
            m4_to_m7: BlockQueue::new(),
        }
    }

    /// Reset and publish both queues before the M7 releases the M4.
    pub fn initialize_primary(&self) -> Result<(), QueueError> {
        self.m7_to_m4.initialize_primary()?;
        self.m4_to_m7.initialize_primary()
    }

    /// Return whether the M7 has published both initialized queues.
    pub fn is_initialized(&self) -> bool {
        self.m7_to_m4.is_initialized() && self.m4_to_m7.is_initialized()
    }
}

impl Default for BulkMailbox {
    fn default() -> Self {
        Self::new()
    }
}

const _: () = assert!(size_of::<BulkMailbox>() <= BULK_REGION_SIZE);
