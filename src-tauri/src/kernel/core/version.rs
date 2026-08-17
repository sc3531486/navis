use serde::{Deserialize, Serialize};

use super::error::{KernelError, KernelResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SchemaVersion {
    pub major: u16,
    pub minor: u16,
}

impl SchemaVersion {
    pub const fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }

    pub fn ensure_registration_compatible(self, supported: Self) -> KernelResult<()> {
        if self.major != supported.major {
            return Err(KernelError::VersionMismatch {
                expected: supported.to_string(),
                actual: self.to_string(),
            });
        }
        Ok(())
    }

    pub fn supports_runtime_payload(self, payload: Self) -> bool {
        self.major == payload.major
    }

    pub fn supports_forward_compat(self, payload: Self) -> bool {
        self.supports_runtime_payload(payload) && self.minor < payload.minor
    }
}

impl Default for SchemaVersion {
    fn default() -> Self {
        Self { major: 1, minor: 0 }
    }
}

impl std::fmt::Display for SchemaVersion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}.{}", self.major, self.minor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registration_rejects_major_mismatch() {
        let err = SchemaVersion::new(2, 0)
            .ensure_registration_compatible(SchemaVersion::new(1, 4))
            .unwrap_err();
        assert!(matches!(err, KernelError::VersionMismatch { .. }));
    }

    #[test]
    fn runtime_payload_allows_any_minor_with_same_major() {
        assert!(SchemaVersion::new(1, 3).supports_runtime_payload(SchemaVersion::new(1, 1)));
        assert!(SchemaVersion::new(1, 1).supports_runtime_payload(SchemaVersion::new(1, 3)));
        assert!(!SchemaVersion::new(1, 1).supports_runtime_payload(SchemaVersion::new(2, 0)));
    }

    #[test]
    fn forward_compat_marks_low_minor_reading_high_minor() {
        assert!(SchemaVersion::new(1, 1).supports_forward_compat(SchemaVersion::new(1, 3)));
        assert!(!SchemaVersion::new(1, 3).supports_forward_compat(SchemaVersion::new(1, 1)));
    }
}
