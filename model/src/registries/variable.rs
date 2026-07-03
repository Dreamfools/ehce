use auto_ops::impl_op_ex;
use bevy_reflect::Reflect;
use registry::registry::id::IdRef;
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt::Formatter;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Reflect)]
#[serde(deny_unknown_fields)]
pub struct UnitVariableModel {
    /// The default value of the variable
    pub default_value: VariableValue,
    /// Whether the variable is read-only (cannot be modified after being initialized)
    pub readonly: bool,
}

pub type UnitVariableMap = BTreeMap<IdRef<UnitVariableModel>, VariableValue>;

#[derive(Debug, Copy, Clone, Serialize, Reflect)]
#[serde(deny_unknown_fields)]
pub struct VariableValue {
    /// Base value of the variable. Stacks additively with other base values
    ///  (e.g., 10 + 5 = 15)
    pub base: f64,
    /// Percentage bonus to the variable (e.g., 0.1 for +10%). Stacks additively
    /// with other bonuses (e.g., 0.1 + 0.2 = 0.3 for +30%) and applies after
    /// base value
    pub bonus: f64,
    /// Multiplier to the variable (e.g., 2.0 for x2). Stacks multiplicatively
    /// with other multipliers (e.g., 2.0 * 1.5 = 3.0 for x3) and applies after
    /// bonus
    pub multiplier: f64,
    /// Flat bonus to the variable (e.g., +5). Stacks additively with other
    /// flat bonuses and applies after all other calculations (e.g., `10<base>
    /// * 1.5<bonus> * 2<multiplier> + 5<flat> = 35`)
    pub flat: f64,
}

impl Default for VariableValue {
    fn default() -> Self {
        Self {
            base: 0.0,
            bonus: 0.0,
            multiplier: 1.0,
            flat: 0.0,
        }
    }
}

impl VariableValue {
    /// Combines two [VariableValue]s into one, stacking their values
    pub fn combine(&self, other: &Self) -> Self {
        Self {
            base: self.base + other.base,
            bonus: self.bonus + other.bonus,
            multiplier: self.multiplier * other.multiplier,
            flat: self.flat + other.flat,
        }
    }

    pub fn combine_in_place(&mut self, other: &Self) {
        self.base += other.base;
        self.bonus += other.bonus;
        self.multiplier *= other.multiplier;
        self.flat += other.flat;
    }

    /// Multiplies the [VariableValue] by a scalar factor, scaling its components
    ///
    /// [multiplier] is raised to the power of the factor, while [base], [bonus],
    /// and [flat] are scaled linearly
    pub fn multiply(&self, factor: f64) -> Self {
        Self {
            base: self.base * factor,
            bonus: self.bonus * factor,
            multiplier: self.multiplier.powf(factor),
            flat: self.flat * factor,
        }
    }

    /// Computes the final value of the variable based on its components
    pub fn compute(&self) -> f64 {
        self.base * (1.0 + self.bonus) * self.multiplier + self.flat
    }
}

impl_op_ex!(+ |a: &VariableValue, b: &VariableValue| -> VariableValue { a.combine(b) });
impl_op_ex!(+= |a: &mut VariableValue, b: &VariableValue| { a.combine_in_place(b) });

fn one() -> f64 {
    1.0
}

const _: () = {
    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
    #[serde(deny_unknown_fields)]
    #[schemars(inline)]
    pub struct FullVariableValueModel {
        #[serde(default)]
        pub base: f64,
        #[serde(default)]
        pub bonus: f64,
        #[serde(default = "one")]
        pub multiplier: f64,
        #[serde(default)]
        pub flat: f64,
    }

    #[derive(Debug, Clone, Serialize, JsonSchema)]
    #[serde(untagged)]
    #[allow(dead_code)]
    enum VariableValueSchema {
        Base(f64),
        Full(FullVariableValueModel),
    }

    impl JsonSchema for VariableValue {
        fn schema_name() -> Cow<'static, str> {
            "VariableValue".into()
        }

        fn json_schema(generator: &mut SchemaGenerator) -> Schema {
            VariableValueSchema::json_schema(generator)
        }
    }

    impl<'de> Deserialize<'de> for VariableValue {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            struct VariableValueModelVisitor;

            impl<'de> Visitor<'de> for VariableValueModelVisitor {
                type Value = VariableValue;

                fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                    write!(
                        formatter,
                        "a numeric base value or expanded variable values"
                    )
                }

                fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    Ok(VariableValue {
                        base: v,
                        bonus: 0.0,
                        multiplier: 1.0,
                        flat: 0.0,
                    })
                }

                fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    self.visit_f64(v as f64)
                }

                fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    self.visit_f64(v as f64)
                }

                fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
                where
                    A: MapAccess<'de>,
                {
                    let de = serde::de::value::MapAccessDeserializer::new(map);
                    let full = FullVariableValueModel::deserialize(de)?;
                    Ok(VariableValue {
                        base: full.base,
                        bonus: full.bonus,
                        multiplier: full.multiplier,
                        flat: full.flat,
                    })
                }
            }

            deserializer.deserialize_any(VariableValueModelVisitor)
        }
    }
};
