use num_traits::{Num, NumAssignRef};
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;
use std::fmt::Debug;
use bevy_reflect::Reflect;
use serde::de::Error;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Reflect)]
pub struct RangeInclusiveModel<T: Debug + Clone + NumAssignRef + PartialOrd> {
    from: T,
    to: T,
    step: T,
}

impl<T: Debug + Clone + NumAssignRef + PartialOrd> RangeInclusiveModel<T> {
    pub fn from(&self) -> &T {
        &self.from
    }

    pub fn to(&self) -> &T {
        &self.to
    }

    pub fn step(&self) -> &T {
        &self.step
    }

    pub fn contains(&self, element: &T) -> bool {
        let (min, max) = self.min_max();
        if element < min || element > max {
            return false;
        }
        let mut elem = element.clone();
        elem -= min;
        elem %= &self.step;
        elem.is_zero()
    }

    pub fn len(&self) -> T {
        let (min, max) = self.min_max();
        let mut val = max.clone();
        val -= min;
        val /= &self.step;
        val
    }

    pub fn iter(&self) -> impl Iterator<Item = T> {
        struct Iter<T> {
            current: T,
            end: T,
            step: T,
            ascending: bool,
        }

        impl<T: Debug + Clone + NumAssignRef + PartialOrd> Iterator for Iter<T> {
            type Item = T;

            fn next(&mut self) -> Option<Self::Item> {
                if (self.ascending && self.current > self.end)
                    || (!self.ascending && self.current < self.end)
                {
                    None
                } else {
                    let value = self.current.clone();
                    if self.ascending {
                        self.current += &self.step;
                    } else {
                        self.current -= &self.step;
                    }
                    Some(value)
                }
            }
        }

        Iter {
            current: self.from.clone(),
            end: self.to.clone(),
            step: self.step.clone(),
            ascending: self.from <= self.to,
        }
    }

    fn min_max(&self) -> (&T, &T) {
        if self.from <= self.to {
            (&self.from, &self.to)
        } else {
            (&self.to, &self.from)
        }
    }
}

fn validate_range<T: Debug + Clone + NumAssignRef + PartialOrd>(
    range: RangeInclusiveModel<T>,
) -> Result<RangeInclusiveModel<T>, String> {
    let (min, max) = range.min_max();
    let mut val = max.clone();
    val -= min;
    val %= &range.step;
    if !val.is_zero() {
        return Err(format!(
            "Range step {:?} does not evenly divide the range from {:?} to {:?}",
            range.step, min, max
        ));
    }
    Ok(range)
}

const _: () = {
    #[derive(JsonSchema)]
    #[serde(untagged)]
    #[allow(dead_code)]
    enum RangeInclusiveModelSchema<T: Num> {
        Single(T),
        Range {
            from: T,
            to: T,
            #[schemars(default = "T::one")]
            step: T,
        },
        RangeSliceWithStep(T, T, T),
        RangeSliceWithoutStep(T, T),
    }

    impl<T: JsonSchema + Debug + Clone + NumAssignRef + PartialOrd> JsonSchema
        for RangeInclusiveModel<T>
    {
        fn schema_name() -> Cow<'static, str> {
            format!("{}RangeInclusive", T::schema_name()).into()
        }

        fn json_schema(generator: &mut SchemaGenerator) -> Schema {
            RangeInclusiveModelSchema::<T>::json_schema(generator)
        }
    }

    #[derive(Serialize)]
    struct RangeInclusiveModelSerialize<'a, T: Num> {
        from: &'a T,
        to: &'a T,
        #[serde(skip_serializing_if = "T::is_one")]
        step: &'a T,
    }

    #[derive(Deserialize)]
    struct RangeInclusiveModelDeSerialize<T: Num> {
        from: T,
        to: T,
        #[serde(default = "T::one")]
        step: T,
    }

    impl<T: Serialize + Debug + Clone + NumAssignRef + PartialOrd> Serialize
        for RangeInclusiveModel<T>
    {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            if self.from == self.to {
                self.from.serialize(serializer)
            } else {
                RangeInclusiveModelSerialize {
                    from: &self.from,
                    to: &self.to,
                    step: &self.step,
                }
                .serialize(serializer)
            }
        }
    }

    impl<'de, T: Deserialize<'de> + Debug + Clone + NumAssignRef + PartialOrd> Deserialize<'de>
        for RangeInclusiveModel<T>
    {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct RangeInclusiveModelVisitor<T> {
                marker: std::marker::PhantomData<T>,
            }

            impl<'de, T: Deserialize<'de> + Debug + Clone + NumAssignRef + PartialOrd>
                serde::de::Visitor<'de> for RangeInclusiveModelVisitor<T>
            {
                type Value = RangeInclusiveModel<T>;

                fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    formatter.write_str("a single value or a range with optional step")
                }

                fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    let de = serde::de::value::F64Deserializer::new(v);
                    let value = T::deserialize(de)?;
                    Ok(RangeInclusiveModel {
                        from: value.clone(),
                        to: value,
                        step: T::one(),
                    })
                }

                fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    let de = serde::de::value::U64Deserializer::new(v);
                    let value = T::deserialize(de)?;
                    Ok(RangeInclusiveModel {
                        from: value.clone(),
                        to: value,
                        step: T::one(),
                    })
                }

                fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
                where
                    A: serde::de::SeqAccess<'de>,
                {
                    let size_hint = seq.size_hint();
                    let de = serde::de::value::SeqAccessDeserializer::new(seq);
                    let (from, to, step) = match size_hint {
                        Some(2) => {
                            let [from, to] = <[T; 2]>::deserialize(de)?;
                            (from, to, T::one())
                        }
                        Some(3) => {
                            let [from, to, step] = <[T; 3]>::deserialize(de)?;
                            (from, to, step)
                        }
                        _ => {
                            let value = Vec::<T>::deserialize(de)?;

                            if value.len() == 2 {
                                let [from, to] = <[T; 2]>::try_from(value).unwrap();
                                (from, to, T::one())
                            } else if value.len() == 3 {
                                let [from, to, step] = <[T; 3]>::try_from(value).unwrap();
                                (from, to, step)
                            } else {
                                return Err(serde::de::Error::invalid_length(
                                    value.len(),
                                    &"2 or 3 elements",
                                ));
                            }
                        }
                    };
                    Ok(RangeInclusiveModel { from, to, step })
                }

                fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
                where
                    A: serde::de::MapAccess<'de>,
                {
                    let de = serde::de::value::MapAccessDeserializer::new(map);
                    let value = RangeInclusiveModelDeSerialize::deserialize(de)?;
                    Ok(RangeInclusiveModel::<T> {
                        from: value.from,
                        to: value.to,
                        step: value.step,
                    })
                }
            }

            let value = deserializer.deserialize_any(RangeInclusiveModelVisitor::<T> {
                marker: std::marker::PhantomData,
            })?;

            if value.step <= T::zero() {
                Err(serde::de::Error::custom("Step must be greater than zero"))
            } else {
                validate_range(value).map_err(serde::de::Error::custom)
            }
        }
    }
};
