use std::borrow::Cow;
use std::fmt::{Display, Formatter, Result as FmtResult};

use symbolic::common::{ByteView, DebugId};
use uuid::Uuid;

use crate::utils::chunks::Assemblable;

pub struct ProguardMapping<'a> {
    bytes: ByteView<'a>,
    uuid: Uuid,
}

impl ProguardMapping<'_> {
    /// Get the UUID of the mapping.
    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    /// Force the UUID of the mapping to a specific value, rather
    /// than the UUID which is derived from the proguard crate.
    pub fn force_uuid(&mut self, uuid: Uuid) {
        self.uuid = uuid;
    }
}

impl<'a> From<ByteView<'a>> for ProguardMapping<'a> {
    fn from(value: ByteView<'a>) -> Self {
        let mapping = ::proguard::ProguardMapping::new(&value);
        let uuid = mapping.uuid();
        Self { bytes: value, uuid }
    }
}

impl AsRef<[u8]> for ProguardMapping<'_> {
    fn as_ref(&self) -> &[u8] {
        self.bytes.as_ref()
    }
}

impl Assemblable for ProguardMapping<'_> {
    fn name(&self) -> Cow<'_, str> {
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
