use core::{
    mem::size_of,
    sync::atomic::{AtomicU32, Ordering},
};

use super::{AsyncWait, Notify, Polling};

const QUEUE_MAGIC: u32 = 0x5351_5031;

/// Error returned by a shared SPSC queue operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum QueueError {
    /// The M7 has not initialized the queue yet.
    NotInitialized,
    /// The queue has zero-sized blocks, no slots, or dimensions too large for
    /// its 32-bit metadata and wrapping counter protocol.
    InvalidConfiguration,
    /// The two core images disagree about the queue's block size or slot count.
    LayoutMismatch,
    /// The queue was reinitialized after this endpoint was claimed.
    StaleEndpoint,
    /// A producer endpoint is already active for this queue.
    ProducerClaimed,
    /// A consumer endpoint is already active for this queue.
    ConsumerClaimed,
    /// Every slot currently contains an outstanding entry.
    Full,
    /// No published entry is available.
    Empty,
    /// The payload does not fit in one queue block.
    PayloadTooLarge,
    /// The destination slice is smaller than the next published payload.
    BufferTooSmall { required: usize },
    /// Shared metadata contains a payload length larger than the block capacity.
    CorruptLength,
}

#[repr(C)]
struct QueueSlot<const WORDS: usize> {
    length: AtomicU32,
    payload: [AtomicU32; WORDS],
}

impl<const WORDS: usize> QueueSlot<WORDS> {
    const fn new() -> Self {
        Self {
            length: AtomicU32::new(0),
            payload: [const { AtomicU32::new(0) }; WORDS],
        }
    }
}

/// Bounded single-producer/single-consumer queue for shared-memory byte blocks.
///
/// `WORDS` is the capacity of each block in 32-bit words and `N` is the number
/// of outstanding blocks the queue can hold. For example,
/// `SharedQueue<384, 4>` stores four blocks of up to 1,536 bytes. The queue's
/// exact memory cost is available as [`Self::MEMORY_SIZE`]. Applications choose
/// the queue's linker region explicitly; the BSP does not reserve D3 SRAM beyond
/// the existing mailbox convention.
///
/// # Memory ordering
///
/// The producer writes payload words and length with relaxed stores, then
/// publishes the new producer position with `Release`. The consumer observes
/// that position with `Acquire` before reading the slot. After copying a block,
/// the consumer releases its position; the producer acquires it before reusing
/// the slot. This permits multiple queued blocks without a mutex.
///
/// # Shared-memory safety
///
/// Payload storage consists only of atomic words. [`Producer`] accepts an
/// ordinary input slice and [`Consumer`] copies into an ordinary output slice;
/// neither endpoint returns a reference into memory concurrently accessed by
/// the other core. One producer and one consumer are enforced with shared claim
/// flags, but assigning those roles to the intended cores remains an application
/// invariant.
///
/// Place the same concrete queue type at the same address in both images using
/// a `NOLOAD`, `KEEP`, 32-byte-aligned linker section. On M7, configure the D3
/// SRAM region as normal, non-cacheable, non-bufferable memory before accessing
/// the queue. Call [`Self::initialize_primary`] before releasing M4. A new
/// initialization generation invalidates old endpoints so they cannot clear or
/// share new claims, but coordinated reset is still required because queued data
/// is discarded. The peer core must be held in reset or otherwise quiescent.
#[repr(C, align(32))]
pub struct SharedQueue<const WORDS: usize, const N: usize> {
    magic: AtomicU32,
    generation: AtomicU32,
    layout_words: AtomicU32,
    layout_slots: AtomicU32,
    producer_claimed: AtomicU32,
    consumer_claimed: AtomicU32,
    producer_position: AtomicU32,
    consumer_position: AtomicU32,
    slots: [QueueSlot<WORDS>; N],
}

impl<const WORDS: usize, const N: usize> SharedQueue<WORDS, N> {
    /// Maximum payload size of one block, in bytes.
    pub const BLOCK_CAPACITY: usize = WORDS * size_of::<u32>();
    /// Maximum number of outstanding blocks.
    pub const SLOT_COUNT: usize = N;
    /// Exact shared-memory footprint of this queue, including metadata and
    /// alignment padding.
    pub const MEMORY_SIZE: usize = size_of::<Self>();

