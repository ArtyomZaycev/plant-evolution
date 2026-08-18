use std::fmt::Debug;

pub trait Formula<P>: Debug + ToString {
    fn calculate(&self, parameters: &P) -> f32;
}
