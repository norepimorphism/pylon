// SPDX-License-Identifier: MPL-2.0

//! A small 3D rendering library.
//!
//! # Coordinate Spaces
//!
//! There are four coordinate spaces: **mesh** space, **world** space, **camera** space, and
//! **clip** space.
//!
//! ## Mesh Space
//!
//! Each mesh has an associated mesh space where the origin is considered the 'center' of the mesh.
//! Meshes are scaled and rotated about this origin.
//!
//! Mesh space is unbounded in all axes.
//!
//! ## World Space
//!
//! During object rendering, the coordinates of each object's mesh must be rebased to world space.
//! This is done through matrix addition of the object's world position and each mesh vertex.
//!
//! World space is unbounded in all axes.
//!
//! ## Camera Space
//!
//! This is the fun part: once all objects are in world space, we transform *the world itself* such
//! that the GPU only renders what the camera sees. Here, it is important to make a distinction
//! between the terms 'camera' and 'viewport'; the camera is managed by Pylon and may be positioned
//! arbitrarily in world space, but the viewport is a fixed region produced by compressing the GPU's
//! 3D clip space into a 2D rectangle. As the viewport is immovable, in order to render the scene as
//! seen through Pylon's camera, we must translate, scale, and rotate the vertices of each and every
//! object as dictated by the camera.
//!
//! Camera space is unbounded in all axes.
//!
//! ## Clip Space
//!
//! Clip space is the final destination for vertices and is produced by constraining camera space to
//! the range `[-1, 1]` in all axes. During rasterization, clip space is compressed into a 2D
//! viewport.

#![feature(portable_simd)]

pub mod linear;
pub mod renderer;

pub use linear::{Matrix, Vector};
pub use renderer::Renderer;

/// The integral type for indexing a mesh's [vertex pool](Mesh::vertex_pool).
pub type MeshVertexIndex = u32;

/// A set of objects and a camera that observes them.
///
/// A scene may be rendered with [`Renderer::render`].
#[derive(Clone, Debug)]
pub struct Scene {
    /// The camera through which objects are observed.
    pub camera: Camera,
    /// The objects.
    pub objects: Vec<Object>,
}

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    /// The location of this camera in world space.
    pub position: Point,
    /// A point, in world space, that this camera is 'looking at'.
    ///
    /// This *target point* determines the pitch and yaw of this camera, so-to-speak. This point
    /// must not be equivalent to [the position](Self::position).
    pub target: Point,
    /// The current roll angle in radians.
    pub roll: f32,
}

impl From<Vector> for Point {
    fn from(v: Vector) -> Self {
        let [x, y, z, _] = v.to_array();

        Self { x, y, z }
    }
}

/// A singular location within a coordinate space.
///
/// The `Point` type definition does not prescribe a particular coordinate space to constrain its
/// coordinates by; rather, the coordinates of a `Point` shall be interpreted by context. For
/// example, in the context of clip space, all coordinates within a `Point` must lie between -1 and
/// 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    /// The X coordinate.
    pub x: f32,
    /// The Y coordinate.
    pub y: f32,
    /// The Z coordinate.
    pub z: f32,
}

unsafe impl bytemuck::Pod for Point {}
unsafe impl bytemuck::Zeroable for Point {}

impl Point {
    /// The point that lies at `(0, 0, 0)`.
    pub const ORIGIN: Self = Self { x: 0., y: 0., z: 0. };
}

impl From<Point> for Vector {
    fn from(p: Point) -> Self {
        Self::new(p.x, p.y, p.z, 0.)
    }
}

/// A [mesh](Mesh), with a [material](Material) applied, within a scene.
#[derive(Clone, Debug)]
pub struct Object {
    /// The position of [the mesh](Self::mesh) in world space.
    pub position: Point,
    /// The scale factor of this object's mesh.
    ///
    /// A scale factor of 1 represents the original mesh size.
    pub scale: f32,
    /// The mesh.
    pub mesh: Mesh,
    /// The material applied to [the mesh](Self::mesh).
    pub material: Material,
}

#[derive(Clone, Debug)]
pub struct Mesh {
    /// The vertices that make up this mesh.
    pub vertex_pool: Vec<MeshVertex>,
    /// Triads of vertices from [`Self::vertex_pool`] that define the triangle primitives of this
    /// mesh.
    pub triangles: Vec<MeshTriangle>
}

impl MeshTriangle {
    /// Creates a new `MeshTriangle` from a triad of vertex indices.
    pub fn new(indices: [MeshVertexIndex; 3]) -> Self {
        Self(indices)
    }
}

/// A triangle within a [mesh](Mesh).
#[derive(Clone, Copy, Debug)]
pub struct MeshTriangle([MeshVertexIndex; 3]);

unsafe impl bytemuck::Pod for MeshTriangle {}
unsafe impl bytemuck::Zeroable for MeshTriangle {}

/// A vertex within a [mesh](Mesh).
#[derive(Clone, Copy, Debug)]
pub struct MeshVertex {
    /// The location of this vertex in mesh space.
    pub point: Point,
}

unsafe impl bytemuck::Pod for MeshVertex {}
unsafe impl bytemuck::Zeroable for MeshVertex {}

#[derive(Clone, Debug)]
pub struct Material;
