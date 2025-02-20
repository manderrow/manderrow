use serde::Deserialize;

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
