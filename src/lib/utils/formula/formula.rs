use std::fmt::Debug;

pub trait Formula<P>: Debug + ToString + Clone {
    fn calculate(&self, parameters: &P) -> f32;
}
