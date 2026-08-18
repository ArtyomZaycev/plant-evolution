use std::fmt::Debug;

pub trait ParameterId: Debug + Clone {
    fn get_name(&self) -> String;
}

pub trait Parameters<PId: ParameterId>: Debug {
    fn get_value(&self, id: &PId) -> f32;
}

impl ParameterId for usize {
    fn get_name(&self) -> String {
        self.to_string()
    }
}

impl Parameters<usize> for &[f32] {
    fn get_value(&self, id: &usize) -> f32 {
        self[*id]
    }
}
