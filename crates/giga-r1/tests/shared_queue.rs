use core::{
    mem::{align_of, size_of},
    sync::atomic::{AtomicUsize, Ordering},
};
use std::{sync::Arc, thread};

use giga_r1::ipc::{Notify, QueueError, SharedQueue};

type TestQueue = SharedQueue<4, 3>;

#[derive(Clone)]
struct CountingNotifier(Arc<AtomicUsize>);

impl Notify for CountingNotifier {
    fn notify(&mut self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

#[test]
fn reports_expected_bulk_memory_cost() {
    assert_eq!(SharedQueue::<384, 4>::BLOCK_CAPACITY, 1_536);
    assert_eq!(SharedQueue::<384, 4>::MEMORY_SIZE, 6_208);
    assert_eq!(size_of::<SharedQueue<384, 4>>(), 6_208);
    assert_eq!(align_of::<SharedQueue<384, 4>>(), 32);
}

#[test]
fn starts_uninitialized_then_empty() {
    let queue = TestQueue::new();
    assert_eq!(queue.producer().err(), Some(QueueError::NotInitialized));

    queue.initialize_primary().unwrap();
    let mut consumer = queue.consumer().unwrap();
    let mut output = [0_u8; 16];
    assert_eq!(consumer.try_pop(&mut output), Err(QueueError::Empty));
}

#[test]
fn reports_full_and_preserves_fifo_order() {
    let queue = TestQueue::new();
    queue.initialize_primary().unwrap();
    let mut producer = queue.producer().unwrap();
    let mut consumer = queue.consumer().unwrap();

    producer.try_push(&[1]).unwrap();
    producer.try_push(&[2, 2]).unwrap();
    producer.try_push(&[3, 3, 3]).unwrap();
    assert_eq!(producer.try_push(&[4]), Err(QueueError::Full));

    let mut output = [0_u8; 16];
    assert_eq!(consumer.try_pop(&mut output).unwrap(), 1);
    assert_eq!(&output[..1], &[1]);
    assert_eq!(consumer.try_pop(&mut output).unwrap(), 2);
    assert_eq!(&output[..2], &[2, 2]);
    assert_eq!(consumer.try_pop(&mut output).unwrap(), 3);
    assert_eq!(&output[..3], &[3, 3, 3]);
    assert_eq!(consumer.try_pop(&mut output), Err(QueueError::Empty));
}

#[test]
fn wraps_slots_and_reuses_released_capacity() {
    let queue = TestQueue::new();
    queue.initialize_primary().unwrap();
    let mut producer = queue.producer().unwrap();
    let mut consumer = queue.consumer().unwrap();
    let mut output = [0_u8; 16];

    for sequence in 0_u32..1_000 {
        let payload = sequence.to_le_bytes();
        producer.try_push(&payload).unwrap();
        assert_eq!(consumer.try_pop(&mut output).unwrap(), payload.len());
        assert_eq!(&output[..payload.len()], &payload);
    }
}

#[test]
fn short_output_does_not_release_the_slot() {
    let queue = TestQueue::new();
    queue.initialize_primary().unwrap();
    let mut producer = queue.producer().unwrap();
    let mut consumer = queue.consumer().unwrap();

    producer.try_push(&[1, 2, 3, 4, 5]).unwrap();
    assert_eq!(consumer.front_len(), Ok(5));
    assert_eq!(
        consumer.try_pop(&mut [0_u8; 4]),
        Err(QueueError::BufferTooSmall { required: 5 })
    );

    let mut output = [0_u8; 16];
    assert_eq!(consumer.try_pop(&mut output), Ok(5));
    assert_eq!(&output[..5], &[1, 2, 3, 4, 5]);
}

#[test]
fn enforces_one_endpoint_per_role_and_releases_claims_on_drop() {
    let queue = TestQueue::new();
    queue.initialize_primary().unwrap();

    let producer = queue.producer().unwrap();
    assert_eq!(queue.producer().err(), Some(QueueError::ProducerClaimed));
    drop(producer);
    assert!(queue.producer().is_ok());

    let consumer = queue.consumer().unwrap();
    assert_eq!(queue.consumer().err(), Some(QueueError::ConsumerClaimed));
    drop(consumer);
    assert!(queue.consumer().is_ok());
}

#[test]
fn reinitialization_invalidates_old_endpoints_without_releasing_new_claims() {
    let queue = TestQueue::new();
    queue.initialize_primary().unwrap();
    let mut old_producer = queue.producer().unwrap();
    let mut old_consumer = queue.consumer().unwrap();

    queue.initialize_primary().unwrap();
    assert_eq!(old_producer.try_push(&[1]), Err(QueueError::StaleEndpoint));
    assert_eq!(
        old_consumer.try_pop(&mut [0_u8; 16]),
        Err(QueueError::StaleEndpoint)
    );

    let new_producer = queue.producer().unwrap();
    let new_consumer = queue.consumer().unwrap();
    drop(old_producer);
    drop(old_consumer);
    assert_eq!(queue.producer().err(), Some(QueueError::ProducerClaimed));
    assert_eq!(queue.consumer().err(), Some(QueueError::ConsumerClaimed));
    drop(new_producer);
    drop(new_consumer);
    assert!(queue.producer().is_ok());
    assert!(queue.consumer().is_ok());
}

#[test]
fn notifies_after_publish_and_release() {
    let queue = TestQueue::new();
    queue.initialize_primary().unwrap();
    let notifications = Arc::new(AtomicUsize::new(0));
    let mut producer = queue
        .producer_with_notifier(CountingNotifier(notifications.clone()))
        .unwrap();
    let mut consumer = queue
        .consumer_with_notifier(CountingNotifier(notifications.clone()))
        .unwrap();

    producer.try_push(&[0xaa]).unwrap();
    consumer.try_pop(&mut [0_u8; 1]).unwrap();
    assert_eq!(notifications.load(Ordering::Relaxed), 2);
}

#[test]
fn rejects_invalid_configuration_and_oversized_payloads() {
    let invalid = SharedQueue::<0, 1>::new();
    assert_eq!(
        invalid.initialize_primary(),
        Err(QueueError::InvalidConfiguration)
    );

    let queue = TestQueue::new();
    queue.initialize_primary().unwrap();
    let mut producer = queue.producer().unwrap();
    assert_eq!(
        producer.try_push(&[0_u8; 17]),
        Err(QueueError::PayloadTooLarge)
    );
}

#[test]
fn sustains_concurrent_producer_consumer_ordering() {
    const ITEMS: u32 = 20_000;
    let queue = SharedQueue::<1, 8>::new();
    queue.initialize_primary().unwrap();
    let mut producer = queue.producer().unwrap();
    let mut consumer = queue.consumer().unwrap();

    thread::scope(|scope| {
        scope.spawn(move || {
            for sequence in 0..ITEMS {
                let payload = sequence.to_le_bytes();
                while producer.try_push(&payload) == Err(QueueError::Full) {
                    core::hint::spin_loop();
                }
            }
        });

        scope.spawn(move || {
            let mut output = [0_u8; 4];
            for expected in 0..ITEMS {
                loop {
                    match consumer.try_pop(&mut output) {
                        Ok(4) => break,
                        Err(QueueError::Empty) => core::hint::spin_loop(),
                        result => panic!("unexpected queue result: {result:?}"),
                    }
                }
                assert_eq!(u32::from_le_bytes(output), expected);
            }
        });
    });
}
