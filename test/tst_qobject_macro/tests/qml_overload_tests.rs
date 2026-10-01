// Copyright (C) 2026 The Qt Company Ltd.
// SPDX-License-Identifier: LicenseRef-Qt-Commercial OR LGPL-3.0-only
#![cfg(test)]

use qtbridge_type_lib::{QGuiApplication, QObjectMutPtr, QQmlApplicationEngine, QVariant, QVariantMap, QString};
use qtbridge::{qobject, QmlObject, QmlElement};
use qtbridge::qtbridge_runtime::QObjectHolder;

#[qobject]
pub mod widget {
    #[derive(Default)]
    pub struct Widget {
        pub rust_ran: bool,
    }
    impl Widget {
        // overridden in qml
        #[qslot(qml_name = "ping")]
        fn ping(&mut self) {
            self.rust_ran = true;
        }
    }
}
pub use widget::Widget;

#[qobject]
pub mod registry {
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Default)]
    pub struct Registry {
        pub widget: Option<Rc<RefCell<crate::Widget>>>,
        pub qml_ran: bool,
    }
    impl Registry {
        #[qslot(qml_name = "registerWidget")]
        fn register_widget(&mut self, w: Rc<RefCell<crate::Widget>>) {
            self.widget = Some(w);
        }
        #[qslot(qml_name = "qmlPingRan")]
        fn qml_ping_ran(&mut self) {
            self.qml_ran = true;
        }
    }
}
pub use registry::Registry;

const QML: &str = r#"
    import QtQuick
    import tst_qobject_macro
    Item {
        required property var registry
        Widget {
            id: w
            function ping() { registry.qmlPingRan(); }
        }
        Component.onCompleted: registry.registerWidget(w);
    }
"#;

#[cfg(not(miri))]
fn main() {
    let _app = QGuiApplication::new();
    Widget::register();

    let registry = Registry::default_with_attached_qobject();
    let registry_var = QVariant::from(&unsafe {
        QObjectMutPtr::from_raw(Registry::rc_ref_cell_to_qobject(&registry).cast_mut())
    });

    let mut props = QVariantMap::default();
    props.insert(QString::from("registry"), registry_var);

    let mut engine = QQmlApplicationEngine::new();
    engine.pin_mut().set_initial_properties(&props);
    engine.pin_mut().load_data(&QML.into(), &Default::default());

    // A QML-extended instance carries a derived QML meta-object; passing it
    // back into Rust must succeed
    let captured = registry.borrow().widget.clone();
    assert!(captured.is_some(), "QML-extended Widget should round-trip back into Rust");
}

#[cfg(miri)]
fn main() {}
