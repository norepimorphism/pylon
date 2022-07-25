// SPDX-License-Identifier: MPL-2.0

//! A small 3D rendering library.

#![feature(portable_simd)]

pub mod linear;
pub mod renderer;
pub mod scene;

pub use linear::{Matrix, Vector};
pub use renderer::Renderer;
pub use scene::{Camera, Material, Mesh, Object, Point, Scene, MeshTriangle, MeshVertex};
