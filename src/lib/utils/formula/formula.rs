use std::fmt::Debug;

pub trait Formula<P>: Debug + ToString {
    #[must_use]
    fn calculate(&self, parameters: &P) -> f32;
}
