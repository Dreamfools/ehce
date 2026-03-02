use std::fmt::{Debug, Display, Formatter};

pub struct AttachField<A: 'static + Send + Sync + Display + Debug>(pub &'static str, pub A);

impl<A: 'static + Send + Sync + Display + Debug> Display for AttachField<A> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.0, self.1)
    }
}

impl<A: 'static + Send + Sync + Display + Debug> Debug for AttachField<A> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AttachField")
            .field("name", &self.0)
            .field("value", &self.1)
            .finish()
    }
}
