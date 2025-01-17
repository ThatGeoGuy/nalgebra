//! Module holding the various sparse-matrix subtraction functions.
//!
//! Sparse-matrix subtraction performance can somewhat depend on the underlying data types. In
//! particular, adding two matrices with the same compression kind will tend to perform better than
//! adding two matrices of opposing (row vs. column major) storage layouts.
//!
//! # Performance
//!
//! Because of the above, the contained `spsub_x_y` functions may have slightly different
//! performance characteristics. In general, `spsub_csr_csr` and `spsub_csc_csc` are the fastest
//! possible subtraction operations one can perform. Mixing formats, such as in `spsub_csc_csr` or
//! `spsub_csr_csc` is slightly slower due to needing to iterate along the minor lane of the
//! secondary matrix.
//!
//! One should prefer to pose their problems as a combination of CSX <-> CSX subtractions, or
//! dense-sparse subtractions.

use crate::{
    convert::utils::CountToOffsetIter,
    cs::{CompressedColumnStorage, CompressedRowStorage, CsMatrix, CscMatrix, CsrMatrix},
    error::{OperationError, OperationErrorKind},
};
use nalgebra::{Dim, Matrix, RawStorage, RawStorageMut, Scalar};
use num_traits::Zero;
use std::{
    borrow::Borrow,
    cmp::Ordering,
    ops::{Add, Neg, Sub},
};

/// Sparse-sparse matrix subtraction.
///
/// This function takes two arguments, a CSR matrix and CSC matrix, and performs sparse-matrix
/// subtraction between the two.
///
/// # Errors
///
/// This function fails and produces an [`OperationError`] with kind
/// [`OperationErrorKind::InvalidPattern`] if the two matrices do not have the exact same shape.
pub fn spsub_csr_csc<T1, T2, MO1, MO2, MI1, MI2, D1, D2>(
    csr: CsMatrix<T1, MO1, MI1, D1, CompressedRowStorage>,
    csc: CsMatrix<T2, MO2, MI2, D2, CompressedColumnStorage>,
) -> Result<CsrMatrix<<T1 as Sub<T2>>::Output>, OperationError>
where
    T1: Scalar + Into<<T1 as Sub<T2>>::Output> + Sub<T2> + Zero,
    T2: Scalar,
    <T1 as Sub<T2>>::Output: Scalar,
    MO1: Borrow<[usize]>,
    MO2: Borrow<[usize]>,
    MI1: Borrow<[usize]>,
    MI2: Borrow<[usize]>,
    D1: Borrow<[T1]>,
    D2: Borrow<[T2]>,
{
    let (lrows, lcols) = csr.shape();
    let (rrows, rcols) = csc.shape();

    if lrows != rrows || lcols != rcols {
        return Err(OperationError::from_kind_and_message(
            OperationErrorKind::InvalidPattern,
            String::from("The two matrices have differing shapes (both should be M × N)"),
        ));
    }

    let mut left_iter = csr.triplet_iter();
    let mut right_iter = csc
        .minor_lane_iter()
        .enumerate()
        .flat_map(|(i, lane)| lane.map(move |(j, value)| (i, j, value)));

    let left_val = left_iter.next();
    let right_val = right_iter.next();

    let added_triplets = TripletSubtractionIter {
        left_val,
        right_val,
        left_iter,
        right_iter,
    };

    let max_nnz = csr.nnz() + csc.nnz();
    let mut counts = vec![0; lrows];
    let mut indices = Vec::with_capacity(max_nnz);
    let mut data = Vec::with_capacity(max_nnz);

    for (i, j, v) in added_triplets {
        counts[i] += 1;
        indices.push(j);
        data.push(v);
    }

    let offsets = CountToOffsetIter::new(counts).collect();

    Ok(unsafe { CsMatrix::from_parts_unchecked(lrows, lcols, offsets, indices, data) })
}

