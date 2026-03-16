use crate::registry::id::lexing::{Namespace, path_errors};
use crate::traverse::TraverseKind;
use bevy_reflect::{Reflect, TypePath};
use itertools::Itertools as _;
use rootcause::{bail, report};
use schemars::_private::serde_json;
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;
use std::fmt::{Debug, Formatter};
use std::hash::Hash;
use std::marker::PhantomData;
use std::str::FromStr;
use ustr::Ustr;

mod lexing;

#[derive(Reflect)]
#[reflect(@TraverseKind::IdRef)]
#[reflect(Clone)]
pub struct IdRef<T> {
    id: RawId,
    #[reflect(ignore)]
    _t: PhantomData<fn() -> T>,
}

impl<T> IdRef<T> {
    #[must_use]
    pub fn new(id: RawId) -> Self {
        Self {
            id,
            _t: Default::default(),
        }
    }

    #[must_use]
    pub fn raw(&self) -> &RawId {
        &self.id
    }
}

impl<T> Debug for IdRef<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IdRef").field("id", &self.id).finish()
    }
}

impl<T> PartialEq for IdRef<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T> Eq for IdRef<T> {}

impl<T> Hash for IdRef<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<T> PartialOrd for IdRef<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for IdRef<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

impl<T> Copy for IdRef<T> {}

impl<T> Clone for IdRef<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: TypePath> JsonSchema for IdRef<T> {
    fn inline_schema() -> bool {
        false
    }

    fn schema_name() -> Cow<'static, str> {
        format!(
            "{}Id",
            T::type_ident().expect("All types in schema are identifiable")
        )
        .into()
    }

    fn schema_id() -> Cow<'static, str> {
        Self::schema_name()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        String::json_schema(generator)
    }
}

impl<T> Serialize for IdRef<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.id.serialize(serializer)
    }
}

impl<'de, T> Deserialize<'de> for IdRef<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let id = RawId::deserialize(deserializer)?;
        Ok(Self {
            id,
            _t: PhantomData,
        })
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Reflect)]
#[serde(transparent)]
#[reflect(opaque, Clone)]
pub struct RawId(pub(crate) Ustr);

impl RawId {
    pub fn new(id: impl Into<Ustr>) -> Self {
        let id = id.into();
        Self::from_str(id.as_str()).expect("All directly constructed raw ids must be valid")
    }

    pub fn try_new(id: impl Into<Ustr>) -> rootcause::Result<Self> {
        let id = id.into();
        Self::from_str(id.as_str())
    }

    #[must_use]
    pub fn as_str(&self) -> &'static str {
        self.0.as_str()
    }
}

impl FromStr for RawId {
    type Err = rootcause::Report;

    fn from_str(data: &str) -> Result<Self, Self::Err> {
        let (namespace, path): (&str, &str) = data
            .split(':')
            .collect_tuple()
            .ok_or_else(|| report!("ID must be in a form of `namespace:path`"))?;

        let namespace = Namespace::from_str(namespace)?;

        if path.is_empty() {
            bail!("ID can't be empty");
        }

        if let Some((i, c)) = path_errors(path) {
            bail!(
                "Invalid symbol `{}` in ID, at position {}",
                c,
                i + namespace.as_ref().len() + 1
            );
        }

        Ok(Self(data.into()))
    }
}

impl JsonSchema for RawId {
    fn inline_schema() -> bool {
        String::inline_schema()
    }

    fn schema_name() -> Cow<'static, str> {
        "RawId".into()
    }

    fn schema_id() -> Cow<'static, str> {
        "RawId".into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        let mut schema = String::json_schema(generator);
        schema.insert(
            "pattern".to_string(),
            serde_json::json!("^[0-9a-z_]+:[0-9a-z_/]+$"),
        );

        schema
    }
}

impl<'de> Deserialize<'de> for RawId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let str = String::deserialize(deserializer)?;
        Self::from_str(&str).map_err(serde::de::Error::custom)
    }
}
