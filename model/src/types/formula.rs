use crate::registries::variable::UnitVariableModel;
use bevy_reflect::{Reflect, TypePath};
use exmex::Express as _;
use registry::registry::id::{IdRef, RawId};
use rootcause::{bail, report};
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::sync::{Arc, LazyLock};
use utils::rootcause_ext::AttachField;

pub mod formula_context;

#[derive(Reflect)]
pub enum FormulaModel<C: FormulaModelContext> {
    Expr(ExprWithArgs<C>),
    Const(f64),
}

#[derive(Reflect)]
#[reflect(Clone)]
pub struct ExprWithArgs<C: FormulaModelContext> {
    #[reflect(ignore, default = "default_expr")]
    pub expr: Arc<exmex::FlatEx<f64>>,
    pub args: Vec<FormulaVariable>,
    #[reflect(ignore)]
    _c: PhantomData<fn() -> C>,
}

#[derive(Debug, Clone, Reflect)]
pub enum FormulaVariable {
    UnitVariable { id_ref: IdRef<UnitVariableModel> },
    Local(String),
}

pub trait FormulaModelContext: TypePath {
    /// Validates the variable
    fn validate_variable(var: &FormulaVariable) -> rootcause::Result<()>;

    /// A description of the formula, used in JSON schema
    #[must_use]
    fn description() -> String;

    /// Default namespace for variables without explicit namespace
    ///
    /// If `None`, all variables must have an explicit namespace
    #[must_use]
    fn default_namespace() -> Option<&'static str> {
        None
    }

    fn resolve_custom_namespace(namespace: &str, var: RawId) -> rootcause::Result<FormulaVariable> {
        let _ = var;
        bail!("Unsupported variable namespace: {}", namespace);
    }

    fn parse_variable(var: &str) -> rootcause::Result<FormulaVariable> {
        if !var.contains(':') {
            return Ok(FormulaVariable::Local(var.to_string()));
        }
        let (ns, var) = if let Some((namespace, var)) = var.split_once('@') {
            (namespace, var)
        } else {
            (
                Self::default_namespace()
                    .ok_or_else(|| report!("Namespace is required for ID variables"))?,
                var,
            )
        };

        let var_id = RawId::try_new(var)?;
        let var = match ns {
            "unit" => FormulaVariable::UnitVariable {
                id_ref: IdRef::new(var_id),
            },
            ns => return Self::resolve_custom_namespace(ns, var_id),
        };
        Self::validate_variable(&var)?;
        Ok(var)
    }
}

impl<C: FormulaModelContext> Debug for FormulaModel<C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FormulaModel::Expr(expr) => write!(f, "FormulaModel::Expr({})", expr.expr),
            FormulaModel::Const(value) => write!(f, "{}", value),
        }
    }
}

impl<C: FormulaModelContext> Clone for FormulaModel<C> {
    fn clone(&self) -> Self {
        match self {
            FormulaModel::Expr(expr) => FormulaModel::Expr(expr.clone()),
            FormulaModel::Const(value) => FormulaModel::Const(*value),
        }
    }
}

impl<C: FormulaModelContext> Debug for ExprWithArgs<C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ExprWithArgs{{ expr: {}, args: {:?} }}",
            self.expr, self.args
        )
    }
}

impl<C: FormulaModelContext> Clone for ExprWithArgs<C> {
    fn clone(&self) -> Self {
        Self {
            expr: self.expr.clone(),
            args: self.args.clone(),
            _c: PhantomData,
        }
    }
}

fn default_expr() -> Arc<exmex::FlatEx<f64>> {
    static DEFAULT_EXPR: LazyLock<Arc<exmex::FlatEx<f64>>> =
        LazyLock::new(|| Arc::new(exmex::FlatEx::parse("0").unwrap()));
    DEFAULT_EXPR.clone()
}

const _: () = {
    #[derive(Debug, Clone, schemars::JsonSchema)]
    #[serde(untagged)]
    #[allow(dead_code)]
    pub enum SerializedFormula {
        /// A formula string, e.g. "2 * x + 1"
        String(String),
        /// A constant number, e.g. 3.14
        Number(f64),
    }

    impl<C: FormulaModelContext> JsonSchema for FormulaModel<C> {
        fn schema_name() -> Cow<'static, str> {
            "Formula".into()
        }

        fn json_schema(generator: &mut SchemaGenerator) -> Schema {
            let mut schema = SerializedFormula::json_schema(generator);
            schema.insert(
                "description".to_owned(),
                serde_json::Value::String(C::description()),
            );
            schema
        }
    }

    impl<C: FormulaModelContext> Serialize for FormulaModel<C> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            match &self {
                FormulaModel::Expr(expr) => {
                    serializer.serialize_str(expr.expr.to_string().as_str())
                }
                FormulaModel::Const(value) => serializer.serialize_f64(*value),
            }
        }
    }

    impl<'de, C: FormulaModelContext> Deserialize<'de> for FormulaModel<C> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct FormulaModelVisitor<C>(PhantomData<fn() -> C>);
            impl<'de, C: FormulaModelContext> Visitor<'de> for FormulaModelVisitor<C> {
                type Value = FormulaModel<C>;

                fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                    write!(formatter, "formula string or a number")
                }

                fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    self.visit_f64(v as f64)
                }

                fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    Ok(FormulaModel::Const(v))
                }

                fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    let formula = exmex::parse::<f64>(v).map_err(|err| {
                        E::custom(format!("Failed to parse formula string: {err}"))
                    })?;

                    let mut args = Vec::new();
                    for var in formula.var_names() {
                        args.push(C::parse_variable(var).map_err(|err| {
                            E::custom(
                                err.context("Failed to parse variable in formula")
                                    .attach(AttachField("variable", var.to_string())),
                            )
                        })?);
                    }

                    Ok(FormulaModel::Expr(ExprWithArgs {
                        expr: Arc::new(formula),
                        args,
                        _c: PhantomData,
                    }))
                }
            }

            deserializer.deserialize_any(FormulaModelVisitor(Default::default()))
        }
    }
};
