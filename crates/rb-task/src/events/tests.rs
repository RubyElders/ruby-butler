use super::*;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc,
};
use std::thread;

struct CountingSink(AtomicUsize);

impl TaskEventSink for CountingSink {
    fn event(&self, _event: TaskEvent) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

#[test]
fn event_bus_fans_out_without_owning_execution() {
    let bus = TaskEvents::default();
    let sink = Arc::new(CountingSink(AtomicUsize::new(0)));
    let other = Arc::new(CountingSink(AtomicUsize::new(0)));
    bus.subscribe(sink.clone());
    bus.subscribe(other.clone());
    bus.emit(TaskEvent::Finished {
        task: TaskId(0),
        worker: 0,
        elapsed: Duration::ZERO,
    });
    assert_eq!(sink.0.load(Ordering::Relaxed), 1);
    assert_eq!(other.0.load(Ordering::Relaxed), 1);
}

#[test]
fn event_callbacks_can_subscribe_without_locking_the_bus() {
    struct Subscriber(TaskEvents);
    struct Ignore;
    impl TaskEventSink for Ignore {
        fn event(&self, _: TaskEvent) {}
    }
    impl TaskEventSink for Subscriber {
        fn event(&self, _: TaskEvent) {
            self.0.subscribe(Arc::new(Ignore));
        }
    }
    let bus = TaskEvents::default();
    bus.subscribe(Arc::new(Subscriber(bus.clone())));
    let emitting = bus.clone();
    let (send, receive) = mpsc::channel();
    thread::spawn(move || {
        emitting.emit(TaskEvent::Finished {
            task: TaskId(0),
            worker: 0,
            elapsed: Duration::ZERO,
        });
        send.send(()).unwrap();
    });
    receive.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(bus.sinks.lock().unwrap().len(), 2);
    bus.sinks.lock().unwrap().clear();
}
