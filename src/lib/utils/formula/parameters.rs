use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use serde::{Deserialize, Serialize};

// TODO: &str
// TODO: Eq
pub trait ParameterId: Debug + Clone {
    fn get_name(&self) -> String;
}

pub trait ParameterIdAll: ParameterId {
    fn get_all() -> impl Iterator<Item = (String, Self)>;
}

pub trait Parameters<PId: ParameterId>: Debug {
    fn get_value(&self, id: &PId) -> f32;
}

impl ParameterId for usize {
    fn get_name(&self) -> String {
        char::from_u32('a' as u32 + *self as u32)
            .unwrap()
            .to_string()
    }
}

impl Parameters<usize> for [f32] {
    fn get_value(&self, id: &usize) -> f32 {
        self[*id]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ArrayIdx<const N: usize>(usize);

impl<const N: usize> Deref for ArrayIdx<N> {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const N: usize> DerefMut for ArrayIdx<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<const N: usize> ParameterId for ArrayIdx<N> {
    fn get_name(&self) -> String {
        char::from_u32('a' as u32 + self.0 as u32)
            .unwrap()
            .to_string()
    }
}

impl<const N: usize> ParameterIdAll for ArrayIdx<N> {
    fn get_all() -> impl Iterator<Item = (String, Self)> {
        (0..N).map(|idx| {
            (
                char::from_u32('a' as u32 + idx as u32).unwrap().to_string(),
                ArrayIdx(idx),
            )
        })
    }
}

impl<const N: usize> Parameters<ArrayIdx<N>> for [f32] {
    fn get_value(&self, id: &ArrayIdx<N>) -> f32 {
        self[**id]
    }
}

impl<const N: usize> Parameters<ArrayIdx<N>> for [f32; N] {
    fn get_value(&self, id: &ArrayIdx<N>) -> f32 {
        self[**id]
    }
}
