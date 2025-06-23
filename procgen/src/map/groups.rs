use noise::NoiseFn;

mod continents;

pub struct Group {
    label: String,
    sub_groups: Vec<Box<dyn SubGroup>>,
}

pub trait SubGroup {
    fn label(&self) -> &'static str;
    // fn define(&self) -> impl NoiseFn<f64, 3>;
}
