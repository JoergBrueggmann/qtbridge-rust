// Copyright (C) 2026 The Qt Company Ltd.
// SPDX-License-Identifier: LicenseRef-Qt-Commercial OR LGPL-3.0-only
#![cfg(test)]


use qtbridge_type_lib::QGuiApplication;
use qtbridge::{qobject, QmlObject, QThreadHandle};
use qtbridge::qtbridge_runtime::QObjectHolder;
use qtbridge::qtbridge_type_lib::{QSignalSpy};
#[qobject]
pub mod test_object {
    use std::cell::Cell;

    #[derive(Default)]
    pub struct TestObject {
        pub int_value: i32,
        pub immutable_slot_called: Cell<bool>,
    }

    impl TestObject {
        #[qsignal]
        pub fn signal_no_args(&mut self);

        #[qslot]
        pub fn immutable_slot(&self) {
            self.immutable_slot_called.set(true);
        }

        #[qslot]
        pub fn set_int(&mut self, value: i32) {
            self.int_value = value;
        }

        #[qslot]
        pub fn add_ints(&mut self, a: i32, b: i32) {
            self.int_value = a + b;
        }
    }
}

pub use test_object::TestObject;

fn test_invoke_mut(app: &QGuiApplication) {
    let qobject_holder = TestObject::default_with_attached_qobject();
    let handle = qobject_holder.borrow().get_thread_handle();
    handle.invoke_mut(|obj| obj.set_int(42));
    app.process_events();
    app.process_events();
    assert_eq!(qobject_holder.borrow().int_value, 42);
}

fn test_invoke(app: &QGuiApplication) {
    let qobject_holder = TestObject::default_with_attached_qobject();
    let handle = qobject_holder.borrow().get_thread_handle();
    handle.invoke(|obj| obj.immutable_slot());
    app.process_events();
    app.process_events();
    assert!(qobject_holder.borrow().immutable_slot_called.get());
}

fn test_signal_emitted(app: &QGuiApplication) {
    let qobject_holder = TestObject::default_with_attached_qobject();
    let spy = QSignalSpy::new(unsafe { &*qobject_holder.borrow().get_qobject_ptr() }, "signal_no_args");
    qobject_holder.borrow().get_thread_handle().invoke_mut(|obj| obj.signal_no_args());
    app.process_events();
    app.process_events();
    assert_eq!(spy.count(), 1);
}

fn test_invoke_from_thread(app: &QGuiApplication) {
    let qobject_holder = TestObject::default_with_attached_qobject();
    let handle = qobject_holder.borrow().get_thread_handle();
    // Captures that have no QVariant representation travel with the closure.
    let summands: Vec<i32> = vec![15, 27];
    std::thread::spawn(move || {
        handle.invoke_mut(move |obj| obj.add_ints(summands[0], summands[1]));
    }).join().unwrap();
    app.process_events();
    app.process_events();
    assert_eq!(qobject_holder.borrow().int_value, 42);
}

fn test_destroyed_target(app: &QGuiApplication) {
    let qobject_holder = TestObject::default_with_attached_qobject();
    let handle = qobject_holder.borrow().get_thread_handle();
    qobject_holder.borrow().detach_qobject();
    // The target's QObject is gone: the closure must be discarded, not run.
    handle.invoke_mut(|obj| obj.set_int(13));
    app.process_events();
    app.process_events();
    assert_eq!(qobject_holder.borrow().int_value, 0);
}

fn test_destroyed_after_post(app: &QGuiApplication) {
    let qobject_holder = TestObject::default_with_attached_qobject();
    let handle = qobject_holder.borrow().get_thread_handle();
    handle.invoke_mut(|obj| obj.set_int(13));
    // Deleting the target between posting and delivery discards the call.
    qobject_holder.borrow().detach_qobject();
    app.process_events();
    app.process_events();
    assert_eq!(qobject_holder.borrow().int_value, 0);
}

fn test_untargeted_from_thread(app: &QGuiApplication) {
    let qt_thread = std::thread::current().id();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let handle = QThreadHandle::default();
        handle.invoke(move || tx.send(std::thread::current().id()).unwrap());
    }).join().unwrap();
    assert!(rx.try_recv().is_err(), "closure must not run before the event loop delivers it");
    app.process_events();
    app.process_events();
    assert_eq!(rx.try_recv().unwrap(), qt_thread);
}

fn test_target_on_worker_thread(app: &QGuiApplication) {
    let (posted_tx, posted_rx) = std::sync::mpsc::channel();
    let (done_tx, done_rx) = std::sync::mpsc::channel::<()>();
    let (ran_tx, ran_rx) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        let qobject_holder = TestObject::default_with_attached_qobject();
        let handle = qobject_holder.borrow().get_thread_handle();
        handle.invoke_mut(move |obj| { obj.set_int(13); ran_tx.send(()).unwrap(); });
        posted_tx.send(()).unwrap();
        // Keep the target alive while the Qt thread processes events.
        done_rx.recv().unwrap();
        qobject_holder.borrow().int_value
    });
    posted_rx.recv().unwrap();
    app.process_events();
    app.process_events();
    done_tx.send(()).unwrap();
    assert_eq!(worker.join().unwrap(), 0);
    assert!(ran_rx.try_recv().is_err(), "closure must not run on a target owned by another thread");
}

#[cfg(not(miri))]
fn main() {
    let app = QGuiApplication::new();
    test_invoke_mut(&app);
    test_invoke(&app);
    test_signal_emitted(&app);
    test_invoke_from_thread(&app);
    test_destroyed_target(&app);
    test_destroyed_after_post(&app);
    test_untargeted_from_thread(&app);
    test_target_on_worker_thread(&app);
}

#[cfg(miri)]
fn main() {}
