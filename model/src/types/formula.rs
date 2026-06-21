use crate::registries::variable::UnitVariableModel;
use crate::types::formula::formula_executor::FormulaExecutor;
use bevy_reflect::erased_serde::__private::serde::de::Error;
use bevy_reflect::{Reflect, TypePath};
use exmex::Express as _;
use itertools::Itertools as _;
use registry::registry::id::{IdRef, RawId};
use rootcause::{bail, report};
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::sync::{Arc, LazyLock, OnceLock};
use utils::map::HashMap;
use utils::rootcause_ext::AttachField;

pub mod formula_context;

pub mod formula_args;
pub mod formula_executor;

#[derive(Reflect)]
pub enum FormulaModel<ARGS: FormulaModelArgs, CTX: FormulaModelContext<ARGS>> {
    Expr(Arc<ExprWithArgs<ARGS, CTX>>),
    Const(f64),
}

impl<ARGS: FormulaModelArgs, CTX: FormulaModelContext<ARGS>> FormulaModel<ARGS, CTX> {
    pub fn eval<EXEC: FormulaExecutor<ARGS, CTX>>(
        &self,
        executor: &EXEC,
        args: ARGS::Input,
    ) -> rootcause::Result<f64> {
        executor.execute_formula(self, args)
    }

    pub fn eval_f32<EXEC: FormulaExecutor<ARGS, CTX>>(
        &self,
        executor: &EXEC,
        args: ARGS::Input,
    ) -> rootcause::Result<f32> {
        executor.execute_formula(self, args).map(|x| x as f32)
    }
}

#[derive(Reflect)]
#[reflect(Clone)]
pub struct ExprWithArgs<ARGS: FormulaModelArgs, CTX: FormulaModelContext<ARGS>> {
    #[reflect(ignore, default = "default_expr")]
    pub expr: exmex::FlatEx<f64>,
    pub args: Vec<FormulaVariable>,
    #[reflect(ignore)]
    _c: PhantomData<fn() -> (ARGS, CTX)>,
}

#[derive(Debug, Clone, Reflect)]
pub enum FormulaVariable {
    UnitVariable { id_ref: IdRef<UnitVariableModel> },
    Local(String),
}

pub trait FormulaModelContext<Args: FormulaModelArgs>: TypePath {
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
        let input_args = Args::argument_names();
        if !var.contains(':') {
            if input_args.iter().any(|arg| arg == var) {
                return Ok(FormulaVariable::Local(var.to_string()));
            } else {
                bail!(
                    "Variable '{}' does not match any argument name. Allowed argument names are {}",
                    var,
                    input_args.iter().map(|s| format!("`{}`", s)).join(", ")
                );
            }
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

pub trait FormulaModelArgs: TypePath {
    /// Input type for providing arguments to the formula
    type Input;

    /// Names of the arguments
    fn argument_names() -> Cow<'static, [String]>;

    /// Converts the input into an iterator of variable names and their values
    ///
    /// The order of the values must correspond to the order of the argument names
    fn from_input(input: Self::Input) -> Vec<f64>;

    /// A mapping from argument names to their indices in the input vector
    fn arguments_indices() -> &'static HashMap<String, usize> {
        static ARG_INDICES: OnceLock<HashMap<String, usize>> = OnceLock::new();
        ARG_INDICES.get_or_init(|| {
            Self::argument_names()
                .iter()
                .enumerate()
                .map(|(idx, name)| (name.clone(), idx))
                .collect()
        })
    }
}

impl<ARGS: FormulaModelArgs, CTX: FormulaModelContext<ARGS>> Debug for FormulaModel<ARGS, CTX> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FormulaModel::Expr(expr) => write!(f, "FormulaModel::Expr({})", expr.expr),
            FormulaModel::Const(value) => write!(f, "{}", value),
        }
    }
}

impl<ARGS: FormulaModelArgs, CTX: FormulaModelContext<ARGS>> Clone for FormulaModel<ARGS, CTX> {
    fn clone(&self) -> Self {
        match self {
            FormulaModel::Expr(expr) => FormulaModel::Expr(expr.clone()),
            FormulaModel::Const(value) => FormulaModel::Const(*value),
        }
    }
}

impl<ARGS: FormulaModelArgs, CTX: FormulaModelContext<ARGS>> Debug for ExprWithArgs<ARGS, CTX> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ExprWithArgs{{ expr: {}, args: {:?} }}",
            self.expr, self.args
        )
    }
}

impl<ARGS: FormulaModelArgs, CTX: FormulaModelContext<ARGS>> Clone for ExprWithArgs<ARGS, CTX> {
    fn clone(&self) -> Self {
        Self {
            expr: self.expr.clone(),
            args: self.args.clone(),
            _c: PhantomData,
        }
    }
}

fn default_expr() -> exmex::FlatEx<f64> {
    static DEFAULT_EXPR: LazyLock<exmex::FlatEx<f64>> =
        LazyLock::new(|| exmex::FlatEx::parse("0").unwrap());
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

    impl<ARGS: FormulaModelArgs, CTX: FormulaModelContext<ARGS>> JsonSchema
        for FormulaModel<ARGS, CTX>
    {
        fn schema_name() -> Cow<'static, str> {
            "Formula".into()
        }

        fn json_schema(generator: &mut SchemaGenerator) -> Schema {
            let mut schema = SerializedFormula::json_schema(generator);
            schema.insert(
                "description".to_owned(),
                serde_json::Value::String(CTX::description()),
            );
            schema
        }
    }

    impl<ARGS: FormulaModelArgs, CTX: FormulaModelContext<ARGS>> Serialize for FormulaModel<ARGS, CTX> {
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

    impl<'de, ARGS: FormulaModelArgs, CTX: FormulaModelContext<ARGS>> Deserialize<'de>
        for FormulaModel<ARGS, CTX>
    {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct FormulaModelVisitor<ARGS, C>(PhantomData<fn() -> (ARGS, C)>);
            impl<'de, ARGS: FormulaModelArgs, CTX: FormulaModelContext<ARGS>> Visitor<'de>
                for FormulaModelVisitor<ARGS, CTX>
            {
                type Value = FormulaModel<ARGS, CTX>;

                fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                    write!(formatter, "formula string or a number")
                }

                fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    self.visit_f64(v as f64)
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
                        args.push(CTX::parse_variable(var).map_err(|err| {
                            E::custom(
                                err.context("Failed to parse variable in formula")
                                    .attach(AttachField("variable", var.to_string())),
                            )
                        })?);
                    }

                    Ok(FormulaModel::Expr(Arc::new(ExprWithArgs {
                        expr: formula,
                        args,
                        _c: PhantomData,
                    })))
                }
            }

            deserializer.deserialize_any(FormulaModelVisitor(Default::default()))
        }
    }
};
