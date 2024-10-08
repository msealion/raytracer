use std::fmt::Debug;

use crate::collections::{Colour, Point};
use crate::objects::{Transform, Transformable};

pub trait Pattern: Debug {
    fn colour_at(&self, shape_point: Point) -> Colour {
        let pattern_point = shape_point.transform(&self.frame_transformation().invert());
        self.local_colour_at(pattern_point)
    }

    fn frame_transformation(&self) -> &Transform;
    fn local_colour_at(&self, pattern_point: Point) -> Colour;
}

impl PartialEq for dyn Pattern {
    fn eq(&self, other: &Self) -> bool {
        format!("{:?}", self) == format!("{:?}", other)
    }
}
