use std::str::FromStr;

use chrono::{DateTime, Utc};
use rkyv::{
    rancor::{Fallible, Source},
    rend::i64_le,
};

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Timestamp(i64);

impl Timestamp {
    pub fn get(self) -> DateTime<Utc> {
        unsafe { DateTime::<Utc>::from_timestamp_micros(self.0).unwrap_unchecked() }
    }
}

impl From<DateTime<Utc>> for Timestamp {
    fn from(value: DateTime<Utc>) -> Self {
        Self(value.timestamp_micros())
    }
}

impl FromStr for Timestamp {
    type Err = <DateTime<Utc> as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<DateTime<Utc>>().map(Self::from)
    }
}

#[derive(Debug, Clone, Copy, rkyv::Portable)]
#[repr(transparent)]
pub struct ArchivedTimestamp(i64_le);

impl From<ArchivedTimestamp> for Timestamp {
    fn from(value: ArchivedTimestamp) -> Self {
        Timestamp(value.0.into())
    }
}

unsafe impl<C: Fallible + ?Sized> rkyv::bytecheck::CheckBytes<C> for ArchivedTimestamp
where
    C::Error: Source,
{
    unsafe fn check_bytes(value: *const Self, _: &mut C) -> Result<(), C::Error> {
        let value = unsafe { value.read().0.to_native() };
        match DateTime::<Utc>::from_timestamp_micros(value) {
            Some(_) => Ok(()),
            None => {
                #[derive(Debug, thiserror::Error)]
                #[error("Timestamp value is out of bounds: {0}")]
                struct Error(i64);
                Err(C::Error::new(Error(value)))
            }
        }
    }
}

impl rkyv::Archive for Timestamp {
    type Archived = ArchivedTimestamp;

    type Resolver = ();

    fn resolve(&self, (): Self::Resolver, out: rkyv::Place<Self::Archived>) {
        unsafe { out.write_unchecked(ArchivedTimestamp(self.0.into())) }
    }
}

impl<S: Fallible + ?Sized> rkyv::Serialize<S> for Timestamp {
    fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<'de> serde::Deserialize<'de> for Timestamp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        DateTime::<Utc>::deserialize(deserializer).map(Timestamp::from)
    }
}

impl serde::Serialize for Timestamp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.get().serialize(serializer)
    }
}
