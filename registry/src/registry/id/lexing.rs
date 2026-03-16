use itertools::Itertools as _;
use rootcause::bail;
use serde::Deserializer;
use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;

#[inline(always)]
pub fn bad_namespace_char(c: char) -> bool {
    !matches!(c, 'a'..='z' | '0'..='9' | '_')
}

#[inline(always)]
pub fn bad_path_char(c: char) -> bool {
    !matches!(c, 'a'..='z' | '0'..='9' | '_' | '/')
}

#[inline(always)]
pub fn bad_ext_char(c: char) -> bool {
    !matches!(c, 'a'..='z' | '0'..='9' | '_')
}

pub fn namespace_errors(namespace: &str) -> Option<(usize, char)> {
    namespace.chars().find_position(|c| bad_namespace_char(*c))
}

pub fn path_errors(namespace: &str) -> Option<(usize, char)> {
    let mut is_ext = false;
    for (i, c) in namespace.chars().enumerate() {
        if is_ext {
            if bad_ext_char(c) {
                return Some((i, c));
            }
        } else if c == '.' {
            is_ext = true;
        } else if bad_path_char(c) {
            return Some((i, c));
        }
    }
    None
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct Namespace {
    id: String,
}

impl Debug for Namespace {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.id, f)
    }
}

impl Display for Namespace {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.id, f)
    }
}

impl AsRef<str> for Namespace {
    fn as_ref(&self) -> &str {
        &self.id
    }
}

impl From<Namespace> for String {
    fn from(value: Namespace) -> Self {
        value.id
    }
}

impl FromStr for Namespace {
    type Err = rootcause::Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            bail!("Namespace can't be empty");
        }

        if let Some((i, c)) = namespace_errors(s) {
            bail!("Invalid symbol `{}` in namespace, at position {}", c, i);
        }

        Ok(Namespace { id: s.into() })
    }
}

impl<'de> serde::Deserialize<'de> for Namespace {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct NamespaceVisitor;

        impl serde::de::Visitor<'_> for NamespaceVisitor {
            type Value = Namespace;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("a string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match Namespace::from_str(v) {
                    Err(e) => Err(serde::de::Error::custom(e)),
                    Ok(v) => Ok(v),
                }
            }
        }

        deserializer.deserialize_string(NamespaceVisitor)
    }
}
