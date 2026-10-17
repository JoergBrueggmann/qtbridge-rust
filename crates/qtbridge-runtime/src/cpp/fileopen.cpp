// Copyright (C) 2026 The Qt Company Ltd.
// SPDX-License-Identifier: LicenseRef-Qt-Commercial OR LGPL-3.0-only

#include "fileopen.h"

#include <QCoreApplication>
#include <QEvent>
#include <QFileOpenEvent>
#include <QGuiApplication>
#include <QObject>
#include <QString>

#include "qtbridge-runtime/src/fileopen.rs.h"

namespace rust::bridge::fileopen {

namespace {

// Catches QEvent::FileOpen on the application and hands the path of the file
// to the Rust handler. Parented to the application so that it dies with it.
// Has no Q_OBJECT: it is found again by its object name.
class FileOpenEventFilter : public QObject
{
public:
    FileOpenEventFilter(QGuiApplication* app, ::rust::Box<FileOpenHandler> handler)
        : QObject(app), m_handler(std::move(handler))
    {
        setObjectName(objectNameOfFilter());
        app->installEventFilter(this);
    }

    static QString objectNameOfFilter()
    {
        return QStringLiteral("rust::bridge::fileopen::FileOpenEventFilter");
    }

    void setHandler(::rust::Box<FileOpenHandler> handler)
    {
        m_handler = std::move(handler);
    }

protected:
    bool eventFilter(QObject* watched, QEvent* event) override
    {
        if (event->type() != QEvent::FileOpen)
            return QObject::eventFilter(watched, event);
        const QString file = static_cast<QFileOpenEvent*>(event)->file();
        m_handler->call(::rust::String(file.toStdString()));
        return true;
    }

private:
    ::rust::Box<FileOpenHandler> m_handler;
};

} // namespace

void installFileOpenHandler(QGuiApplication& app, ::rust::Box<FileOpenHandler> handler)
{
    // One filter per application: a second handler replaces the first.
    auto* filter = app.findChild<QObject*>(
        FileOpenEventFilter::objectNameOfFilter(), Qt::FindDirectChildrenOnly);
    if (filter) {
        static_cast<FileOpenEventFilter*>(filter)->setHandler(std::move(handler));
        return;
    }
    new FileOpenEventFilter(&app, std::move(handler));
}

void postFileOpenEvent(QGuiApplication& app, ::rust::Str path)
{
    const QString file = QString::fromUtf8(path.data(), static_cast<qsizetype>(path.size()));
    QCoreApplication::postEvent(&app, new QFileOpenEvent(file));
}

} // namespace rust::bridge::fileopen
