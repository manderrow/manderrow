//! A list of QObjects to give to QML.

use cxx_qt_lib::{QList, QObjectMutPtr};

/// A list of QObjects, as Qt's `QObjectList`, which QML knows as a `list<QObject>`.
///
/// This is cxx-qt-lib's `QList<QObjectMutPtr>`, which is the same C++ type, but which the
/// generated type information names `QList_QObjectMutPtr`, a name that qmllint and qmlls don't
/// know, so a property or method of that type would be untyped in QML.
#[repr(transparent)]
#[derive(Clone, Default)]
pub struct QObjectList(pub QList<QObjectMutPtr>);

// SAFETY: QObjectList is a typedef of QList<QObject *>, as QList<QObjectMutPtr> is, which
// cxx-qt-lib declares to be trivial
unsafe impl cxx::ExternType for QObjectList {
    type Id = cxx::type_id!("QObjectList");
    type Kind = cxx::kind::Trivial;
}

impl QObjectList {
    pub fn push(&mut self, object: QObjectMutPtr) {
        self.0.append(object);
    }
}

impl FromIterator<QObjectMutPtr> for QObjectList {
    fn from_iter<I: IntoIterator<Item = QObjectMutPtr>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}
