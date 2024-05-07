use core::fmt;
use std::fmt::{Display, Formatter};

pub enum MetricType {
    Gauge,
    Distribution,
    Counter,
    Set,
}

impl Display for MetricType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            MetricType::Gauge => write!(f, "g"),
            MetricType::Distribution => write!(f, "d"),
            MetricType::Counter => write!(f, "c"),
            MetricType::Set => write!(f, "s"),
        }
    }
}
