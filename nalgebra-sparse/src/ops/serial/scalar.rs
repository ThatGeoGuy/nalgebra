//! Module holding the various sparse-matrix scalar operation functions.

use crate::cs::{Compression, CsMatrix};
use nalgebra::Scalar;
use std::{
    borrow::Borrow,
    ops::{Div, Mul},
};

/// Scalar product for sparse matrices.
///
/// This does not perform any checks to ensure that the Scalar is non-zero. This means that if zero
/// (or close to zero) is passed in, the resulting sparse matrix will store the final values as
/// explicit zeros.
pub fn sp_cs_scalar_prod<T1, T2, MO, MI, D, C>(
    cs: CsMatrix<T1, MO, MI, D, C>,
    scalar: T2,
) -> CsMatrix<<T1 as Mul<T2>>::Output, MO, MI, Vec<<T1 as Mul<T2>>::Output>, C>
where
    T1: Scalar + Mul<T2>,
    T2: Scalar,
    <T1 as Mul<T2>>::Output: Scalar,
    MO: Borrow<[usize]>,
    MI: Borrow<[usize]>,
    D: Borrow<[T1]>,
    C: Compression,
{
    let (rows, columns) = cs.shape();
    let (offsets, indices, data) = cs.disassemble();

    let data = data
        .borrow()
        .iter()
        .map(|x| x.clone() * scalar.clone())
        .collect();

    unsafe { CsMatrix::from_parts_unchecked(rows, columns, offsets, indices, data) }
}

/// Scalar division for sparse matrices.
///
/// This does not perform any checks to ensure that the division will result in non-zeros. This
/// means that if for example you have an explicit zero in the sparse-matrix, and you divide by
/// `1.0f32`, then the explicit zero will remain in the output.
pub fn sp_cs_scalar_div<T1, T2, MO, MI, D, C>(
    cs: CsMatrix<T1, MO, MI, D, C>,
    scalar: T2,
) -> CsMatrix<<T1 as Div<T2>>::Output, MO, MI, Vec<<T1 as Div<T2>>::Output>, C>
where
    T1: Scalar + Div<T2>,
    T2: Scalar,
    <T1 as Div<T2>>::Output: Scalar,
    MO: Borrow<[usize]>,
    MI: Borrow<[usize]>,
    D: Borrow<[T1]>,
    C: Compression,
{
    let (rows, columns) = cs.shape();
    let (offsets, indices, data) = cs.disassemble();

    let data = data
        .borrow()
        .iter()
        .map(|x| x.clone() / scalar.clone())
        .collect();

    unsafe { CsMatrix::from_parts_unchecked(rows, columns, offsets, indices, data) }
}
