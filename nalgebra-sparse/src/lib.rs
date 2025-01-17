//! Sparse matrices and algorithms for [nalgebra](https://www.nalgebra.org).
//!
//! This crate extends `nalgebra` with sparse matrix formats and operations on sparse matrices.
//!
//! ## Goals
//! The long-term goals for this crate are listed below.
//!
//! - Provide proven sparse matrix formats in an easy-to-use and idiomatic Rust API that
//!   naturally integrates with `nalgebra`.
//! - Provide additional expert-level APIs for fine-grained control over operations.
//! - Integrate well with external sparse matrix libraries.
//! - Provide native Rust high-performance routines, including parallel matrix operations.
//!
//! ## Highlighted current features
//!
//! - [CSR](cs::CsrMatrix), [CSC](cs::CscMatrix) and [COO](coo::CooMatrix) formats, and
//!   [conversions](`convert`) between them.
//! - Common arithmetic operations are implemented. See the [`ops`] module.
//! - Sparsity patterns in CSR and CSC matrices are explicitly represented by the
//!   [SparsityPattern](pattern::SparsityPattern) type, which encodes the invariants of the
//!   associated index data structures.
//! - [proptest strategies](`proptest`) for sparse matrices when the feature
//!   `proptest-support` is enabled.
//! - [matrixcompare support](https://crates.io/crates/matrixcompare) for effortless
//!   (approximate) comparison of matrices in test code (requires the `compare` feature).
//!
//! ## Current state
//!
//! The library is in an early, but usable state. The API has been designed to be extensible,
//! but breaking changes will be necessary to implement several planned features. While it is
//! backed by an extensive test suite, it has yet to be thoroughly battle-tested in real
//! applications. Moreover, the focus so far has been on correctness and API design, with little
//! focus on performance. Future improvements will include incremental performance enhancements.
//!
//! Current limitations:
//!
//! - Limited or no availability of sparse system solvers.
//! - Limited support for complex numbers. Currently only arithmetic operations that do not
//!   rely on particular properties of complex numbers, such as e.g. conjugation, are
//!   supported.
//! - No integration with external libraries.
//!
//! # Usage
//!
//! Add the following to your `Cargo.toml` file:
//!
//! ```toml
//! [dependencies]
//! nalgebra_sparse = "0.1"
//! ```
//!
//! # Supported matrix formats
//!
//! | Format                  | Notes                                        |
//! | ------------------------|--------------------------------------------- |
//! | [COO](`coo::CooMatrix`) | Well-suited for matrix construction. <br /> Ill-suited for algebraic operations. |
//! | [CSR](`cs::CsrMatrix`) | Immutable sparsity pattern, suitable for algebraic operations. <br /> Fast row access. |
//! | [CSC](`cs::CscMatrix`) | Immutable sparsity pattern, suitable for algebraic operations. <br /> Fast column access. |
//!
//! What format is best to use depends on the application. The most common use case for sparse
//! matrices in science is the solution of sparse linear systems. Here we can differentiate between
//! two common cases:
//!
//! - Direct solvers. Typically, direct solvers take their input in CSR or CSC format.
//! - Iterative solvers. Many iterative solvers require only matrix-vector products,
//!   for which the CSR or CSC formats are suitable.
//!
//! The [COO](coo::CooMatrix) format is primarily intended for matrix construction.
//! A common pattern is to use COO for construction, before converting to CSR or CSC for use
//! in a direct solver or for computing matrix-vector products in an iterative solver.
//! Some high-performance applications might also directly manipulate the CSR and/or CSC
//! formats.
//!
//! # Example: COO -> CSR -> matrix-vector product
//!
//! ```
//! use nalgebra_sparse::{coo::CooMatrix, cs::CsrMatrix};
//! use nalgebra::{DMatrix, DVector};
//! use matrixcompare::assert_matrix_eq;
//!
//! // The dense representation of the matrix
//! let dense = DMatrix::from_row_slice(3, 3,
//!     &[1.0, 0.0, 3.0,
//!       2.0, 0.0, 1.3,
//!       0.0, 0.0, 4.1]);
//!
//! // Build the equivalent COO representation. We only add the non-zero values
//! let mut coo = CooMatrix::new(3, 3);
//!
//! // We can add elements in any order. For clarity, we do so in row-major order here.
//! coo.push(0, 0, 1.0);
//! coo.push(0, 2, 3.0);
//! coo.push(1, 0, 2.0);
//! coo.push(1, 2, 1.3);
//! coo.push(2, 2, 4.1);
//!
//! // ... or add entire dense matrices like so:
//! // coo.push_matrix(0, 0, &dense);
//!
//! // The simplest way to construct a CSR matrix is to first construct a COO matrix, and
//! // then convert it to CSR. The `From` trait is implemented for conversions between different
//! // sparse matrix types.
//! // Alternatively, we can construct a matrix directly from the CSR data.
//! // See the docs for CsrMatrix for how to do that.
//! let csr = CsrMatrix::from(coo);
//!
//! // Let's check that the CSR matrix and the dense matrix represent the same matrix.
//! // We can use macros from the `matrixcompare` crate to easily do this, despite the fact that
//! // we're comparing across two different matrix formats. Note that these macros are only really
//! // appropriate for writing tests, however.
//! assert_matrix_eq!(csr, dense);
//!
//! let x = DVector::from_column_slice(&[1.3, -4.0, 3.5]);
//!
//! // Compute the matrix-vector product y = A * x. We don't need to specify the type here,
//! // but let's just do it to make sure we get what we expect
//! let y = csr * x;
//!
//! // Verify the result with a small element-wise absolute tolerance
//! let y_expected = DVector::from_column_slice(&[11.8, 7.15, 14.35]);
//!
//! assert_matrix_eq!(y, y_expected, comp = abs, tol = 1e-9);
//! ```
#![deny(
    nonstandard_style,
    unused,
    missing_docs,
    rust_2018_idioms,
    rust_2018_compatibility,
    // future_incompatible,
    missing_copy_implementations
)]