    /// Construct zeroed queue storage for placement in a shared linker section.
    ///
    /// The queue is not usable until M7 calls [`Self::initialize_primary`].
    pub const fn new() -> Self {
        Self {
            magic: AtomicU32::new(0),
            generation: AtomicU32::new(0),
            layout_words: AtomicU32::new(0),
            layout_slots: AtomicU32::new(0),
            producer_claimed: AtomicU32::new(0),
            consumer_claimed: AtomicU32::new(0),
            producer_position: AtomicU32::new(0),
            consumer_position: AtomicU32::new(0),
            slots: [const { QueueSlot::new() }; N],
        }
    }

    /// Reset and publish the queue before M7 releases M4.
    ///
    /// Payload words are intentionally not cleared: resetting the positions
    /// makes stale slots unreachable until the producer overwrites and publishes
    /// them. Existing endpoints become [`QueueError::StaleEndpoint`] and cannot
    /// release claims from the new generation. The caller must still ensure the
    /// peer core is held in reset or otherwise quiescent because queued data is
    /// discarded.
    pub fn initialize_primary(&self) -> Result<(), QueueError> {
        if !Self::configuration_is_valid() {
            return Err(QueueError::InvalidConfiguration);
        }

        self.magic.store(0, Ordering::SeqCst);
        let generation = self
            .generation
            .load(Ordering::Relaxed)
            .wrapping_add(1)
            .max(1);
        self.generation.store(generation, Ordering::Relaxed);
        self.layout_words.store(WORDS as u32, Ordering::Relaxed);
        self.layout_slots.store(N as u32, Ordering::Relaxed);
        self.producer_claimed.store(0, Ordering::Relaxed);
        self.consumer_claimed.store(0, Ordering::Relaxed);
        self.producer_position.store(0, Ordering::Relaxed);
        self.consumer_position.store(0, Ordering::Relaxed);
        for slot in &self.slots {
            slot.length.store(0, Ordering::Relaxed);
        }
        self.magic.store(QUEUE_MAGIC, Ordering::Release);
        Ok(())
    }

    /// Return whether the primary core has published initialized queue state.
    pub fn is_initialized(&self) -> bool {
        self.current_generation().is_ok()
    }

    /// Claim the queue's sole producer with polling notification.
    pub fn producer(&self) -> Result<Producer<'_, WORDS, N, Polling>, QueueError> {
        self.producer_with_notifier(Polling)
    }

    /// Claim the queue's sole producer with an application-supplied doorbell.
    pub fn producer_with_notifier<D>(
        &self,
        notifier: D,
    ) -> Result<Producer<'_, WORDS, N, D>, QueueError> {
        let generation = self.current_generation()?;
        self.producer_claimed
            .compare_exchange(0, generation, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| QueueError::ProducerClaimed)?;
        if self.check_endpoint(generation).is_err() {
            let _ = self.producer_claimed.compare_exchange(
                generation,
                0,
                Ordering::AcqRel,
                Ordering::Acquire,
            );
            return Err(QueueError::StaleEndpoint);
        }
        Ok(Producer {
            queue: self,
            notifier: Some(notifier),
            generation,
        })
    }

    /// Claim the queue's sole consumer with polling notification.
    pub fn consumer(&self) -> Result<Consumer<'_, WORDS, N, Polling>, QueueError> {
        self.consumer_with_notifier(Polling)
    }

    /// Claim the queue's sole consumer with an application-supplied doorbell.
    pub fn consumer_with_notifier<D>(
        &self,
        notifier: D,
    ) -> Result<Consumer<'_, WORDS, N, D>, QueueError> {
        let generation = self.current_generation()?;
        self.consumer_claimed
            .compare_exchange(0, generation, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| QueueError::ConsumerClaimed)?;
        if self.check_endpoint(generation).is_err() {
            let _ = self.consumer_claimed.compare_exchange(
                generation,
                0,
                Ordering::AcqRel,
                Ordering::Acquire,
            );
            return Err(QueueError::StaleEndpoint);
        }
        Ok(Consumer {
            queue: self,
            notifier: Some(notifier),
            generation,
        })
    }

    fn configuration_is_valid() -> bool {
        WORDS != 0
            && WORDS <= (u32::MAX as usize) / size_of::<u32>()
            && N != 0
            && N <= (u32::MAX as usize) / 2
    }

    fn current_generation(&self) -> Result<u32, QueueError> {
        if !Self::configuration_is_valid() {
            return Err(QueueError::InvalidConfiguration);
        }
        if self.magic.load(Ordering::Acquire) != QUEUE_MAGIC {
            return Err(QueueError::NotInitialized);
        }
        if self.layout_words.load(Ordering::Relaxed) != WORDS as u32
            || self.layout_slots.load(Ordering::Relaxed) != N as u32
        {
            return Err(QueueError::LayoutMismatch);
        }
        let generation = self.generation.load(Ordering::Relaxed);
        if generation == 0 {
            Err(QueueError::NotInitialized)
        } else {
            Ok(generation)
        }
    }

    fn check_endpoint(&self, generation: u32) -> Result<(), QueueError> {
        if self.magic.load(Ordering::Acquire) != QUEUE_MAGIC
            || self.generation.load(Ordering::Acquire) != generation
        {
            Err(QueueError::StaleEndpoint)
        } else {
            Ok(())
        }
    }
}

