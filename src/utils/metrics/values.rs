use core::fmt;
use std::fmt::{Display, Formatter};

pub enum MetricValue {
    Int(i64),
    Float(f64),
}

impl Display for MetricValue {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            MetricValue::Int(value) => write!(f, "{value}"),
            MetricValue::Float(value) => write!(f, "{value}"),
        }
    }
}
