use crate::types::formula::FormulaModelArgs;
use bevy_reflect::TypePath;
use std::borrow::Cow;

#[derive(TypePath)]
pub struct EmptyArgs;

impl FormulaModelArgs for EmptyArgs {
    type Input = ();

    fn argument_names() -> Cow<'static, [String]> {
        Cow::Borrowed(&[])
    }

    fn from_input(_: Self::Input) -> Vec<f64> {
        Vec::new()
    }
}