impl<const WORDS: usize, const N: usize> Default for SharedQueue<WORDS, N> {
    fn default() -> Self {
        Self::new()
    }
}

/// Sole producer endpoint for a [`SharedQueue`].
///
/// Dropping the endpoint releases the producer claim. Constructing another
/// producer while this value is alive returns [`QueueError::ProducerClaimed`].
pub struct Producer<'a, const WORDS: usize, const N: usize, D = Polling> {
    queue: &'a SharedQueue<WORDS, N>,
    notifier: Option<D>,
    generation: u32,
}

impl<const WORDS: usize, const N: usize, D> Producer<'_, WORDS, N, D> {
    /// Return ownership of the notification policy and release the producer
    /// claim.
    pub fn into_notifier(mut self) -> D {
        self.notifier
            .take()
            .expect("producer always owns its notifier")
    }
}

impl<const WORDS: usize, const N: usize, D: Notify> Producer<'_, WORDS, N, D> {
    /// Copy and publish one block without waiting.
    pub fn try_push(&mut self, payload: &[u8]) -> Result<(), QueueError> {
        self.queue.check_endpoint(self.generation)?;
        if payload.len() > SharedQueue::<WORDS, N>::BLOCK_CAPACITY {
            return Err(QueueError::PayloadTooLarge);
        }

        let producer = self.queue.producer_position.load(Ordering::Relaxed);
        let consumer = self.queue.consumer_position.load(Ordering::Acquire);
        if producer.wrapping_sub(consumer) >= N as u32 {
            return Err(QueueError::Full);
        }

        let slot = &self.queue.slots[producer as usize % N];
        for (word, chunk) in slot.payload.iter().zip(payload.chunks(size_of::<u32>())) {
            let mut packed = [0_u8; size_of::<u32>()];
            packed[..chunk.len()].copy_from_slice(chunk);
            word.store(u32::from_le_bytes(packed), Ordering::Relaxed);
        }
        slot.length.store(payload.len() as u32, Ordering::Relaxed);
        self.queue
            .producer_position
            .store(producer.wrapping_add(1), Ordering::Release);
        self.notifier
            .as_mut()
            .expect("producer always owns its notifier")
            .notify();
        Ok(())
    }

    /// Wait synchronously for space, invoking `idle` while the queue is full.
    pub fn push_blocking(
        &mut self,
        payload: &[u8],
        mut idle: impl FnMut(),
    ) -> Result<(), QueueError> {
        loop {
            match self.try_push(payload) {
                Err(QueueError::Full) => idle(),
                result => return result,
            }
        }
    }

    /// Wait asynchronously for space using a runtime-supplied wait strategy.
    pub async fn push(
        &mut self,
        payload: &[u8],
        waiter: &mut impl AsyncWait,
    ) -> Result<(), QueueError> {
        loop {
            match self.try_push(payload) {
                Err(QueueError::Full) => waiter.wait().await,
                result => return result,
            }
        }
    }
}

