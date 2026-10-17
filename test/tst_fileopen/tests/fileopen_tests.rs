// Copyright (C) 2026 The Qt Company Ltd.
// SPDX-License-Identifier: LicenseRef-Qt-Commercial OR LGPL-3.0-only

//! The file-open event of the application: a handler installed with
//! `QApp::on_file_open` receives the path of every `QFileOpenEvent` the
//! application gets, from within the event loop. The events are raised with
//! `QApp::post_file_open` in place of the operating system.

use std::cell::RefCell;
use std::rc::Rc;

use qtbridge::QApp;

const QML: &[u8] = br#"
    import QtQuick
    Item {
        Timer {
            interval: 10; running: true
            onTriggered: Qt.quit()
        }
    }
"#;

/// A file-open event posted before `run` reaches the handler once the loop
/// runs, with the path of the file.
fn posted_event_reaches_the_handler() {
    let opened = Rc::new(RefCell::new(Vec::new()));
    let recorder = opened.clone();

    assert_eq!(
        QApp::new()
            .on_file_open(move |path| recorder.borrow_mut().push(path))
            .post_file_open("/tmp/first.txt")
            .post_file_open("/tmp/second.txt")
            .load_qml(QML)
            .run(),
        0
    );

    assert_eq!(
        *opened.borrow(),
        vec!["/tmp/first.txt".to_string(), "/tmp/second.txt".to_string()]
    );
}

/// Only one handler is kept: the second replaces the first.
fn second_handler_replaces_the_first() {
    let first = Rc::new(RefCell::new(Vec::new()));
    let second = Rc::new(RefCell::new(Vec::new()));
    let first_recorder = first.clone();
    let second_recorder = second.clone();

    QApp::new()
        .on_file_open(move |path| first_recorder.borrow_mut().push(path))
        .on_file_open(move |path| second_recorder.borrow_mut().push(path))
        .post_file_open("/tmp/replaced.txt")
        .load_qml(QML)
        .run();

    assert!(first.borrow().is_empty());
    assert_eq!(*second.borrow(), vec!["/tmp/replaced.txt".to_string()]);
}

/// Without a handler the posted event is delivered and ignored.
fn event_without_handler_is_harmless() {
    assert_eq!(
        QApp::new()
            .post_file_open("/tmp/unhandled.txt")
            .load_qml(QML)
            .run(),
        0
    );
}

fn main() {
    if cfg!(miri) {
        return;
    }
    posted_event_reaches_the_handler();
    second_handler_replaces_the_first();
    event_without_handler_is_harmless();
}