/// Sparse-sparse matrix subtraction.
///
/// This function takes two arguments, a CSC matrix and a CSR matrix, and performs sparse-matrix
/// subtraction between the two.
///
/// # Errors
///
/// This function fails and produces an [`OperationError`] with kind
/// [`OperationErrorKind::InvalidPattern`] if the two matrices do not have the exact same shape.
pub fn spsub_csc_csr<T1, T2, MO1, MO2, MI1, MI2, D1, D2>(
    csc: CsMatrix<T1, MO1, MI1, D1, CompressedColumnStorage>,
    csr: CsMatrix<T2, MO2, MI2, D2, CompressedRowStorage>,
) -> Result<CscMatrix<<T1 as Sub<T2>>::Output>, OperationError>
where
    T1: Scalar + Into<<T1 as Sub<T2>>::Output> + Sub<T2> + Zero,
    T2: Scalar,
    <T1 as Sub<T2>>::Output: Scalar,
    MO1: Borrow<[usize]>,
    MO2: Borrow<[usize]>,
    MI1: Borrow<[usize]>,
    MI2: Borrow<[usize]>,
    D1: Borrow<[T1]>,
    D2: Borrow<[T2]>,
{
    let (lrows, lcols) = csc.shape();
    let (rrows, rcols) = csr.shape();

    if lrows != rrows || lcols != rcols {
        return Err(OperationError::from_kind_and_message(
            OperationErrorKind::InvalidPattern,
            String::from("The two matrices have differing shapes (both should be M × N)"),
        ));
    }

    let mut left_iter = csc.triplet_iter();
    let mut right_iter = csr
        .minor_lane_iter()
        .enumerate()
        .flat_map(|(i, lane)| lane.map(move |(j, value)| (i, j, value)));

    let left_val = left_iter.next();
    let right_val = right_iter.next();

    let added_triplets = TripletSubtractionIter {
        left_val,
        right_val,
        left_iter,
        right_iter,
    };

    let max_nnz = csr.nnz() + csc.nnz();
    let mut counts = vec![0; lcols];
    let mut indices = Vec::with_capacity(max_nnz);
    let mut data = Vec::with_capacity(max_nnz);

    for (i, j, v) in added_triplets {
        counts[i] += 1;
        indices.push(j);
        data.push(v);
    }

    let offsets = CountToOffsetIter::new(counts).collect();

    Ok(unsafe { CsMatrix::from_parts_unchecked(lrows, lcols, offsets, indices, data) })
}

/// Sparse-sparse matrix subtraction.
///
/// This function takes two CSC matrices, and performs sparse-matrix subtraction between the two.
///
/// # Errors
///
/// This function fails and produces an [`OperationError`] with kind
/// [`OperationErrorKind::InvalidPattern`] if the two matrices do not have the exact same shape.
pub fn spsub_csc_csc<T1, T2, MO1, MO2, MI1, MI2, D1, D2>(
    lhs: CsMatrix<T1, MO1, MI1, D1, CompressedColumnStorage>,
    rhs: CsMatrix<T2, MO2, MI2, D2, CompressedColumnStorage>,
) -> Result<CscMatrix<<T1 as Sub<T2>>::Output>, OperationError>
where
    T1: Scalar + Into<<T1 as Sub<T2>>::Output> + Sub<T2> + Zero,
    T2: Scalar,
    <T1 as Sub<T2>>::Output: Scalar,
    MO1: Borrow<[usize]>,
    MO2: Borrow<[usize]>,
    MI1: Borrow<[usize]>,
    MI2: Borrow<[usize]>,
    D1: Borrow<[T1]>,
    D2: Borrow<[T2]>,
{
    let (lrows, lcols) = lhs.shape();
    let (rrows, rcols) = rhs.shape();

    if lrows != rrows || lcols != rcols {
        return Err(OperationError::from_kind_and_message(
            OperationErrorKind::InvalidPattern,
            String::from("The two matrices have differing shapes (both should be M × N)"),
        ));
    }

    let mut left_iter = lhs.triplet_iter();
    let mut right_iter = rhs.triplet_iter();

    let left_val = left_iter.next();
    let right_val = right_iter.next();

    let added_triplets = TripletSubtractionIter {
        left_val,
        right_val,
        left_iter,
        right_iter,
    };

    let max_nnz = lhs.nnz() + rhs.nnz();
    let mut counts = vec![0; lcols];
    let mut indices = Vec::with_capacity(max_nnz);
    let mut data = Vec::with_capacity(max_nnz);

    for (i, j, v) in added_triplets {
        counts[i] += 1;
        indices.push(j);
        data.push(v);
    }

    let offsets = CountToOffsetIter::new(counts).collect();

    Ok(unsafe { CsMatrix::from_parts_unchecked(lrows, lcols, offsets, indices, data) })
}

