// Copyright (C) 2025 The Qt Company Ltd.
// SPDX-License-Identifier: LicenseRef-Qt-Commercial OR LGPL-3.0-only

use std::cell::RefCell;
use std::rc::Rc;

use cxx::UniquePtr;
use cxx_qt_lib::QObjectMutPtr;
use qtbridge_type_lib::{QGuiApplication, QQmlApplicationEngine, QString, QVariant, QVariantMap};
use crate::qmlelement::QmlElement;
use crate::qobjectholder::QObjectHolder;

/// Entry point for a QML application.
///
/// Wraps the Qt application and QML engine. Configure it with the builder
/// methods and call [`run`](QApp::run) to start the event loop.
///
/// # Example
///
/// A minimal “Hello World” application without a Rust backend:
///
/// ```rust
///# use qtbridge_runtime::QApp;
/// QApp::new()
///     .load_qml(br#"
///         import QtQuick
///         import QtQuick.Controls
///         Text {
///             text: "Hello Rust!"
///#            Component.onCompleted: closeTimer.start()
///#            Timer {
///#                id: closeTimer
///#                interval: 1
///#                onTriggered: Qt.quit()
///#            }
///         }"#)
///     .run();
/// ```
pub struct QApp {
    engine: UniquePtr<QQmlApplicationEngine>,
    #[allow(dead_code)]
    app: UniquePtr<QGuiApplication>,
    // create QVariant at load time to avoid storing raw QObject pointers.
    initial_properties: Vec<(QString, Box<dyn FnOnce() -> QVariant>)>,
}

impl Drop for QApp {
    fn drop(&mut self) {
        // Drop the engine first, releasing every JS wrapper: the final
        // collect then frees all objects Rust no longer holds, so their
        // Drop runs here instead of leaking at thread exit.
        self.engine = UniquePtr::null();
        crate::registry::collect_garbage();
    }
}

impl QApp {
    /// Creates the Qt application and QML engine.
    ///
    /// Must be called before any QML or GUI functionality is used.
    pub fn new() -> Self {
        let app = QGuiApplication::new();
        crate::qmlprivate::call_qml_register_callbacks();
        let mut engine = QQmlApplicationEngine::new();
        // Clean up the registry in sync with the QML
        // garbage collection (see `registry::install_gc_sentinel`).
        crate::registry::install_gc_sentinel(engine.pin_mut());
        Self {
            engine: engine,
            app: app,
            initial_properties: Vec::new(),
        }
    }

    /// Enters the Qt main event loop.
    ///
    /// Blocks until the application exits and returns the exit code.
    /// Usually the last call in `main`.
    pub fn run(&mut self) -> i32 {
        self.app.pin_mut().exec()
    }

    /// Queues an initial property to be set on the root QML object.
    ///
    /// Properties are applied when [`load_qml`](QApp::load_qml) or
    /// [`load_qml_from_file`](QApp::load_qml_from_file) is called.
    /// Call multiple times to set several properties.
    ///
    /// # Example
    ///
    /// ```rust
    ///# use qtbridge_runtime::QApp;
    /// let prop = 42;
    ///
    /// QApp::new()
    /// .set_initial_property("answer", &prop)
    /// .load_qml(br#"
    ///     import QtQuick
    ///     import QtQuick.Controls
    ///     ApplicationWindow {
    ///         required property var answer
    ///#        Component.onCompleted: closeTimer.start()
    ///#        Timer {
    ///#            id: closeTimer
    ///#            interval: 1
    ///#            onTriggered: Qt.quit()
    ///#        }
    ///     }"#)
    /// .run();
    /// ```
    pub fn set_initial_property(&mut self, id: &str, value: impl Into<QVariant>) -> &mut Self {
        let variant = value.into();
        self.initial_properties.push((QString::from(id), Box::new( move || {
            variant
        })));
        self
    }