impl<const WORDS: usize, const N: usize, D> Drop for Producer<'_, WORDS, N, D> {
    fn drop(&mut self) {
        let _ = self.queue.producer_claimed.compare_exchange(
            self.generation,
            0,
            Ordering::AcqRel,
            Ordering::Acquire,
        );
    }
}

/// Sole consumer endpoint for a [`SharedQueue`].
///
/// The consumer copies payloads out before releasing their slots. It never
/// returns a reference into shared queue storage.
pub struct Consumer<'a, const WORDS: usize, const N: usize, D = Polling> {
    queue: &'a SharedQueue<WORDS, N>,
    notifier: Option<D>,
    generation: u32,
}

impl<const WORDS: usize, const N: usize, D> Consumer<'_, WORDS, N, D> {
    /// Return ownership of the notification policy and release the consumer
    /// claim.
    pub fn into_notifier(mut self) -> D {
        self.notifier
            .take()
            .expect("consumer always owns its notifier")
    }

    /// Return the next payload length without consuming it.
    pub fn front_len(&self) -> Result<usize, QueueError> {
        self.queue.check_endpoint(self.generation)?;
        let consumer = self.queue.consumer_position.load(Ordering::Relaxed);
        let producer = self.queue.producer_position.load(Ordering::Acquire);
        if producer == consumer {
            return Err(QueueError::Empty);
        }
        let length = self.queue.slots[consumer as usize % N]
            .length
            .load(Ordering::Relaxed) as usize;
        if length > SharedQueue::<WORDS, N>::BLOCK_CAPACITY {
            Err(QueueError::CorruptLength)
        } else {
            Ok(length)
        }
    }
}

impl<const WORDS: usize, const N: usize, D: Notify> Consumer<'_, WORDS, N, D> {
    /// Copy and release the next published block without waiting.
    pub fn try_pop(&mut self, output: &mut [u8]) -> Result<usize, QueueError> {
        let length = self.front_len()?;
        if output.len() < length {
            return Err(QueueError::BufferTooSmall { required: length });
        }

        let consumer = self.queue.consumer_position.load(Ordering::Relaxed);
        let slot = &self.queue.slots[consumer as usize % N];
        for (word, chunk) in slot
            .payload
            .iter()
            .zip(output[..length].chunks_mut(size_of::<u32>()))
        {
            let packed = word.load(Ordering::Relaxed).to_le_bytes();
            chunk.copy_from_slice(&packed[..chunk.len()]);
        }
        self.queue
            .consumer_position
            .store(consumer.wrapping_add(1), Ordering::Release);
        self.notifier
            .as_mut()
            .expect("consumer always owns its notifier")
            .notify();
        Ok(length)
    }

    /// Wait synchronously for a block, invoking `idle` while the queue is empty.
    pub fn pop_blocking(
        &mut self,
        output: &mut [u8],
        mut idle: impl FnMut(),
    ) -> Result<usize, QueueError> {
        loop {
            match self.try_pop(output) {
                Err(QueueError::Empty) => idle(),
                result => return result,
            }
        }
    }

    /// Wait asynchronously for a block using a runtime-supplied wait strategy.
    pub async fn pop(
        &mut self,
        output: &mut [u8],
        waiter: &mut impl AsyncWait,
    ) -> Result<usize, QueueError> {
        loop {
            match self.try_pop(output) {
                Err(QueueError::Empty) => waiter.wait().await,
                result => return result,
            }
        }
    }
}

impl<const WORDS: usize, const N: usize, D> Drop for Consumer<'_, WORDS, N, D> {
    fn drop(&mut self) {
        let _ = self.queue.consumer_claimed.compare_exchange(
            self.generation,
            0,
            Ordering::AcqRel,
            Ordering::Acquire,
        );
    }
}