/// Sparse-sparse matrix subtraction.
///
/// This function takes two CSR matrices, and performs sparse-matrix subtraction between the two.
///
/// # Errors
///
/// This function fails and produces an [`OperationError`] with kind
/// [`OperationErrorKind::InvalidPattern`] if the two matrices do not have the exact same shape.
pub fn spsub_csr_csr<T1, T2, MO1, MO2, MI1, MI2, D1, D2>(
    lhs: CsMatrix<T1, MO1, MI1, D1, CompressedRowStorage>,
    rhs: CsMatrix<T2, MO2, MI2, D2, CompressedRowStorage>,
) -> Result<CsrMatrix<<T1 as Sub<T2>>::Output>, OperationError>
where
    T1: Scalar + Into<<T1 as Sub<T2>>::Output> + Sub<T2> + Zero,
    T2: Scalar,
    <T1 as Sub<T2>>::Output: Scalar,
    MO1: Borrow<[usize]>,
    MO2: Borrow<[usize]>,
    MI1: Borrow<[usize]>,
    MI2: Borrow<[usize]>,
    D1: Borrow<[T1]>,
    D2: Borrow<[T2]>,
{
    Ok(spsub_csc_csc(lhs.transpose(), rhs.transpose())?.transpose_owned())
}

/// Dense-sparse matrix subtraction.
///
/// This function takes in two matrices, one dense and one CSC matrix, and performs dense-sparse
/// subtraction between the two.
///
/// # Errors
///
/// Thsi function fails and produces an [`OperationError`] with kind
/// [`OperationErrorKind::InvalidPattern`] if the two matrices do not have the exact same shape.
pub fn spsub_dense_csc<T1, T2, R, C, S, MO, MI, D>(
    mut dense: Matrix<T1, R, C, S>,
    csc: CsMatrix<T2, MO, MI, D, CompressedColumnStorage>,
) -> Result<Matrix<T1, R, C, S>, OperationError>
where
    T1: Scalar + Sub<T2, Output = T1>,
    R: Dim,
    C: Dim,
    S: RawStorage<T1, R, C> + RawStorageMut<T1, R, C>,
    T2: Scalar,
    MO: Borrow<[usize]>,
    MI: Borrow<[usize]>,
    D: Borrow<[T2]>,
{
    let (lrows, lcols) = dense.shape();
    let (rrows, rcols) = csc.shape();

    if lrows != rrows || lcols != rcols {
        return Err(OperationError::from_kind_and_message(
            OperationErrorKind::InvalidPattern,
            String::from("The two matrices have differing shapes (both should be M × N)"),
        ));
    }

    for (col, row, val) in csc.triplet_iter() {
        let current = dense.index((row, col)).clone();
        *dense.index_mut((row, col)) = current - val.clone();
    }

    Ok(dense)
}

/// Dense-sparse matrix subtraction.
///
/// This function takes in two matrices, one dense and one CSC matrix, and performs dense-sparse
/// subtraction between the two.
///
/// # Errors
///
/// Thsi function fails and produces an [`OperationError`] with kind
/// [`OperationErrorKind::InvalidPattern`] if the two matrices do not have the exact same shape.
pub fn spsub_csc_dense<T1, T2, R, C, S, MO, MI, D>(
    csc: CsMatrix<T1, MO, MI, D, CompressedColumnStorage>,
    mut dense: Matrix<T2, R, C, S>,
) -> Result<Matrix<T2, R, C, S>, OperationError>
where
    T2: Scalar + Neg<Output = T2> + Add<T1, Output = T2>,
    R: Dim,
    C: Dim,
    S: RawStorage<T2, R, C> + RawStorageMut<T2, R, C>,
    T1: Scalar,
    MO: Borrow<[usize]>,
    MI: Borrow<[usize]>,
    D: Borrow<[T1]>,
{
    let (lrows, lcols) = dense.shape();
    let (rrows, rcols) = csc.shape();

    if lrows != rrows || lcols != rcols {
        return Err(OperationError::from_kind_and_message(
            OperationErrorKind::InvalidPattern,
            String::from("The two matrices have differing shapes (both should be M × N)"),
        ));
    }

    for dense_val in dense.iter_mut() {
        let neg = -(dense_val.clone());
        *dense_val = neg;
    }

    for (col, row, val) in csc.triplet_iter() {
        let current = dense.index((row, col)).clone();
        *dense.index_mut((row, col)) = current + val.clone();
    }

    Ok(dense)
}

