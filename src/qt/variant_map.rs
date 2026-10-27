//! A map of values to give QML.

use cxx_qt_lib::{QMap, QMapPair_QString_QVariant, QString, QVariant};

/// A map from strings to values, as Qt's `QVariantMap`, which QML knows as an object.
///
/// This is cxx-qt-lib's `QMap<QMapPair_QString_QVariant>`, which is the same C++ type, but which
/// the generated type information names `QMap_QString_QVariant`, a name that qmllint and qmlls
/// don't know, so a property of that type would be untyped in QML.
#[repr(transparent)]
#[derive(Clone, Default)]
pub struct QVariantMap(pub QMap<QMapPair_QString_QVariant>);

// SAFETY: QVariantMap is a typedef of QMap<QString, QVariant>, as QMap<QMapPair_QString_QVariant>
// is, which cxx-qt-lib declares to be trivial
unsafe impl cxx::ExternType for QVariantMap {
    type Id = cxx::type_id!("QVariantMap");
    type Kind = cxx::kind::Trivial;
}

impl FromIterator<(QString, QVariant)> for QVariantMap {
    fn from_iter<I: IntoIterator<Item = (QString, QVariant)>>(iter: I) -> Self {
        let mut map = QMap::default();
        for (key, value) in iter {
            map.insert(key, value);
        }
        Self(map)
    }
}
