use std::ops::Index;

use crate::collections::{Colour, Point, Vector};
use crate::objects::{RefractionBoundary, Shape};
use crate::utils::floats::EPSILON;

use super::Light;
use super::Ray;

#[derive(Clone, Debug)]
pub struct RawIntersect<'a, S>
where
    S: Shape + ?Sized,
{
    pub t: f64,
    pub object: &'a S,
    pub ray: &'a Ray,
    pub uv_coordinates: Option<(f64, f64)>,
}

impl<'a, S> RawIntersect<'a, S>
where
    S: Shape + ?Sized,
{
    pub fn new(
        t: f64,
        object: &'a S,
        ray: &'a Ray,
        uv_coordinates: Option<(f64, f64)>,
    ) -> RawIntersect<'a, S> {
        RawIntersect {
            t,
            object,
            ray,
            uv_coordinates,
        }
    }

    pub fn precompute(&self) -> ComputedIntersect<'_, S> {
        let t = self.t;
        let object = self.object;
        let ray = self.ray;
        let uv_coordinates = self.uv_coordinates;
        let target = self.ray.position(t);
        let eyev = -self.ray.direction;
        let mut normal = object.normal_at(target, uv_coordinates);
        let inside = match normal.dot(eyev) {
            _x if _x < 0.0 => {
                normal = -normal;
                true
            }
            _x if _x >= 0.0 => false,
            _ => panic!(),
        };
        let over_point = target + normal * EPSILON;
        let under_point = target - normal * EPSILON;
        let reflected_ray = Ray::new(over_point, ray.direction.reflect(normal));
        ComputedIntersect {
            t,
            object,
            ray,
            uv_coordinates,
            target,
            eyev,
            normal,
            inside,
            over_point,
            under_point,
            reflected_ray,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ComputedIntersect<'a, S>
where
    S: Shape + ?Sized,
{
    pub t: f64,
    pub object: &'a S,
    pub ray: &'a Ray,
    pub uv_coordinates: Option<(f64, f64)>,

    pub target: Point,
    pub eyev: Vector,
    pub normal: Vector,
    pub inside: bool,
    pub over_point: Point,
    pub under_point: Point,
    pub reflected_ray: Ray,
}

impl<S> ComputedIntersect<'_, S>
where
    S: Shape + ?Sized,
{
    pub fn shade(&self, light: &Light, shadowed: bool) -> Colour {
        light.shade_phong(
            self.object.material(),
            self.over_point,
            self.eyev,
            self.normal,
            shadowed,
        )
    }

    pub fn schlick_reflectance(&self, n1: f64, n2: f64) -> f64 {
        let mut cos = self.eyev.dot(self.normal);

        if n1 > n2 {
            let n = n1 / n2;
            let sin2_t = n.powi(2) * (1.0 - cos.powi(2));
            if sin2_t > 1.0 {
                return 1.0;
            }

            let cos_t = (1.0 - sin2_t).sqrt();

            cos = cos_t
        }

        let r0 = ((n1 - n2) / (n1 + n2)).powi(2);
        r0 + (1.0 - r0) * (1.0 - cos).powi(5)
    }
}

#[derive(Clone, Debug)]
pub struct Intersections<'a, S>(pub Vec<RawIntersect<'a, S>>)
where
    S: Shape + ?Sized;

impl<'a, S> Intersections<'a, S>
where
    S: Shape + ?Sized,
{
    pub fn new(mut vec: Vec<RawIntersect<'_, S>>) -> Intersections<'_, S> {
        assert_ne!(vec.len(), 0);
        vec.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap());
        Intersections(vec)
    }

    pub fn add_raw_intersect(&mut self, intersect: RawIntersect<'a, S>) {
        for (i, v) in self.0.iter_mut().enumerate() {
            if intersect.t < v.t {
                self.0.insert(i, intersect);
                return;
            }
        }
        self.0.push(intersect);
    }

    pub fn combine_intersections(&mut self, intersections: Intersections<'a, S>) {
        for intersect in intersections.0 {
            self.add_raw_intersect(intersect);
        }
    }

    pub fn hit(&self) -> Option<&RawIntersect<'_, S>> {
        self.0.iter().find(|&v| v.t >= 0.0)
    }

    pub fn compute_refraction_boundaries(&self) -> Vec<RefractionBoundary>
    where
        S: PartialEq,
    {
        let mut in_objects: Vec<&S> = vec![];
        let mut refraction_boundaries = vec![];

        for raw_intersect in &self.0 {
            let n1 = match in_objects.last() {
                Some(last_object) => last_object.material().refractive_index,
                None => 1.0,
            };

            match in_objects
                .iter()
                .position(|&object| object == raw_intersect.object)
            {
                Some(idx_object) => {
                    in_objects.remove(idx_object);
                }
                None => {
                    in_objects.push(raw_intersect.object);
                }
            };

            let n2 = match in_objects.last() {
                Some(last_object) => last_object.material().refractive_index,
                None => 1.0,
            };

            refraction_boundaries.push(RefractionBoundary { n1, n2 });
        }

        refraction_boundaries
    }
}

