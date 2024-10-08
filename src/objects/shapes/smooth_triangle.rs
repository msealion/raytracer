use crate::collections::{Point, Vector};
use crate::objects::*;
use crate::utils::{Buildable, ConsumingBuilder, EPSILON};

#[derive(Debug)]
pub struct SmoothTriangle {
    frame_transformation: Transform,
    material: Material,
    vertices: [Point; 3],
    edges: [Vector; 2],
    normals: [Vector; 3],
    bounds: Bounds,
}

impl SmoothTriangle {
    // always unbounded
    const PRIMITIVE_BOUNDING_BOX: BoundingBox = BoundingBox::new_unbounded();

    pub fn vertices(&self) -> [Point; 3] {
        self.vertices
    }

    pub fn edges(&self) -> [Vector; 2] {
        self.edges
    }

    pub fn normals(&self) -> [Vector; 3] {
        self.normals
    }
}

impl PrimitiveShape for SmoothTriangle {
    fn frame_transformation(&self) -> &Transform {
        &self.frame_transformation
    }

    fn material(&self) -> &Material {
        &self.material
    }

    fn local_normal_at(&self, _local_point: Point, uv_coordinates: Option<(f64, f64)>) -> Vector {
        let [n1, n2, n3] = self.normals;
        let (u, v) = uv_coordinates.unwrap();
        (n2 * u + n3 * v + n1 * (1.0 - u - v)).normalise()
    }

    fn local_intersect(&self, local_ray: &Ray) -> Vec<Coordinates> {
        let dir_cross_e2 = local_ray.direction.cross(self.edges[1]);
        let det = self.edges[0].dot(dir_cross_e2);
        if det.abs() < EPSILON {
            return vec![];
        }

        let f = 1.0 / det;
        let p1_to_origin = local_ray.origin - self.vertices[0];
        let u = f * p1_to_origin.dot(dir_cross_e2);
        if u < 0.0 || u > 1.0 {
            return vec![];
        }

        let origin_cross_e1 = p1_to_origin.cross(self.edges[0]);
        let v = f * local_ray.direction.dot(origin_cross_e1);
        if v < 0.0 || (u + v) > 1.0 {
            return vec![];
        }

        let t = f * self.edges[1].dot(origin_cross_e1);
        vec![(t, Some((u, v)))]
            .iter()
            .map(|&(t, uv_coordinates)| Coordinates::new(t, uv_coordinates))
            .collect()
    }
}

impl Bounded for SmoothTriangle {
    fn bounds(&self) -> &Bounds {
        &self.bounds
    }
}

#[derive(Debug, Default)]
pub struct SmoothTriangleBuilder {
    frame_transformation: Option<Transform>,
    material: Option<Material>,
    vertices: Option<[Point; 3]>,
    normals: Option<[Vector; 3]>,
}

impl SmoothTriangleBuilder {
    pub fn set_frame_transformation(
        mut self,
        frame_transformation: Transform,
    ) -> SmoothTriangleBuilder {
        self.frame_transformation = Some(frame_transformation);
        self
    }

    pub fn set_material(mut self, material: Material) -> SmoothTriangleBuilder {
        self.material = Some(material);
        self
    }

    pub fn set_vertices(mut self, vertices: [Point; 3]) -> SmoothTriangleBuilder {
        self.vertices = Some(vertices);
        self
    }

    pub fn set_normals(mut self, normals: [Vector; 3]) -> SmoothTriangleBuilder {
        self.normals = Some(normals);
        self
    }
}

impl Buildable for SmoothTriangle {
    type Builder = SmoothTriangleBuilder;

    fn builder() -> Self::Builder {
        SmoothTriangleBuilder::default()
    }
}

impl ConsumingBuilder for SmoothTriangleBuilder {
    type Built = SmoothTriangle;

    fn build(self) -> Self::Built {
        let frame_transformation = self.frame_transformation.unwrap_or_default();
        let material = self.material.unwrap_or_default();
        let [v1, v2, v3] = self.vertices.unwrap();
        let normals = self.normals.unwrap();
        let e1 = v2 - v1;
        let e2 = v3 - v1;
        let bounds = Bounds::new(SmoothTriangle::PRIMITIVE_BOUNDING_BOX);
        let smooth_triangle = SmoothTriangle {
            frame_transformation,
            material,
            vertices: [v1, v2, v3],
            edges: [e1, e2],
            normals,
            bounds,
        };
        smooth_triangle
    }
}

impl Into<Shape> for SmoothTriangle {
    fn into(self) -> Shape {
        Shape::Primitive(Box::new(self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::approx_eq;

    #[test]
    fn intersection_collects_uv_coordinates() {
        let vertices = [
            Point::new(0.0, 1.0, 0.0),
            Point::new(-1.0, 0.0, 0.0),
            Point::new(1.0, 0.0, 0.0),
        ];
        let normals = [
            Vector::new(0.0, 1.0, 0.0),
            Vector::new(-1.0, 0.0, 0.0),
            Vector::new(1.0, 0.0, 0.0),
        ];
        let smooth_triangle = SmoothTriangle::builder()
            .set_vertices(vertices)
            .set_normals(normals)
            .build();
        let ray = Ray::new(Point::new(-0.2, 0.3, -2.0), Vector::new(0.0, 0.0, 1.0));
        let intersections = smooth_triangle.local_intersect(&ray);
        let (u, v) = intersections[0].uv_coordinates().unwrap();
        approx_eq!(u, 0.45);
        approx_eq!(v, 0.25);
    }

    #[test]
    fn smooth_triangle_interpolates_normals() {
        let vertices = [
            Point::new(0.0, 1.0, 0.0),
            Point::new(-1.0, 0.0, 0.0),
            Point::new(1.0, 0.0, 0.0),
        ];
        let normals = [
            Vector::new(0.0, 1.0, 0.0),
            Vector::new(-1.0, 0.0, 0.0),
            Vector::new(1.0, 0.0, 0.0),
        ];
        let smooth_triangle = SmoothTriangle::builder()
            .set_vertices(vertices)
            .set_normals(normals)
            .build();
        let normal = smooth_triangle.local_normal_at(Point::new(0.0, 0.0, 0.0), Some((0.45, 0.25)));
        let resulting_normal = Vector::new(-0.5547, 0.83205, 0.0);
        approx_eq!(normal.x, resulting_normal.x);
        approx_eq!(normal.y, resulting_normal.y);
        approx_eq!(normal.z, resulting_normal.z);
    }
}
