//! Linear algebra definitions.

use std::{ops::{Add, Mul}, simd::Simd};

pub type Scalar = f32;

impl Matrix {
    pub const fn new(
        r0c0: Scalar,
        r0c1: Scalar,
        r0c2: Scalar,
        r0c3: Scalar,
        r1c0: Scalar,
        r1c1: Scalar,
        r1c2: Scalar,
        r1c3: Scalar,
        r2c0: Scalar,
        r2c1: Scalar,
        r2c2: Scalar,
        r2c3: Scalar,
        r3c0: Scalar,
        r3c1: Scalar,
        r3c2: Scalar,
        r3c3: Scalar,
    ) -> Self {
        Self([
            Vector::new(r0c0, r1c0, r2c0, r3c0),
            Vector::new(r0c1, r1c1, r2c1, r3c1),
            Vector::new(r0c2, r1c2, r2c2, r3c2),
            Vector::new(r0c3, r1c3, r2c3, r3c3),
        ])
    }
}

/// A 4x4 square matrix of `Scalar`s.
pub struct Matrix([Vector; 4]);

impl Matrix {
    pub const ZERO: Self = Self::new(
        0., 0., 0., 0.,
        0., 0., 0., 0.,
        0., 0., 0., 0.,
        0., 0., 0., 0.,
    );

    pub const IDENTITY: Self = Self::new(
        1., 0., 0., 0.,
        0., 1., 0., 0.,
        0., 0., 1., 0.,
        0., 0., 0., 1.,
    );
}

impl Add<Self> for Matrix {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let [a, b, c, d] = self.0;
        let [e, f, g, h] = rhs.0;

        Self([
            a + e,
            b + f,
            c + g,
            d + h,
        ])
    }
}

impl Mul<Scalar> for Matrix {
    type Output = Self;

    fn mul(self, rhs: Scalar) -> Self::Output {
        Self(self.0.map(|vector| vector * rhs))
    }
}

impl Vector {
    pub const fn new(r0: Scalar, r1: Scalar, r2: Scalar, r3: Scalar) -> Self {
        Self(Simd::from_array([r0, r1, r2, r3]))
    }
}

/// A 4x1 column matrix of `Scalar`s.
pub struct Vector(Simd<Scalar, 4>);

impl Vector {
    pub const ZERO: Self = Self::new(0., 0., 0., 0.);

    pub const fn to_array(&self) -> [Scalar; 4] {
        self.0.to_array()
    }
}

impl Add<Self> for Vector {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Mul<Scalar> for Vector {
    type Output = Self;

    fn mul(self, rhs: Scalar) -> Self::Output {
        Self(self.0 * Simd::splat(rhs))
    }
}

impl Mul<Vector> for Scalar {
    type Output = Vector;

    fn mul(self, rhs: Vector) -> Self::Output {
        // Multiplication with a scalar is commutative.
        rhs * self
    }
}
