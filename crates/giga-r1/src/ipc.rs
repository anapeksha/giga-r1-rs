//! Typed, allocation-free Cortex-M7/Cortex-M4 request/response IPC.
//!
//! The wire format is [`postcard`]. The mailbox itself uses atomic words so
//! neither core creates a Rust reference to memory concurrently modified by
//! the other core. Place exactly one [`IpcMailbox`] at the same non-cacheable
//! D3 SRAM address in both images.

use core::{
    future::Future,
    marker::PhantomData,
    sync::atomic::{AtomicU32, Ordering},
};

use serde::{Serialize, de::DeserializeOwned};

pub const IPC_MAGIC: u32 = 0x4950_4332;
pub const IPC_CAPACITY: usize = 256;
const IPC_WORDS: usize = IPC_CAPACITY / size_of::<u32>();

/// A notification hook invoked after publishing a request or response.
///
/// Implementations may ring an STM32 HSEM interrupt, issue an event, or do
/// nothing when the application intentionally polls.
pub trait Notify {
    fn notify(&mut self);
}

/// Runtime-supplied asynchronous wait strategy.
///
/// An executor integration can implement this with an HSEM interrupt signal,
/// an async event listener, or a timer. Keeping the future associated with the
/// application avoids coupling `giga-r1` to a particular async runtime.
pub trait AsyncWait {
    type Wait<'a>: Future<Output = ()>
    where
        Self: 'a;

    fn wait(&mut self) -> Self::Wait<'_>;
}

/// Polling notification policy.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Polling;

impl Notify for Polling {
    fn notify(&mut self) {}
}

/// Local Cortex event notification.
///
/// This pairs with a waiter on the same core. Waking the other STM32H747 core
/// requires a configured peripheral interrupt such as HSEM; a plain `SEV`
/// must not be relied upon as an inter-core doorbell.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct EventDoorbell;

impl Notify for EventDoorbell {
    fn notify(&mut self) {
        cortex_m::asm::dsb();
        cortex_m::asm::sev();
    }
}

/// IPC framing or protocol failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum IpcError {
    NotInitialized,
    RequestPending,
    ResponsePending,
    Serialize,
    Deserialize,
    PayloadTooLarge,
}

/// Sequence number returned after publishing a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct RequestId(pub u32);

/// Shared, single-client/single-server IPC storage.
#[repr(C, align(32))]
pub struct IpcMailbox {
    magic: AtomicU32,
    worker_state: AtomicU32,
    request_sequence: AtomicU32,
    request_length: AtomicU32,
    response_sequence: AtomicU32,
    response_length: AtomicU32,
    request: [AtomicU32; IPC_WORDS],
    response: [AtomicU32; IPC_WORDS],
}

impl IpcMailbox {
    pub const fn new() -> Self {
        Self {
            magic: AtomicU32::new(0),
            worker_state: AtomicU32::new(0),
            request_sequence: AtomicU32::new(0),
            request_length: AtomicU32::new(0),
            response_sequence: AtomicU32::new(0),
            response_length: AtomicU32::new(0),
            request: [const { AtomicU32::new(0) }; IPC_WORDS],
            response: [const { AtomicU32::new(0) }; IPC_WORDS],
        }
    }

    /// Reset the protocol before the M7 starts the M4 image.
    pub fn initialize_primary(&self) {
        self.magic.store(0, Ordering::SeqCst);
        self.worker_state.store(0, Ordering::Relaxed);
        self.request_sequence.store(0, Ordering::Relaxed);
        self.request_length.store(0, Ordering::Relaxed);
        self.response_sequence.store(0, Ordering::Relaxed);
        self.response_length.store(0, Ordering::Relaxed);
        clear_words(&self.request);
        clear_words(&self.response);
        self.magic.store(IPC_MAGIC, Ordering::Release);
    }

    pub fn is_initialized(&self) -> bool {
        self.magic.load(Ordering::Acquire) == IPC_MAGIC
    }

