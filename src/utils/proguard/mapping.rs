use std::borrow::Cow;
use std::fmt::{Display, Formatter, Result as FmtResult};

use symbolic::common::{ByteView, DebugId};
use thiserror::Error;
use uuid::Uuid;

use crate::utils::chunks::Assemblable;

#[derive(Debug, Error)]
pub enum ProguardMappingError {
    #[error("Proguard mapping does not contain line information")]
    MissingLineInfo,
}

pub struct ProguardMapping<'a> {
    bytes: ByteView<'a>,
    uuid: Uuid,
}

impl<'a> ProguardMapping<'a> {
    /// Get the length of the mapping in bytes.
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Get the UUID of the mapping.
    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    /// Force the UUID of the mapping to a specific value, rather
    /// than the UUID which is derived from the proguard crate.
    pub fn force_uuid(&mut self, uuid: Uuid) {
        self.uuid = uuid;
    }

    /// Create a new `ProguardMapping` from a `ByteView`.
    /// Not public because we want to ensure that the `ByteView` contains line
    /// information, and this method does not check for that. To create a
    /// `ProguardMapping` externally, use the `TryFrom<ByteView>` implementation.
    fn new(bytes: ByteView<'a>, uuid: Uuid) -> Self {
        Self { bytes, uuid }
    }
}

impl<'a> TryFrom<ByteView<'a>> for ProguardMapping<'a> {
    type Error = ProguardMappingError;

    /// Try to create a `ProguardMapping` from a `ByteView`.
    /// The method returns an error if the mapping does not contain
    /// line information.
    fn try_from(value: ByteView<'a>) -> Result<Self, Self::Error> {
        let mapping = ::proguard::ProguardMapping::new(&value);

        if !mapping.has_line_info() {
            return Err(ProguardMappingError::MissingLineInfo);
        }

        let uuid = mapping.uuid();
        Ok(ProguardMapping::new(value, uuid))
    }
}

impl AsRef<[u8]> for ProguardMapping<'_> {
    fn as_ref(&self) -> &[u8] {
        self.bytes.as_ref()
    }
}

impl Assemblable for ProguardMapping<'_> {
    fn name(&self) -> Cow<str> {
        format!("/proguard/{}.txt", self.uuid).into()
    }

    fn debug_id(&self) -> Option<DebugId> {
        None
    }
}

impl Display for ProguardMapping<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{} (Proguard mapping)", self.uuid)
    }
}
