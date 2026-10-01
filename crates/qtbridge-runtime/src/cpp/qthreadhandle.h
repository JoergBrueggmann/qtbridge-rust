// Copyright (C) 2026 The Qt Company Ltd.
// SPDX-License-Identifier: LicenseRef-Qt-Commercial OR LGPL-3.0-only

#ifndef QT_THREAD_HANDLE_H
#define QT_THREAD_HANDLE_H

#include "rust/cxx.h"

#include <QObject>
#include <QPointer>

#include <memory>

// Opaque Rust types (qt_thread_handle.rs)
struct QObjectClosureBox;
struct VoidClosureBox;

// Guard for one target object. Must be constructed on the target's thread
// and the target must live on the Qt thread.
// afterwards it may be moved to, used from and destroyed on any thread.
// make_qt_thread_handle() nulls targets not living on the application thread.
class QThreadHandleCpp
{
public:
    explicit QThreadHandleCpp(QObject *target) : target(target) { }
    QPointer<QObject> target;
};

std::unique_ptr<QThreadHandleCpp> make_qt_thread_handle(QObject *target);

// Thread-safe: posts `closure` on the event loop where the proper user
// object is reconstructed from the QPointer in the `handle`. The QPointer
// is only touched on the Qt thread, which should also be the only thread
// where a QObject is deleted (at least in the context of qtbridge).
// `closure` must contain a FnOnce(&T: QObjectHandler) + Send + 'static.
void post_closure(const QThreadHandleCpp &handle, rust::Box<QObjectClosureBox> closure);

// Thread-safe: posts `closure` on the event loop without a target.
// `closure` must contain a FnOnce() + Send + 'static.
void post_void_closure(rust::Box<VoidClosureBox> closure);

#endif // QT_THREAD_HANDLE_H