    /// Record a worker-defined diagnostic state.
    ///
    /// This is useful for startup supervision and remains independent of the
    /// request/response protocol. Zero conventionally means "not started".
    pub fn set_worker_state(&self, state: u32) {
        self.worker_state.store(state, Ordering::Release);
    }

    /// Return the most recently published worker diagnostic state.
    pub fn worker_state(&self) -> u32 {
        self.worker_state.load(Ordering::Acquire)
    }
}

impl Default for IpcMailbox {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for IpcMailbox {
    fn format(&self, formatter: defmt::Formatter) {
        defmt::write!(
            formatter,
            "IpcMailbox(initialized={}, request_sequence={}, response_sequence={})",
            self.is_initialized(),
            self.request_sequence.load(Ordering::Acquire),
            self.response_sequence.load(Ordering::Acquire),
        );
    }
}

/// A typed postcard channel over an [`IpcMailbox`].
///
/// Construct one endpoint on each core with the request and response types in
/// the same order. The M7 normally calls [`Self::send`] and
/// [`Self::try_response`]; the M4 calls [`Self::try_request`] and
/// [`Self::respond`].
pub struct Channel<'a, T, R, N = Polling> {
    mailbox: &'a IpcMailbox,
    notifier: N,
    next_sequence: u32,
    last_request: u32,
    marker: PhantomData<fn(T) -> R>,
}

impl<'a, T, R> Channel<'a, T, R, Polling> {
    pub const fn new(mailbox: &'a IpcMailbox) -> Self {
        Self::with_notifier(mailbox, Polling)
    }
}

impl<'a, T, R, N> Channel<'a, T, R, N> {
    pub const fn with_notifier(mailbox: &'a IpcMailbox, notifier: N) -> Self {
        Self {
            mailbox,
            notifier,
            next_sequence: 1,
            last_request: 0,
            marker: PhantomData,
        }
    }

    pub fn into_notifier(self) -> N {
        self.notifier
    }
}

