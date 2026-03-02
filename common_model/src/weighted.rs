use crate::numbers::nonneg::FiniteNonNegative;
use bevy_reflect::Reflect;
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt::Formatter;
use weighted_rand::builder::{NewBuilder as _, WalkerTableBuilder};
use weighted_rand::table::WalkerTable;

#[derive(Debug, Clone, Serialize, Reflect)]
pub struct WeightedItem<T> {
    pub item: T,
    pub weight: FiniteNonNegative<f32>,
}

#[derive(Debug, Clone, Serialize, Reflect)]
pub struct WeightedCollection<T> {
    items: Vec<WeightedItem<T>>,
    #[reflect(ignore)]
    wt: WalkerTable,
}

impl<T> WeightedCollection<T> {
    #[inline]
    #[must_use]
    pub fn get(&self, rng: &mut impl rand::Rng) -> &T {
        let index = self.wt.next_rng(rng);

        &self.items[index].item
    }

    fn try_new(items: Vec<WeightedItem<T>>) -> Result<Self, &'static str> {
        if items.is_empty() {
            return Err("WeightedCollection cannot be empty");
        }
        let index_weights = items.iter().map(|i| i.weight.get()).collect::<Vec<_>>();

        Ok(Self {
            wt: WalkerTableBuilder::new(&index_weights).build(),
            items,
        })
    }
}

const _: () = {
    #[derive(JsonSchema)]
    pub struct WeightedSchema<T> {
        pub item: T,
        pub weight: FiniteNonNegative<f32>,
    }

    impl<T: JsonSchema> JsonSchema for WeightedItem<T> {
        fn schema_name() -> Cow<'static, str> {
            format!("Weighted{}", T::schema_name()).into()
        }

        fn json_schema(generator: &mut SchemaGenerator) -> Schema {
            WeightedSchema::<T>::json_schema(generator)
        }
    }

    impl<'de, T: Deserialize<'de>> Deserialize<'de> for WeightedItem<T> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct WeightedVisitor<T> {
                marker: std::marker::PhantomData<T>,
            }

            impl<'de, T: Deserialize<'de>> Visitor<'de> for WeightedVisitor<T> {
                type Value = WeightedItem<T>;

                fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                    formatter.write_str("a weighted item struct, or a tuple of (item, weight)")
                }

                fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
                where
                    A: SeqAccess<'de>,
                {
                    let de = serde::de::value::SeqAccessDeserializer::new(seq);
                    let (item, weight): (T, FiniteNonNegative<f32>) = Deserialize::deserialize(de)?;
                    Ok(WeightedItem { item, weight })
                }

                fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
                where
                    A: MapAccess<'de>,
                {
                    #[derive(Deserialize)]
                    struct WeightedDe<T> {
                        item: T,
                        weight: FiniteNonNegative<f32>,
                    }

                    let de = serde::de::value::MapAccessDeserializer::new(map);
                    let item = WeightedDe::<T>::deserialize(de)?;
                    Ok(WeightedItem {
                        item: item.item,
                        weight: item.weight,
                    })
                }
            }

            deserializer.deserialize_any(WeightedVisitor {
                marker: Default::default(),
            })
        }
    }
};

const _: () = {
    #[derive(JsonSchema)]
    enum WeightedCollectionSchema<T> {
        Vec(Vec<WeightedItem<T>>),
        Map(BTreeMap<T, FiniteNonNegative<f32>>),
    }

    impl<T: JsonSchema> JsonSchema for WeightedCollection<T> {
        fn schema_name() -> Cow<'static, str> {
            format!("Weighted{}Collection", T::schema_name()).into()
        }

        fn json_schema(generator: &mut SchemaGenerator) -> Schema {
            WeightedCollectionSchema::<T>::json_schema(generator)
        }
    }

    impl<'de, T: Deserialize<'de>> Deserialize<'de> for WeightedCollection<T> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct WeightedCollectionVisitor<T> {
                marker: std::marker::PhantomData<T>,
            }

            impl<'de, T: Deserialize<'de>> Visitor<'de> for WeightedCollectionVisitor<T> {
                type Value = WeightedCollection<T>;

                fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                    write!(
                        formatter,
                        "a list of weighted items or a map of items to weights"
                    )
                }

                fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
                where
                    A: SeqAccess<'de>,
                {
                    let de = serde::de::value::SeqAccessDeserializer::new(seq);
                    let items = Vec::<WeightedItem<T>>::deserialize(de)?;
                    WeightedCollection::try_new(items).map_err(serde::de::Error::custom)
                }

                fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
                where
                    A: MapAccess<'de>,
                {
                    let mut items = vec![];
                    while let Some((item, weight)) =
                        map.next_entry::<T, FiniteNonNegative<f32>>()?
                    {
                        items.push(WeightedItem { item, weight });
                    }

                    WeightedCollection::try_new(items).map_err(serde::de::Error::custom)
                }
            }

            deserializer.deserialize_any(WeightedCollectionVisitor {
                marker: Default::default(),
            })
        }
    }
};
