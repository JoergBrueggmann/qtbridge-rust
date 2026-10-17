// Copyright (C) 2026 The Qt Company Ltd.
// SPDX-License-Identifier: LicenseRef-Qt-Commercial OR LGPL-3.0-only

//! The file-open event of the application.
//!
//! The operating system asks a running application to open a document, e.g.
//! when the user double-clicks a file registered for the application bundle
//! on macOS. Qt delivers the request as a `QFileOpenEvent` to the
//! `QGuiApplication`. This module installs an event filter on the application
//! that catches the event and hands the path of the file to a Rust handler.
//! [`crate::QApp::on_file_open`] installs it; call
//! [`install_file_open_handler`] manually when driving a raw application.

use std::pin::Pin;

use qtbridge_type_lib::QGuiApplication;

#[cxx::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("qtbridge-type-lib/src/generated/gui/qguiapplication/cpp/qguiapplication.h");
        type QGuiApplication = qtbridge_type_lib::QGuiApplication;

        include!("cpp/fileopen.h");
    }

    #[namespace = "rust::bridge::fileopen"]
    unsafe extern "C++" {
        #[rust_name = install_file_open_handler_impl]
        fn installFileOpenHandler(app: Pin<&mut QGuiApplication>, handler: Box<FileOpenHandler>);

        #[rust_name = post_file_open_event_impl]
        fn postFileOpenEvent(app: Pin<&mut QGuiApplication>, path: &str);
    }

    #[namespace = "rust::bridge::fileopen"]
    extern "Rust" {
        type FileOpenHandler;
        fn call(self: &mut FileOpenHandler, path: String);
    }
}

/// The Rust handler of file-open events, owned by the event filter on the
/// C++ side and called with the path of every file the application is asked
/// to open.
pub struct FileOpenHandler {
    handler: Box<dyn FnMut(String)>,
}

impl FileOpenHandler {
    fn call(&mut self, path: String) {
        (self.handler)(path);
    }
}

/// Installs `handler` for the file-open events of `app`.
///
/// The handler is called on the main thread from within the event loop with
/// the path of the file. Only one handler is kept per application: a second
/// call replaces the first. The event filter is parented to the application
/// and dies with it.
pub fn install_file_open_handler(
    app: Pin<&mut QGuiApplication>,
    handler: impl FnMut(String) + 'static,
) {
    let handler = Box::new(FileOpenHandler {
        handler: Box::new(handler),
    });
    ffi::install_file_open_handler_impl(app, handler);
}

/// Posts a file-open event for `path` to `app`, as the operating system would.
///
/// The event is delivered when the event loop runs, so a handler installed
/// with [`install_file_open_handler`] receives `path` from within the loop.
pub fn post_file_open_event(app: Pin<&mut QGuiApplication>, path: &str) {
    ffi::post_file_open_event_impl(app, path);
}
