//! Linear algebra definitions.

use std::{ops::{Add, AddAssign, Mul, MulAssign}, simd::Simd};

/// The backing storage unit of [matrices](Matrix) and [vectors](Vector).
pub type Scalar = f32;

impl Matrix {
    /// Creates a new `Matrix` with the given 16 elements provided in left-to-right, top-to-bottom
    /// order.
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

/// A 4x4 square matrix of [`Scalar`](Scalar)s.
#[derive(Clone, Copy, Debug)]
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

    pub fn columns(&self) -> &[Vector; 4] {
        &self.0
    }

    pub fn columns_mut(&mut self) -> &mut [Vector; 4] {
        &mut self.0
    }

    pub fn as_rows(&self) -> [Vector; 4] {
        let cols = self.to_array();

        [
            Vector::new(cols[0][0], cols[1][0], cols[2][0], cols[3][0]),
            Vector::new(cols[0][1], cols[1][1], cols[2][1], cols[3][1]),
            Vector::new(cols[0][2], cols[1][2], cols[2][2], cols[3][2]),
            Vector::new(cols[0][3], cols[1][3], cols[2][3], cols[3][3]),
        ]
    }

    pub fn to_array(&self) -> [[Scalar; 4]; 4] {
        self.0.map(|v| v.to_array())
    }
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

impl Mul<Matrix> for Scalar {
    type Output = Matrix;

    fn mul(self, rhs: Matrix) -> Self::Output {
        // Multiplication with a matrix is commutative.
        rhs * self
    }
}

impl Mul<Self> for Matrix {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        let rows = self.as_rows();
        let cols = rhs.columns();
        let elem = |a: usize, b: usize| (rows[a] * cols[b]).sum();

        Self::new(
            elem(0, 0),
            elem(0, 1),
            elem(0, 2),
            elem(0, 3),
            elem(1, 0),
            elem(1, 1),
            elem(1, 2),
            elem(1, 3),
            elem(2, 0),
            elem(2, 1),
            elem(2, 2),
            elem(2, 3),
            elem(3, 0),
            elem(3, 1),
            elem(3, 2),
            elem(3, 3),
        )
    }
}

impl Vector {
    pub const fn new(r0: Scalar, r1: Scalar, r2: Scalar, r3: Scalar) -> Self {
        Self(Simd::from_array([r0, r1, r2, r3]))
    }
}

/// A 4x1 column matrix of [`Scalar`](Scalar)s.
#[derive(Clone, Copy, Debug)]
pub struct Vector(Simd<Scalar, 4>);

impl Vector {
    pub const ZERO: Self = Self::new(0., 0., 0., 0.);

    pub fn sum(&self) -> Scalar {
        self.0.reduce_sum()
    }

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

impl AddAssign for Vector {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
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

impl Mul<Self> for Vector {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}
