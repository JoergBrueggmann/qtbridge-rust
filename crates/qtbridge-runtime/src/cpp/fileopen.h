// Copyright (C) 2026 The Qt Company Ltd.
// SPDX-License-Identifier: LicenseRef-Qt-Commercial OR LGPL-3.0-only

#ifndef _FILEOPEN_RUST_BRIDGE_H_
#define _FILEOPEN_RUST_BRIDGE_H_

#include "rust/cxx.h"

class QGuiApplication;

namespace rust::bridge::fileopen {

struct FileOpenHandler;

void installFileOpenHandler(QGuiApplication& app, ::rust::Box<FileOpenHandler> handler);
void postFileOpenEvent(QGuiApplication& app, ::rust::Str path);

} // namespace rust::bridge::fileopen

#endif // _FILEOPEN_RUST_BRIDGE_H_
