use crate::numbers::validated::ValueValidator;
use schemars::Schema;
use std::borrow::Cow;
use variadics_please::all_tuples_enumerated;

impl<T> ValueValidator<T> for () {
    fn validate(_value: &T) -> Result<(), String> {
        Ok(())
    }

    fn debug() -> Cow<'static, str> {
        "()".into()
    }
}

macro_rules! impl_validator {
    ($(($n:tt, $V:ident)),*) => {
        impl <T, $($V: ValueValidator<T>),*> ValueValidator<T> for ($($V,)*) {
            fn validate(value: &T) -> Result<(), String> {
                $(
                    $V::validate(value)?;
                )*
                Ok(())
            }

            fn debug() -> Cow<'static, str> {
                let mut s = String::from("(");
                $(
                    if $n != 0 {
                        s.push_str(", ");
                    }
                    s.push_str(&$V::debug());
                )*
                s.push_str(")");
                s.into()
            }

            fn modify_schema(schema: &mut Schema) {
                $(
                    $V::modify_schema(schema);
                )*
            }

            fn schema_name() -> Cow<'static, str> {
                let mut s = String::from("(");
                $(
                    if $n != 0 {
                        s.push_str(", ");
                    }
                    s.push_str(&$V::schema_name());
                )*
                s.push_str(")");
                s.into()
            }
        }
    };
}

all_tuples_enumerated!(impl_validator, 1, 12, V);
