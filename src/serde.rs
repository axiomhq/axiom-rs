use serde::de::{Deserialize, Deserializer};

/// Set `deserialize_with` to this fn to get the default if null.
/// See <https://github.com/serde-rs/serde/issues/1098#issuecomment-760711617>
pub(crate) fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    T: Default + Deserialize<'de>,
    D: Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}
