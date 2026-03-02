use assets_manager::BoxedError;
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
struct PathError<E: Error + 'static>(serde_path_to_error::Error<E>);

impl<E: Error + 'static> Display for PathError<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error at {}", self.0.path())
    }
}

impl<E: Error + 'static> Error for PathError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}

pub fn load_yaml<'de, T: serde::Deserialize<'de>>(bytes: &'de [u8]) -> Result<T, BoxedError> {
    let str = std::str::from_utf8(bytes).map_err(BoxedError::from)?;
    let yd = serde_norway::Deserializer::from_str(str);
    let jv = serde_path_to_error::deserialize::<_, serde_json::Value>(yd)?;
    let res = serde_path_to_error::deserialize::<_, T>(jv);
    res.map_err(|err| BoxedError::from(PathError(err)))
}

pub fn load_toml<'de, T: serde::Deserialize<'de>>(bytes: &'de [u8]) -> Result<T, BoxedError> {
    let str = std::str::from_utf8(bytes).map_err(BoxedError::from)?;
    let td = toml::de::Deserializer::parse(str).map_err(BoxedError::from)?;
    serde_path_to_error::deserialize::<_, T>(td).map_err(|err| BoxedError::from(PathError(err)))
}

pub fn load_json5<'de, T: serde::Deserialize<'de>>(bytes: &'de [u8]) -> Result<T, BoxedError> {
    let str = std::str::from_utf8(bytes).map_err(BoxedError::from)?;
    let jd = &mut serde_json5::Deserializer::from_str(str).map_err(BoxedError::from)?;
    serde_path_to_error::deserialize::<_, T>(jd).map_err(|err| BoxedError::from(PathError(err)))
}

pub fn load_auto<'de, T: serde::Deserialize<'de>>(bytes: &'de [u8]) -> Result<T, BoxedError> {
    if bytes.starts_with(b"{") {
        load_json5::<T>(bytes)
    } else if bytes.starts_with(b"[") {
        load_toml::<T>(bytes)
    } else {
        load_yaml::<T>(bytes)
        // Err(BoxedError::from("Unsupported format for auto-detection"))
    }
}