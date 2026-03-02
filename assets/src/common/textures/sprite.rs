use common_model::color::{ColorModel, default_white, is_default_white};
use common_model::numbers::glam_wraps::UVec2Model;
use crate::common::textures::SpriteId;
use crate::common::tilemap::Tilemap;
use facet::Facet;
use itertools::Itertools as _;
use registry::registry::id::{IdRef, RawId};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::de::{Error, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use serde_json::json;
use std::borrow::Cow;
use std::fmt::{Debug, Formatter};

/// Sprite data, currently contains texture and an optional tint
///
/// Can be deserialized from various formats
/// - A plain string with texture name
/// - An object with `sprite` and `tint` fields
/// - An object with `tilemap`, `index` and `tint` fields
#[derive(Clone, Facet, Eq, PartialEq, Hash)]
#[repr(C)]
pub enum SpriteModel {
    Sprite {
        sprite: SpriteId,
        tint: ColorModel,
    },
    Tilemap {
        tilemap: IdRef<Tilemap>,
        index: UVec2Model,
        tint: ColorModel,
    },
    SolidColor {
        color: ColorModel,
    },
}

impl SpriteModel {
    /// Returns tint color, or white color if none
    #[must_use]
    pub fn tint_or_white(&self) -> macroquad::color::Color {
        match self {
            SpriteModel::Sprite { tint, .. }
            | SpriteModel::Tilemap { tint, .. }
            | SpriteModel::SolidColor { color: tint } => tint.0,
        }
    }
}

impl Debug for SpriteModel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SpriteModel::Sprite { sprite, tint } => {
                if tint == &default_white() {
                    write!(f, "Sprite({})", sprite.raw().as_str())
                } else {
                    write!(f, "Sprite({}, {:?})", sprite.raw().as_str(), tint)
                }
            }
            SpriteModel::Tilemap {
                tint,
                index,
                tilemap,
            } => {
                let index = index.glam();
                if tint == &default_white() {
                    write!(
                        f,
                        "Sprite(tilemap: {}, [{};{}])",
                        tilemap.raw().as_str(),
                        index.x,
                        index.y
                    )
                } else {
                    write!(
                        f,
                        "Sprite(tilemap: {}, [{};{}], {:?})",
                        tilemap.raw().as_str(),
                        index.x,
                        index.y,
                        tint
                    )
                }
            }
            SpriteModel::SolidColor { color } => {
                write!(f, "SolidColor({color:?})")
            }
        }
    }
}

const _: () = {
    #[derive(Clone, Serialize, Deserialize)]
    struct SpriteDataModel {
        sprite: Option<SpriteId>,
        tint: Option<ColorModel>,
        solid_color: Option<ColorModel>,
        tilemap: Option<IdRef<Tilemap>>,
        index: Option<UVec2Model>,
    }

    impl JsonSchema for SpriteModel {
        fn schema_name() -> Cow<'static, str> {
            "SpriteData".into()
        }

        fn json_schema(generator: &mut SchemaGenerator) -> Schema {
            let mut white_color = generator.subschema_for::<ColorModel>();
            white_color.insert("default".to_string(), json!([1.0, 1.0, 1.0, 1.0]));

            let desc = SpriteModel::SHAPE
                .doc
                .iter()
                .map(|l| l.strip_prefix(' ').unwrap_or(l).trim_end())
                .join("\n");

            json_schema!({
              "description": desc,
              "oneOf": [
                {
                  "type": "string"
                },
                {
                  "type": "object",
                  "properties": {
                    "sprite": {
                      "type": "string"
                    },
                    "tint": white_color
                  },
                  "required": [
                    "sprite"
                  ]
                },
                {
                  "type": "object",
                  "properties": {
                    "index": generator.subschema_for::<UVec2Model>(),
                    "tilemap": {
                      "type": "string"
                    },
                    "tint": white_color
                  },
                  "required": [
                    "tilemap",
                    "index"
                  ]
                }
              ]
            })
        }
    }

    impl Serialize for SpriteModel {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            match self {
                SpriteModel::Sprite { tint, sprite } => SpriteDataModel {
                    sprite: Some(*sprite),
                    tint: if is_default_white(tint) { None } else { Some(*tint) },
                    solid_color: None,
                    tilemap: None,
                    index: None,
                }
                .serialize(serializer),
                SpriteModel::Tilemap {
                    tint,
                    tilemap,
                    index,
                } => SpriteDataModel {
                    sprite: None,
                    tint: if is_default_white(tint) { None } else { Some(*tint) },
                    solid_color: None,
                    tilemap: Some(*tilemap),
                    index: Some(*index),
                }
                .serialize(serializer),
                SpriteModel::SolidColor { color } => {
                    SpriteDataModel {
                        sprite: None,
                        tint: None,
                        solid_color: Some(*color),
                        tilemap: None,
                        index: None,
                    }
                    .serialize(serializer)
                }
            }
        }
    }

    impl<'de> Deserialize<'de> for SpriteModel {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct SpriteDataVisitor;

            impl<'de> Visitor<'de> for SpriteDataVisitor {
                type Value = SpriteModel;

                fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                    formatter.write_str("string or map")
                }

                fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    Ok(SpriteModel::Sprite {
                        sprite: SpriteId::new(RawId::new(v)),
                        tint: default_white(),
                    })
                }

                fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
                where
                    A: MapAccess<'de>,
                {
                    let model = <SpriteDataModel as Deserialize>::deserialize(
                        de::value::MapAccessDeserializer::new(map),
                    )?;

                    match (model.sprite, model.tilemap, model.solid_color) {
                        (Some(sprite), None, None) => {
                            if model.index.is_some() {
                                return Err(Error::unknown_field("index", &["sprite", "tint"]));
                            }
                            if model.solid_color.is_some() {
                                return Err(Error::unknown_field("solid_color", &["sprite", "tint"]));
                            }
                            Ok(SpriteModel::Sprite {
                                sprite,
                                tint: model.tint.unwrap_or(default_white()),
                            })
                        }
                        (None, Some(tilemap), None) => {
                            if model.solid_color.is_some() {
                                return Err(Error::unknown_field("solid_color", &["tilemap", "index", "tint"]));
                            }
                            let index = model
                                .index
                                .ok_or_else(|| Error::missing_field("index"))?;
                            Ok(SpriteModel::Tilemap {
                                tilemap,
                                index,
                                tint: model.tint.unwrap_or(default_white()),
                            })
                        }
                        (None, None, Some(color)) => {
                            if model.tint.is_some() {
                                return Err(Error::unknown_field("tint", &["solid_color"]));
                            }
                            Ok(SpriteModel::SolidColor { color })
                        }
                        (None, None, None) => Err(Error::custom(
                            "either sprite or tilemap field must be present",
                        )),
                        _ => Err(Error::custom(
                            "only one of `sprite`, `tilemap` or `solid_color` fields can be present",
                        )),
                    }
                }
            }

            deserializer.deserialize_any(SpriteDataVisitor)
        }
    }
};
