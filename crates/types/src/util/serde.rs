use rkyv::vec::ArchivedVec;
use serde::{Deserialize, Deserializer, ser::SerializeSeq};

use super::rkyv::ArchivedInternedString;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, rkyv::Archive, rkyv::Serialize)]
#[rkyv(derive(Copy, Clone, Debug, Default, PartialEq, Eq))]
pub struct IgnoredAny;

impl<'de> Deserialize<'de> for IgnoredAny {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        serde::de::IgnoredAny::deserialize(deserializer).map(|_| Self)
    }
}

pub struct SerializeArchivedVec<'a, T: serde::Serialize>(pub &'a ArchivedVec<T>);

impl<T: serde::Serialize> serde::Serialize for SerializeArchivedVec<'_, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut ser = serializer.serialize_seq(Some(self.0.len()))?;
        for value in self.0.iter() {
            ser.serialize_element(value)?;
        }
        ser.end()
    }
}

impl serde::Serialize for ArchivedInternedString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self)
    }
}

pub fn empty_string_as_none<'de, D: Deserializer<'de>, T: AsRef<str> + Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    let o: Option<T> = Option::deserialize(d)?;
    Ok(o.filter(|s| !s.as_ref().is_empty()))
}
