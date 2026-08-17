use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! id_type {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn try_new(value: impl Into<String>) -> crate::kernel::KernelResult<Self> {
                let value = value.into();
                if value.is_empty() {
                    return Err(crate::kernel::KernelError::invalid_input(concat!(
                        stringify!($name),
                        " cannot be empty"
                    )));
                }
                Ok(Self(value))
            }

            pub fn generate() -> Self {
                Self(Uuid::now_v7().to_string())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }

            pub fn into_string(self) -> String {
                self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self::new(value)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self::new(value)
            }
        }
    };
}

id_type!(CapabilityId);
id_type!(StageId);
id_type!(PolicyId);
id_type!(Topic);
id_type!(SubscriptionId);
id_type!(TraceId);
id_type!(SpanId);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_new_rejects_empty_ids_without_panicking() {
        let error = Topic::try_new("").unwrap_err();
        assert!(error.to_string().contains("Topic cannot be empty"));
    }

    #[test]
    fn from_empty_id_is_not_a_release_runtime_assertion() {
        let topic = Topic::from("");
        assert_eq!(topic.as_str(), "");
    }
}