impl<T, R, N: Notify> Channel<'_, T, R, N> {
    /// Serialize and publish one request without allocating.
    pub fn send(&mut self, request: &T) -> Result<RequestId, IpcError>
    where
        T: Serialize,
    {
        if !self.mailbox.is_initialized() {
            return Err(IpcError::NotInitialized);
        }

        let published = self.mailbox.request_sequence.load(Ordering::Acquire);
        let completed = self.mailbox.response_sequence.load(Ordering::Acquire);
        if published != completed {
            return Err(IpcError::RequestPending);
        }

        let mut bytes = [0_u8; IPC_CAPACITY];
        let encoded = postcard::to_slice(request, &mut bytes).map_err(|error| {
            if matches!(error, postcard::Error::SerializeBufferFull) {
                IpcError::PayloadTooLarge
            } else {
                IpcError::Serialize
            }
        })?;
        write_words(&self.mailbox.request, encoded);
        self.mailbox
            .request_length
            .store(encoded.len() as u32, Ordering::Relaxed);

        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.wrapping_add(1).max(1);
        self.mailbox
            .request_sequence
            .store(sequence, Ordering::Release);
        self.notifier.notify();
        Ok(RequestId(sequence))
    }

    /// Deserialize a response when the worker has completed `request`.
    pub fn try_response(&mut self, request: RequestId) -> Result<Option<R>, IpcError>
    where
        R: DeserializeOwned,
    {
        if !self.mailbox.is_initialized() {
            return Err(IpcError::NotInitialized);
        }
        if self.mailbox.response_sequence.load(Ordering::Acquire) != request.0 {
            return Ok(None);
        }

        let length = checked_length(self.mailbox.response_length.load(Ordering::Relaxed))?;
        let mut bytes = [0_u8; IPC_CAPACITY];
        read_words(&self.mailbox.response, &mut bytes[..length]);
        postcard::from_bytes(&bytes[..length])
            .map(Some)
            .map_err(|_| IpcError::Deserialize)
    }

    /// Wait synchronously for a typed response, yielding with `idle` between
    /// checks. `cortex_m::asm::wfe` is a suitable low-power idle hook.
    pub fn call_blocking(&mut self, request: &T, mut idle: impl FnMut()) -> Result<R, IpcError>
    where
        T: Serialize,
        R: DeserializeOwned,
    {
        let request = self.send(request)?;
        loop {
            if let Some(response) = self.try_response(request)? {
                return Ok(response);
            }
            idle();
        }
    }

    /// Wait asynchronously for a typed response using a runtime-supplied
    /// interrupt or event listener.
    pub async fn call(&mut self, request: &T, waiter: &mut impl AsyncWait) -> Result<R, IpcError>
    where
        T: Serialize,
        R: DeserializeOwned,
    {
        let request = self.send(request)?;
        loop {
            if let Some(response) = self.try_response(request)? {
                return Ok(response);
            }
            waiter.wait().await;
        }
    }

    /// Receive the next request on the worker core.
    pub fn try_request(&mut self) -> Result<Option<(RequestId, T)>, IpcError>
    where
        T: DeserializeOwned,
    {
        if !self.mailbox.is_initialized() {
            return Err(IpcError::NotInitialized);
        }
        let sequence = self.mailbox.request_sequence.load(Ordering::Acquire);
        if sequence == 0 || sequence == self.last_request {
            return Ok(None);
        }

        let length = checked_length(self.mailbox.request_length.load(Ordering::Relaxed))?;
        let mut bytes = [0_u8; IPC_CAPACITY];
        read_words(&self.mailbox.request, &mut bytes[..length]);
        let request = postcard::from_bytes(&bytes[..length]).map_err(|_| IpcError::Deserialize)?;
        self.last_request = sequence;
        Ok(Some((RequestId(sequence), request)))
    }

    /// Serialize and publish the worker response for `request`.
    pub fn respond(&mut self, request: RequestId, response: &R) -> Result<(), IpcError>
    where
        R: Serialize,
    {
        if self.mailbox.response_sequence.load(Ordering::Acquire) == request.0 {
            return Err(IpcError::ResponsePending);
        }

        let mut bytes = [0_u8; IPC_CAPACITY];
        let encoded = postcard::to_slice(response, &mut bytes).map_err(|error| {
            if matches!(error, postcard::Error::SerializeBufferFull) {
                IpcError::PayloadTooLarge
            } else {
                IpcError::Serialize
            }
        })?;
        write_words(&self.mailbox.response, encoded);
        self.mailbox
            .response_length
            .store(encoded.len() as u32, Ordering::Relaxed);
        self.mailbox
            .response_sequence
            .store(request.0, Ordering::Release);
        self.notifier.notify();
        Ok(())
    }
}

#[cfg(feature = "defmt")]
impl<T, R, N> defmt::Format for Channel<'_, T, R, N> {
    fn format(&self, formatter: defmt::Formatter) {
        defmt::write!(
            formatter,
            "Channel(next_sequence={}, last_request={})",
            self.next_sequence,
            self.last_request,
        );
    }
}

fn checked_length(length: u32) -> Result<usize, IpcError> {
    let length = length as usize;
    if length > IPC_CAPACITY {
        Err(IpcError::PayloadTooLarge)
    } else {
        Ok(length)
    }
}

fn clear_words(words: &[AtomicU32; IPC_WORDS]) {
    for word in words {
        word.store(0, Ordering::Relaxed);
    }
}

fn write_words(words: &[AtomicU32; IPC_WORDS], bytes: &[u8]) {
    for (index, chunk) in bytes.chunks(4).enumerate() {
        let mut packed = [0_u8; 4];
        packed[..chunk.len()].copy_from_slice(chunk);
        words[index].store(u32::from_le_bytes(packed), Ordering::Relaxed);
    }
}

fn read_words(words: &[AtomicU32; IPC_WORDS], bytes: &mut [u8]) {
    for (index, chunk) in bytes.chunks_mut(4).enumerate() {
        let packed = words[index].load(Ordering::Relaxed).to_le_bytes();
        chunk.copy_from_slice(&packed[..chunk.len()]);
    }
}