/// Dense-sparse matrix subtraction.
///
/// This function takes in two matrices, one dense and one CSR matrix, and performs dense-sparse
/// subtraction between the two.
///
/// # Errors
///
/// Thsi function fails and produces an [`OperationError`] with kind
/// [`OperationErrorKind::InvalidPattern`] if the two matrices do not have the exact same shape.
pub fn spsub_dense_csr<T1, T2, R, C, S, MO, MI, D>(
    mut dense: Matrix<T1, R, C, S>,
    csr: CsMatrix<T2, MO, MI, D, CompressedRowStorage>,
) -> Result<Matrix<T1, R, C, S>, OperationError>
where
    T1: Scalar + Sub<T2, Output = T1>,
    R: Dim,
    C: Dim,
    S: RawStorage<T1, R, C> + RawStorageMut<T1, R, C>,
    T2: Scalar,
    MO: Borrow<[usize]>,
    MI: Borrow<[usize]>,
    D: Borrow<[T2]>,
{
    let (lrows, lcols) = dense.shape();
    let (rrows, rcols) = csr.shape();

    if lrows != rrows || lcols != rcols {
        return Err(OperationError::from_kind_and_message(
            OperationErrorKind::InvalidPattern,
            String::from("The two matrices have differing shapes (both should be M × N)"),
        ));
    }

    for (row, col, val) in csr.triplet_iter() {
        let current = dense.index((row, col)).clone();
        *dense.index_mut((row, col)) = current - val.clone();
    }

    Ok(dense)
}

/// Dense-sparse matrix subtraction.
///
/// This function takes in two matrices, one dense and one CSR matrix, and performs dense-sparse
/// subtraction between the two.
///
/// # Errors
///
/// Thsi function fails and produces an [`OperationError`] with kind
/// [`OperationErrorKind::InvalidPattern`] if the two matrices do not have the exact same shape.
pub fn spsub_csr_dense<T1, T2, R, C, S, MO, MI, D>(
    csr: CsMatrix<T1, MO, MI, D, CompressedRowStorage>,
    mut dense: Matrix<T2, R, C, S>,
) -> Result<Matrix<T2, R, C, S>, OperationError>
where
    T2: Scalar + Neg<Output = T2> + Add<T1, Output = T2>,
    R: Dim,
    C: Dim,
    S: RawStorage<T2, R, C> + RawStorageMut<T2, R, C>,
    T1: Scalar,
    MO: Borrow<[usize]>,
    MI: Borrow<[usize]>,
    D: Borrow<[T1]>,
{
    let (lrows, lcols) = dense.shape();
    let (rrows, rcols) = csr.shape();

    if lrows != rrows || lcols != rcols {
        return Err(OperationError::from_kind_and_message(
            OperationErrorKind::InvalidPattern,
            String::from("The two matrices have differing shapes (both should be M × N)"),
        ));
    }

    for dense_val in dense.iter_mut() {
        let neg = -(dense_val.clone());
        *dense_val = neg;
    }

    for (col, row, val) in csr.triplet_iter() {
        let current = dense.index((row, col)).clone();
        *dense.index_mut((row, col)) = current + val.clone();
    }

    Ok(dense)
}

