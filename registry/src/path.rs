use std::fmt::{Debug, Formatter};

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum Segment {
    ListIndex(usize),
    TupleIndex(usize),
    SetEntry,
    MapKey,
    MapEntry(String),
    Field(String),
    EnumVariant(String),
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct FieldPath {
    root: String,
    segments: Vec<Segment>,
}

impl FieldPath {
    #[must_use] pub fn new(root: &str) -> Self {
        FieldPath {
            root: root.to_string(),
            segments: Vec::new(),
        }
    }

    pub fn with_segment<T>(&mut self, segment: Segment, cb: impl FnOnce(&mut FieldPath) -> T) -> T {
        self.segments.push(segment);
        let res = cb(self);
        self.segments.pop();
        res
    }

    #[must_use] pub fn format_path(&self) -> String {
        let mut path_str = self.root.clone();

        for s in &self.segments {
            match s {
                Segment::ListIndex(i) => {
                    path_str.push('[');
                    path_str.push_str(&i.to_string());
                    path_str.push(']');
                }
                Segment::TupleIndex(i) => {
                    path_str.push('.');
                    path_str.push_str(&i.to_string());
                }
                Segment::SetEntry => {
                    path_str.push_str(".<set entry>");
                }
                Segment::MapKey => {
                    path_str.push_str(".<map key>");
                }
                Segment::MapEntry(k) => {
                    path_str.push('[');
                    path_str.push_str(k);
                    path_str.push(']');
                }
                Segment::Field(field) => {
                    path_str.push('.');
                    path_str.push_str(field);
                }
                Segment::EnumVariant(variant) => {
                    path_str.push_str(".$");
                    path_str.push_str(variant);
                    path_str.push('$');
                }
            }
        }

        path_str
    }
}

impl Debug for FieldPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Path({})", self.format_path())
    }
}
