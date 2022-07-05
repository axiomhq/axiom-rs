use serde::de::{Deserialize, Deserializer, IntoDeserializer};

/// Set `deserialize_with` to this fn to get the default if null.
/// See https://github.com/serde-rs/serde/issues/1098#issuecomment-760711617
pub(crate) fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    T: Default + Deserialize<'de>,
    D: Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

// Set `deserialize_with` to this fn to get the none for an empty string.
// Stolen from https://github.com/serde-rs/serde/issues/1425#issuecomment-462282398
pub(crate) fn empty_string_as_none<'de, D, T>(de: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    let opt = Option::<String>::deserialize(de)?;
    let opt = opt.as_deref();
    match opt {
        None | Some("") => Ok(None),
        Some(s) => T::deserialize(s.into_deserializer()).map(Some),
    }
}