/// Helper type which defines an iterator that takes two triplet iterators (in the same major
/// ordering) and outputs a single triplet iterator of `(major, minor, owned_value)`. This iterator
/// combines duplicate entries via subtraction i.e. if we have two entries with the same major and
/// minor indices we add the two values together.
///
/// This iterator combinator is not terribly well bound, which is why it is private! First and
/// foremost, `left_val` and `right_val` need to not be `None` immediately, or this will always
/// return `None`. Secondly, this iterator is difficult to use directly outside of the above
/// `spsub_csx_csx` functions because it relies on the fact that both iterators are outputting
/// triplets in the same major ordering. This means that if you want to add a CSC and CSR matrix,
/// you cannot do the following:
///
/// ```ignore
/// let mut left_iter = csr.triplet_iter();
/// let mut right_iter = csc.triplet_iter();
///
/// let left_val = left_iter.next();
/// let right_val = right_iter.next();
/// ```
///
/// Because the major and minor ordering of those two are reversed. Instead, you'll want to do
/// something like the following:
///
/// ```ignore
/// let mut left_iter = csr.triplet_iter();
/// let mut right_iter = csc
///     .minor_lane_iter()
///     .enumerate()
///     .flat_map(|(i, lane)| lane.map(move |(j, value)| (i, j, value)));
///
/// let left_val = left_iter.next();
/// let right_val = right_iter.next();
/// ```
///
/// The minor lane iterator is necessary so that you can both traverse the two matrices in
/// the equivalent order (which is needed for the boolean logic below to work) as well as compare
/// `i` to `i` and `j` to `j` between both left and right values.
///
/// Of course, because subtraction is commutative, one could just reverse the order of the
/// arguments and do the same trick; however, it means that the final result will be a column-major
/// matrix instead of a row-major matrix.
struct TripletSubtractionIter<'a, TL, TR, IL, IR>
where
    TL: Scalar + Into<<TL as Sub<TR>>::Output> + Sub<TR> + Zero,
    TR: Scalar,
    IL: Iterator<Item = (usize, usize, &'a TL)>,
    IR: Iterator<Item = (usize, usize, &'a TR)>,
{
    left_val: Option<(usize, usize, &'a TL)>,
    right_val: Option<(usize, usize, &'a TR)>,
    left_iter: IL,
    right_iter: IR,
}

impl<'a, TL, TR, IL, IR> Iterator for TripletSubtractionIter<'a, TL, TR, IL, IR>
where
    TL: Scalar + Into<<TL as Sub<TR>>::Output> + Sub<TR> + Zero,
    TR: Scalar,
    IL: Iterator<Item = (usize, usize, &'a TL)>,
    IR: Iterator<Item = (usize, usize, &'a TR)>,
{
    type Item = (usize, usize, <TL as Sub<TR>>::Output);

    fn next(&mut self) -> Option<Self::Item> {
        match (self.left_val, self.right_val) {
            // Neither left nor right are exhausted
            (Some((il, jl, vl)), Some((ir, jr, vr))) => match (il.cmp(&ir), jl.cmp(&jr)) {
                (Ordering::Less, _) | (Ordering::Equal, Ordering::Less) => {
                    self.left_val = self.left_iter.next();
                    Some((il, jl, vl.clone().into()))
                }

                (Ordering::Greater, _) | (Ordering::Equal, Ordering::Greater) => {
                    self.right_val = self.right_iter.next();
                    Some((ir, jr, TL::zero() - vr.clone()))
                }

                (Ordering::Equal, Ordering::Equal) => {
                    self.left_val = self.left_iter.next();
                    self.right_val = self.right_iter.next();

                    Some((il, jl, vl.clone() - vr.clone()))
                }
            },

            // Only right is exhausted
            (Some((i, j, v)), None) => {
                self.left_val = self.left_iter.next();
                Some((i, j, v.clone().into()))
            }

            // Only left is exhausted
            (None, Some((i, j, v))) => {
                self.right_val = self.right_iter.next();
                Some((i, j, TL::zero() - v.clone()))
            }

            // Both are exhausted
            (None, None) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proptest::*;
    use matrixcompare::{assert_matrix_eq, prop_assert_matrix_eq};
    use nalgebra::DMatrix;
    use proptest::prelude::*;

    #[test]
    fn spsub_csr_csr_agrees_with_dense() {
        let a = CsrMatrix::try_from_parts(
            4,
            4,
            vec![0, 4, 7, 10],
            vec![0, 1, 2, 3, 0, 1, 3, 1, 2, 3, 0, 1, 3],
            vec![1, 2, 3, 4, -1, 2, 5, 4, -2, 6, 2, 4, 6],
        )
        .unwrap();

        let b = CsrMatrix::try_from_parts(
            4,
            4,
            vec![0, 4, 6, 8],
            vec![0, 1, 2, 3, 1, 3, 1, 3, 0, 1, 2, 3],
            vec![6, 4, 2, 8, 1, 7, 2, 6, 4, 1, 6, 3],
        )
        .unwrap();

        let dense_a = DMatrix::from(&a);
        let dense_b = DMatrix::from(&b);

        let diff = spsub_csr_csr(a, b).unwrap();
        let dense_diff = dense_a - dense_b;

        assert_matrix_eq!(diff, dense_diff);
    }

    #[test]
    fn spsub_csr_csc_agrees_with_dense() {
        let a = CsrMatrix::try_from_parts(
            4,
            4,
            vec![0, 4, 7, 10],
            vec![0, 1, 2, 3, 0, 1, 3, 1, 2, 3, 0, 1, 3],
            vec![1, 2, 3, 4, -1, 2, 5, 4, -2, 6, 2, 4, 6],
        )
        .unwrap();

        let b = CscMatrix::try_from_parts(
            4,
            4,
            vec![0, 4, 6, 8],
            vec![0, 1, 2, 3, 1, 3, 1, 3, 0, 1, 2, 3],
            vec![6, 4, 2, 8, 1, 7, 2, 6, 4, 1, 6, 3],
        )
        .unwrap();

        let dense_a = DMatrix::from(&a);
        let dense_b = DMatrix::from(&b);

        let diff = spsub_csr_csc(a, b).unwrap();
        let dense_diff = dense_a - dense_b;

        assert_matrix_eq!(diff, dense_diff);
    }

    #[test]
    fn spsub_csc_csr_agrees_with_dense() {
        let a = CscMatrix::try_from_parts(
            4,
            4,
            vec![0, 4, 7, 10],
            vec![0, 1, 2, 3, 0, 1, 3, 1, 2, 3, 0, 1, 3],
            vec![1, 2, 3, 4, -1, 2, 5, 4, -2, 6, 2, 4, 6],
        )
        .unwrap();

        let b = CsrMatrix::try_from_parts(
            4,
            4,
            vec![0, 4, 6, 8],
            vec![0, 1, 2, 3, 1, 3, 1, 3, 0, 1, 2, 3],
            vec![6, 4, 2, 8, 1, 7, 2, 6, 4, 1, 6, 3],
        )
        .unwrap();

        let dense_a = DMatrix::from(&a);
        let dense_b = DMatrix::from(&b);

        let diff = spsub_csc_csr(a, b).unwrap();
        let dense_diff = dense_a - dense_b;

        assert_matrix_eq!(diff, dense_diff);
    }

    #[test]
    fn spsub_csc_csc_agrees_with_dense() {
        let a = CscMatrix::try_from_parts(
            4,
            4,
            vec![0, 4, 7, 10],
            vec![0, 1, 2, 3, 0, 1, 3, 1, 2, 3, 0, 1, 3],
            vec![1, 2, 3, 4, -1, 2, 5, 4, -2, 6, 2, 4, 6],
        )
        .unwrap();

        let b = CscMatrix::try_from_parts(
            4,
            4,
            vec![0, 4, 6, 8],
            vec![0, 1, 2, 3, 1, 3, 1, 3, 0, 1, 2, 3],
            vec![6, 4, 2, 8, 1, 7, 2, 6, 4, 1, 6, 3],
        )
        .unwrap();

        let dense_a = DMatrix::from(&a);
        let dense_b = DMatrix::from(&b);

        let diff = spsub_csc_csc(a, b).unwrap();
        let dense_diff = dense_a - dense_b;

        assert_matrix_eq!(diff, dense_diff);
    }

    proptest! {
        #[test]
        fn spsub_csr_csr_subtractive_identity(matrix in csr_strategy()) {
            let (nrows, ncols) = matrix.shape();
            let zero = CsrMatrix::<i32>::zeros(nrows, ncols);
            let diff = spsub_csr_csr(matrix.to_view(), zero).unwrap();

            prop_assert_matrix_eq!(diff, matrix);
        }

        #[test]
        fn spsub_csr_csc_subtractive_identity(matrix in csr_strategy()) {
            let (nrows, ncols) = matrix.shape();
            let zero = CscMatrix::<i32>::zeros(nrows, ncols);
            let diff = spsub_csr_csc(matrix.to_view(), zero).unwrap();

            prop_assert_matrix_eq!(diff, matrix);
        }

        #[test]
        fn spsub_csc_csr_subtractive_identity(matrix in csc_strategy()) {
            let (nrows, ncols) = matrix.shape();
            let zero = CsrMatrix::<i32>::zeros(nrows, ncols);
            let diff = CscMatrix::from(spsub_csc_csr(matrix.to_view(), zero).unwrap());

            prop_assert_matrix_eq!(diff, matrix);
        }

        #[test]
        fn spsub_csc_csc_subtractive_identity(matrix in csc_strategy()) {
            let (nrows, ncols) = matrix.shape();
            let zero = CscMatrix::<i32>::zeros(nrows, ncols);
            let diff = spsub_csc_csc(matrix.to_view(), zero).unwrap();

            prop_assert_matrix_eq!(diff, matrix);
        }
    }
}
