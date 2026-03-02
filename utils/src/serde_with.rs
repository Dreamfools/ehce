use schemars::JsonSchema;

pub mod opt_color {
    use glam::Vec4;
    use serde::{Deserialize as _, Deserializer, Serialize as _, Serializer};

    pub fn serialize<S>(c: &Option<bevy_color::LinearRgba>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        c.map(|c| Vec4::from([c.red, c.green, c.blue, c.alpha]))
            .serialize(s)
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Option<bevy_color::LinearRgba>, D::Error>
    where
        D: Deserializer<'de>,
    {
        match Option::<super::color::ColorDataSerialized>::deserialize(d)? {
            None => Ok(None),
            Some(data) => Ok(Some(data.into_color::<D>()?)),
        }
    }
}

pub mod color {
    use glam::Vec4;
    use schemars::JsonSchema;
    use serde::de::Error as _;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(c: &bevy_color::LinearRgba, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Vec4::from([c.red, c.green, c.blue, c.alpha]).serialize(s)
    }

    pub fn deserialize<'de, D>(d: D) -> Result<bevy_color::LinearRgba, D::Error>
    where
        D: Deserializer<'de>,
    {
        ColorDataSerialized::deserialize(d)?.into_color::<D>()
    }

    #[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
    #[serde(untagged)]
    pub enum ColorDataSerialized {
        String(String),
        Array([f32; 4]),
        Obj { r: f32, g: f32, b: f32, a: f32 },
    }

    impl ColorDataSerialized {
        pub fn into_color<'de, D: Deserializer<'de>>(
            self,
        ) -> Result<bevy_color::LinearRgba, D::Error> {
            match self {
                ColorDataSerialized::String(string) => {
                    let data = csscolorparser::parse(&string)
                        .map_err(|e| D::Error::custom(format!("color format error: {e}")))?;
                    Ok(bevy_color::LinearRgba::new(data.r, data.g, data.b, data.a))
                }
                ColorDataSerialized::Array([r, g, b, a])
                | ColorDataSerialized::Obj { r, g, b, a } => {
                    Ok(bevy_color::LinearRgba::new(r, g, b, a))
                }
            }
        }
    }
}

#[derive(JsonSchema)]
pub struct IVec2Schema {
    pub x: i32,
    pub y: i32,
}