pub extern crate nalgebra as na;
pub mod convert;
pub mod coo;
pub mod cs;
pub mod error;
pub mod factorization;
pub mod ops;

#[cfg(feature = "proptest-support")]
pub mod proptest;

#[cfg(feature = "compare")]
mod matrixcompare;

use num_traits::Zero;

pub use self::coo::CooMatrix;

/// An entry in a sparse matrix.
///
/// Sparse matrices do not store all their entries explicitly. Therefore, entry (i, j) in the matrix
/// can either be a reference to an explicitly stored element, or it is implicitly zero.
#[derive(Debug, PartialEq, Eq)]
pub enum SparseEntry<'a, T> {
    /// The entry is a reference to an explicitly stored element.
    ///
    /// Note that the naming here is a misnomer: The element can still be zero, even though it
    /// is explicitly stored (a so-called "explicit zero").
    NonZero(&'a T),
    /// The entry is implicitly zero, i.e. it is not explicitly stored.
    Zero,
}

impl<'a, T: Clone + Zero> SparseEntry<'a, T> {
    /// Returns the value represented by this entry.
    ///
    /// Either clones the underlying reference or returns zero if the entry is not explicitly
    /// stored.
    pub fn into_value(self) -> T {
        match self {
            SparseEntry::NonZero(value) => value.clone(),
            SparseEntry::Zero => T::zero(),
        }
    }
}

/// A mutable entry in a sparse matrix.
///
/// See also `SparseEntry`.
#[derive(Debug, PartialEq, Eq)]
pub enum SparseEntryMut<'a, T> {
    /// The entry is a mutable reference to an explicitly stored element.
    ///
    /// Note that the naming here is a misnomer: The element can still be zero, even though it
    /// is explicitly stored (a so-called "explicit zero").
    NonZero(&'a mut T),
    /// The entry is implicitly zero i.e. it is not explicitly stored.
    Zero,
}

impl<'a, T: Clone + Zero> SparseEntryMut<'a, T> {
    /// Returns the value represented by this entry.
    ///
    /// Either clones the underlying reference or returns zero if the entry is not explicitly
    /// stored.
    pub fn into_value(self) -> T {
        match self {
            SparseEntryMut::NonZero(value) => value.clone(),
            SparseEntryMut::Zero => T::zero(),
        }
    }
}