    /// Sets a `#[qobject]` instance as initial property on the root QML
    /// object, attaching a `QObject` to it first if none exists.
    ///
    /// Must be called before [`load_qml`](QApp::load_qml) or
    /// [`load_qml_from_file`](QApp::load_qml_from_file).
    /// Call multiple times to set several objects.
    ///
    /// # Example
    ///
    /// ```rust
    ///# use std::cell::RefCell;
    ///# use std::rc::Rc;
    ///# use qtbridge::{QApp, qobject};
    /// #[derive(Default)]
    /// pub struct Backend {
    /// }
    /// #[qobject]
    /// impl Backend {
    /// }
    ///
    /// let backend = Rc::new(RefCell::new(Backend::default()));
    ///
    /// QApp::new()
    ///     .set_initial_object("backend", backend)
    ///     .load_qml(br#"
    ///         import QtQuick
    ///         Item {
    ///             required property var backend
    ///#            Component.onCompleted: closeTimer.start()
    ///#            Timer {
    ///#                id: closeTimer
    ///#                interval: 1
    ///#                onTriggered: Qt.quit()
    ///#            }
    ///         }"#)
    ///     .run();
    /// ```
    pub fn set_initial_object<T: QObjectHolder>(
        &mut self, id: &str, object: Rc<RefCell<T>>,
    ) -> &mut Self {
        self.initial_properties.push((QString::from(id), Box::new( move || {
            let ptr = T::rc_ref_cell_to_qobject(&object).cast_mut();
            let ptr_wrap = unsafe { QObjectMutPtr::from_raw(ptr.cast()) };
            (&ptr_wrap).into()
        })));
        self
    }

    /// Loads QML source from an in-memory byte slice.
    ///
    /// Applies any properties queued with [`set_initial_property`](QApp::set_initial_property)
    /// before loading.
    pub fn load_qml(&mut self, code: &[u8]) -> &mut Self {
        if !self.initial_properties.is_empty() {
           let mut initial_properties_resolved = QVariantMap::default();
            for (id, resolve) in std::mem::take(&mut self.initial_properties) {
                initial_properties_resolved.insert(id, resolve());
            }
            self.engine.pin_mut().set_initial_properties(&initial_properties_resolved);
        }
        self.engine.pin_mut().load_data(&code.into(), &Default::default());
        self
    }

    /// Loads the entry-point QML file by URL.
    ///
    /// Use this instead of [`load_qml`](QApp::load_qml) when the QML is
    /// embedded in a Qt resource (`qrc:`) or accessible as a file path.
    /// Accepts URLs such as `"qrc:/qt/qml/MyApp/Main.qml"` or
    /// `"file:///path/to/main.qml"`.
    ///
    /// Import paths for any modules the file uses must be registered with
    /// [`add_import_path`](QApp::add_import_path) before this call.
    pub fn load_qml_from_file(&mut self, url: &str) -> &mut Self {
        if !self.initial_properties.is_empty() {
           let mut initial_properties_resolved = QVariantMap::default();
            for (id, resolve) in std::mem::take(&mut self.initial_properties) {
                initial_properties_resolved.insert(id, resolve());
            }
            self.engine.pin_mut().set_initial_properties(&initial_properties_resolved);
        }
        self.engine.pin_mut().load(&url.into());
        self
    }

    /// Adds a directory to the QML engine's module import search path.
    ///
    /// Call before [`load_qml_from_file`](QApp::load_qml_from_file) when the
    /// loaded QML imports modules from a directory the engine would not
    /// otherwise find. Accepts both URLs and file-system paths.
    pub fn add_import_path(&mut self, path: &str) -> &mut Self {
        self.engine.pin_mut().add_import_path(&path.into());
        self
    }