impl<'a, S> From<Vec<RawIntersect<'a, S>>> for Intersections<'a, S>
where
    S: Shape + ?Sized,
{
    fn from(value: Vec<RawIntersect<'a, S>>) -> Intersections<'a, S> {
        Intersections::new(value)
    }
}

impl<'a, S> Default for Intersections<'a, S>
where
    S: Shape + ?Sized,
{
    fn default() -> Intersections<'a, S> {
        Intersections(vec![])
    }
}

impl<'a, S> Index<usize> for Intersections<'a, S>
where
    S: Shape + ?Sized,
{
    type Output = RawIntersect<'a, S>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

#[cfg(test)]
mod tests {
    use crate::objects::{Material, Plane, Sphere, Transform, TransformKind};
    use crate::scenes::World;
    use crate::utils::Preset;

    use super::*;

    #[test]
    fn create_raw_intersect() {
        let sphere = Sphere::default();
        let ray = Ray::new(Point::new(0.0, 0.0, 0.0), Vector::new(0.0, 1.0, 0.0));
        let raw_intersect = RawIntersect::new(1.0, &sphere, &ray, None);
        let resulting_intersect = RawIntersect {
            t: 1.0,
            object: &sphere,
            ray: &ray,
            uv_coordinates: None,
        };
        assert_eq!(raw_intersect.t, resulting_intersect.t);
        assert!(std::ptr::eq(
            raw_intersect.object,
            resulting_intersect.object,
        ));
        assert!(std::ptr::eq(raw_intersect.ray, resulting_intersect.ray));
    }

    #[test]
    fn compute_intersect_ray_outside() {
        let ray = Ray::new(Point::new(0.0, 0.0, -5.0), Vector::new(0.0, 0.0, 1.0));
        let shape = Sphere::default();
        let raw_intersect = RawIntersect::new(4.0, &shape, &ray, None);
        let computed_intersect = raw_intersect.precompute();
        assert_eq!(computed_intersect.target, Point::new(0.0, 0.0, -1.0));
        assert_eq!(computed_intersect.eyev, Vector::new(0.0, 0.0, -1.0));
        assert_eq!(computed_intersect.normal, Vector::new(0.0, 0.0, -1.0));
        assert_eq!(
            computed_intersect.over_point,
            Point::new(0.0, 0.0, -1.0) + Vector::new(0.0, 0.0, -1.0) * EPSILON
        );
    }

    #[test]
    fn compute_intersect_ray_inside() {
        let ray = Ray::new(Point::new(0.0, 0.0, 0.0), Vector::new(0.0, 0.0, 1.0));
        let shape = Sphere::default();
        let raw_intersect = RawIntersect::new(1.0, &shape, &ray, None);
        let computed_intersect = raw_intersect.precompute();
        assert_eq!(computed_intersect.target, Point::new(0.0, 0.0, 1.0));
        assert_eq!(computed_intersect.eyev, Vector::new(0.0, 0.0, -1.0));
        assert!(computed_intersect.inside);
        assert_eq!(computed_intersect.normal, Vector::new(0.0, 0.0, -1.0));
    }

    #[test]
    fn intersections_hit() {
        let sphere = Sphere::default();
        let ray = Ray::new(Point::new(0.0, 0.0, 0.0), Vector::new(0.0, 1.0, 0.0));
        let intersect1 = RawIntersect::new(-1.0, &sphere, &ray, None);
        let intersect2 = RawIntersect::new(2.0, &sphere, &ray, None);
        let intersect3 = RawIntersect::new(3.0, &sphere, &ray, None);
        let intersections = Intersections::new(vec![intersect1, intersect2, intersect3]);
        let resulting_hit = &intersections.0[1];
        assert!(std::ptr::eq(intersections.hit().unwrap(), resulting_hit));
    }

    #[test]
    fn hit_offset_point() {
        let ray = Ray::new(Point::new(0.0, 0.0, -5.0), Vector::new(0.0, 0.0, 1.0));
        let shape = Sphere::new(
            Transform::new(TransformKind::Translate(0.0, 0.0, 1.0)),
            Material::preset(),
        );
        let raw_intersect = RawIntersect::new(5.0, &shape, &ray, None);
        let computed_intersect = raw_intersect.precompute();
        assert!(computed_intersect.over_point.z < -EPSILON / 2.0);
        assert!(computed_intersect.target.z > computed_intersect.over_point.z);
        assert!(computed_intersect.under_point.z > -EPSILON / 2.0);
        assert!(computed_intersect.target.z < computed_intersect.under_point.z);
    }

    #[test]
    fn precompute_reflection_vector() {
        let plane = Plane::preset();
        let ray = Ray::new(
            Point::new(0.0, 1.0, -1.0),
            Vector::new(0.0, -(2.0_f64.sqrt()) / 2.0, 2.0_f64.sqrt() / 2.0),
        );
        let raw_intersect = RawIntersect::new(2.0_f64.sqrt() / 2.0, &plane, &ray, None);
        let computed_intersect = raw_intersect.precompute();
        assert_eq!(
            computed_intersect.reflected_ray.direction,
            Vector::new(0.0, 2.0_f64.sqrt() / 2.0, 2.0_f64.sqrt() / 2.0)
        );
    }
}