    /// Registers `T` with the QML type system, making it instantiable from QML.
    ///
    /// ```rust
    ///# use qtbridge::{QApp, qobject};
    /// #[derive(Default)]
    /// pub struct Backend {
    /// }
    /// #[qobject]
    /// impl Backend {
    /// }
    ///
    /// QApp::new()
    ///     .register::<Backend>()
    ///     .load_qml(br#"
    ///         import QtQuick
    ///         import QtQuick.Controls
    ///#        import qtbridge_runtime
    ///         ApplicationWindow {
    ///             Backend {}
    ///#            Component.onCompleted: closeTimer.start()
    ///#            Timer {
    ///#                id: closeTimer
    ///#                interval: 1
    ///#                onTriggered: Qt.quit()
    ///#            }
    ///      }"#)
    ///     .run();
    /// ```
    pub fn register<T: QmlElement>(&mut self) -> &mut Self {
        T::register();
        self
    }

    /// Sets the application name reported to the OS.
    pub fn application_name(&mut self, name: &str) -> &mut Self {
        self.app.pin_mut().set_application_name(&name.into());
        self
    }

    /// Installs the handler for the file-open event of the application.
    ///
    /// The operating system sends this event when the user opens a document
    /// registered for the application, e.g. by double-clicking it in the
    /// Finder on macOS; Qt delivers it as `QFileOpenEvent`. The handler is
    /// called with the path of the file, on the main thread from within the
    /// event loop: the event of a launch by double-click arrives after
    /// [`run`](QApp::run) has started. Call before [`run`](QApp::run).
    /// Only one handler is kept; a second call replaces the first.
    ///
    /// # Example
    ///
    /// ```rust
    ///# use std::cell::RefCell;
    ///# use std::rc::Rc;
    ///# use qtbridge_runtime::QApp;
    /// let opened = Rc::new(RefCell::new(Vec::new()));
    /// let recorder = opened.clone();
    ///
    /// QApp::new()
    ///     .on_file_open(move |path| recorder.borrow_mut().push(path))
    ///#    .post_file_open("/tmp/example.txt")
    ///     .load_qml(br#"
    ///         import QtQuick
    ///         Item {
    ///#            Component.onCompleted: closeTimer.start()
    ///#            Timer {
    ///#                id: closeTimer
    ///#                interval: 1
    ///#                onTriggered: Qt.quit()
    ///#            }
    ///         }"#)
    ///     .run();
    ///# assert_eq!(*opened.borrow(), vec!["/tmp/example.txt".to_string()]);
    /// ```
    pub fn on_file_open(&mut self, handler: impl FnMut(String) + 'static) -> &mut Self {
        crate::fileopen::install_file_open_handler(self.app.pin_mut(), handler);
        self
    }

    /// Posts a file-open event for `path` to the application, as the
    /// operating system would.
    ///
    /// Lets a test or the application itself raise the event. It is
    /// delivered when the event loop runs, so a handler installed with
    /// [`on_file_open`](QApp::on_file_open) receives `path` after
    /// [`run`](QApp::run) has started.
    ///
    /// # Example
    ///
    /// ```rust
    ///# use std::cell::RefCell;
    ///# use std::rc::Rc;
    ///# use qtbridge_runtime::QApp;
    /// let opened = Rc::new(RefCell::new(Vec::new()));
    /// let recorder = opened.clone();
    ///
    /// QApp::new()
    ///     .on_file_open(move |path| recorder.borrow_mut().push(path))
    ///     .post_file_open("/tmp/example.txt")
    ///     .load_qml(br#"
    ///         import QtQuick
    ///         Item {
    ///#            Component.onCompleted: closeTimer.start()
    ///#            Timer {
    ///#                id: closeTimer
    ///#                interval: 1
    ///#                onTriggered: Qt.quit()
    ///#            }
    ///         }"#)
    ///     .run();
    ///
    /// assert_eq!(*opened.borrow(), vec!["/tmp/example.txt".to_string()]);
    /// ```
    pub fn post_file_open(&mut self, path: &str) -> &mut Self {
        crate::fileopen::post_file_open_event(self.app.pin_mut(), path);
        self
    }
}
