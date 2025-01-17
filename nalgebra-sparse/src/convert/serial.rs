//! Serial routines for converting between matrix formats.
//!
//! All routines in this module are single-threaded. At present these routines offer no
//! advantage over using the [`From`] trait, but future changes to the API might offer more
//! control to the user.
use super::utils;
use crate::{
    coo::CooMatrix,
    cs::{
        CompressedColumnStorage, CompressedRowStorage, Compression, CsMatrix, CscMatrix, CsrMatrix,
    },
};
use nalgebra::{ClosedAdd, DMatrix, Dim, Matrix, RawStorage, Scalar};
use num_traits::Zero;
use std::{borrow::Borrow, ops::Add};

/// Converts a dense matrix to [`CooMatrix`].
pub fn convert_dense_coo<T, R, C, S>(dense: &Matrix<T, R, C, S>) -> CooMatrix<T>
where
    T: Scalar + Zero,
    R: Dim,
    C: Dim,
    S: RawStorage<T, R, C>,
{
    let mut coo = CooMatrix::new(dense.nrows(), dense.ncols());

    for (index, v) in dense.iter().enumerate() {
        if v != &T::zero() {
            // We use the fact that matrix iteration is guaranteed to be column-major
            let i = index % dense.nrows();
            let j = index / dense.nrows();
            coo.push(i, j, v.clone());
        }
    }

    coo
}

/// Converts a [`CooMatrix`] to a dense matrix.
pub fn convert_coo_dense<T>(coo: &CooMatrix<T>) -> DMatrix<T>
where
    T: Scalar + Zero + ClosedAdd,
{
    let mut output = DMatrix::<T>::zeros(coo.nrows(), coo.ncols());
    for (i, j, v) in coo.triplet_iter() {
        output[(i, j)] += v.clone();
    }
    output
}

/// Converts a [`CooMatrix`] to a [`CsrMatrix`].
pub fn convert_coo_csr<T>(coo: CooMatrix<T>) -> CsrMatrix<T>
where
    T: Scalar + Add<Output = T>,
{
    convert_coo_cs(coo, &Add::add)
}

/// Converts a [`CsrMatrix`] to a [`CooMatrix`].
pub fn convert_csr_coo<T, MO, MI, D>(
    csr: &CsMatrix<T, MO, MI, D, CompressedRowStorage>,
) -> CooMatrix<T>
where
    T: Scalar,
    MO: Borrow<[usize]>,
    MI: Borrow<[usize]>,
    D: Borrow<[T]>,
{
    let mut result = CooMatrix::new(csr.nrows(), csr.ncols());
    for (i, j, v) in csr.triplet_iter() {
        result.push(i, j, v.clone());
    }
    result
}

/// Converts a [`CsrMatrix`] to a dense matrix.
pub fn convert_csr_dense<T, MO, MI, D>(
    csr: &CsMatrix<T, MO, MI, D, CompressedRowStorage>,
) -> DMatrix<T>
where
    T: Scalar + ClosedAdd + Zero,
    MO: Borrow<[usize]>,
    MI: Borrow<[usize]>,
    D: Borrow<[T]>,
{
    let mut output = DMatrix::zeros(csr.nrows(), csr.ncols());

    for (i, j, v) in csr.triplet_iter() {
        output[(i, j)] += v.clone();
    }

    output
}

/// Converts a dense matrix to a [`CsrMatrix`].
pub fn convert_dense_csr<T, R, C, S>(dense: &Matrix<T, R, C, S>) -> CsrMatrix<T>
where
    T: Scalar + Zero,
    R: Dim,
    C: Dim,
    S: RawStorage<T, R, C>,
{
    let mut row_offsets = Vec::with_capacity(dense.nrows());
    let mut col_idx = Vec::new();
    let mut values = Vec::new();

    // We have to iterate row-by-row to build the CSR matrix, which is at odds with
    // nalgebra's column-major storage. The alternative would be to perform an initial sweep
    // to count number of non-zeros per row.
    row_offsets.push(0);
    for i in 0..dense.nrows() {
        for j in 0..dense.ncols() {
            let v = dense.index((i, j));
            if v != &T::zero() {
                col_idx.push(j);
                values.push(v.clone());
            }
        }

        if i < dense.nrows() - 1 {
            row_offsets.push(col_idx.len());
        }
    }

    unsafe {
        CsrMatrix::from_parts_unchecked(dense.nrows(), dense.ncols(), row_offsets, col_idx, values)
    }
}

/// Converts a [`CooMatrix`] to a [`CscMatrix`].
pub fn convert_coo_csc<T>(coo: CooMatrix<T>) -> CscMatrix<T>
where
    T: Scalar + Add<Output = T>,
{
    convert_coo_cs(coo, &Add::add)
}

/// Converts a [`CscMatrix`] to a [`CooMatrix`].
pub fn convert_csc_coo<T, MO, MI, D>(
    csc: &CsMatrix<T, MO, MI, D, CompressedColumnStorage>,
) -> CooMatrix<T>
where
    T: Scalar,
    MO: Borrow<[usize]>,
    MI: Borrow<[usize]>,
    D: Borrow<[T]>,
{
    let mut coo = CooMatrix::new(csc.nrows(), csc.ncols());
    for (i, j, v) in csc.triplet_iter() {
        coo.push(j, i, v.clone());
    }
    coo
}

/// Converts a [`CscMatrix`] to a dense matrix.
pub fn convert_csc_dense<T, MO, MI, D>(
    csc: &CsMatrix<T, MO, MI, D, CompressedColumnStorage>,
) -> DMatrix<T>
where
    T: Scalar + ClosedAdd + Zero,
    MO: Borrow<[usize]>,
    MI: Borrow<[usize]>,
    D: Borrow<[T]>,
{
    let mut output = DMatrix::zeros(csc.nrows(), csc.ncols());

    for (i, j, v) in csc.triplet_iter() {
        output[(j, i)] += v.clone();
    }

    output
}

/// Converts a dense matrix to a [`CscMatrix`].
pub fn convert_dense_csc<T, R, C, S>(dense: &Matrix<T, R, C, S>) -> CscMatrix<T>
where
    T: Scalar + Zero,
    R: Dim,
    C: Dim,
    S: RawStorage<T, R, C>,
{
    let mut col_offsets = Vec::with_capacity(dense.ncols());
    let mut row_idx = Vec::new();
    let mut values = Vec::new();

    col_offsets.push(0);

    for j in 0..dense.ncols() {
        for i in 0..dense.nrows() {
            let v = dense.index((i, j));
            if v != &T::zero() {
                row_idx.push(i);
                values.push(v.clone());
            }
        }

        if j < dense.ncols() - 1 {
            col_offsets.push(row_idx.len());
        }
    }

    unsafe {
        CscMatrix::from_parts_unchecked(dense.nrows(), dense.ncols(), col_offsets, row_idx, values)
    }
}

/// Converts a [`CsrMatrix`] to a [`CscMatrix`].
pub fn convert_csr_csc<T, MO, MI, D>(
    csr: &CsMatrix<T, MO, MI, D, CompressedRowStorage>,
) -> CscMatrix<T>
where
    T: Scalar,
    MO: Borrow<[usize]>,
    MI: Borrow<[usize]>,
    D: Borrow<[T]>,
{
    let (nrows, ncols) = csr.shape();

    let (counts, indices_and_data) = csr
        .minor_lane_iter()
        .map(|lane| {
            let (indices, data) = lane
                .map(|(i, v)| (i, v.clone()))
                .unzip::<_, _, Vec<_>, Vec<_>>();

            (indices.len(), (indices, data))
        })
        .unzip::<_, _, Vec<_>, Vec<_>>();

    let nnz = counts.iter().sum();
    let offsets = utils::CountToOffsetIter::new(counts).collect();

    let mut indices = Vec::with_capacity(nnz);
    let mut data = Vec::with_capacity(nnz);

    for (mut row_indices, mut row_data) in indices_and_data {
        indices.append(&mut row_indices);
        data.append(&mut row_data);
    }

    unsafe { CscMatrix::from_parts_unchecked(nrows, ncols, offsets, indices, data) }
}

/// Converts a [`CscMatrix`] to a [`CsrMatrix`].
pub fn convert_csc_csr<T, MO, MI, D>(
    csc: &CsMatrix<T, MO, MI, D, CompressedColumnStorage>,
) -> CsrMatrix<T>
where
    T: Scalar,
    MO: Borrow<[usize]>,
    MI: Borrow<[usize]>,
    D: Borrow<[T]>,
{
    let (nrows, ncols) = csc.shape();

    let (counts, indices_and_data) = csc
        .minor_lane_iter()
        .map(|lane| {
            let (indices, data) = lane
                .map(|(i, v)| (i, v.clone()))
                .unzip::<_, _, Vec<_>, Vec<_>>();

            (indices.len(), (indices, data))
        })
        .unzip::<_, _, Vec<_>, Vec<_>>();

    let nnz = counts.iter().sum();
    let offsets = utils::CountToOffsetIter::new(counts).collect();

    let mut indices = Vec::with_capacity(nnz);
    let mut data = Vec::with_capacity(nnz);

    for (mut row_indices, mut row_data) in indices_and_data {
        indices.append(&mut row_indices);
        data.append(&mut row_data);
    }

    unsafe { CsrMatrix::from_parts_unchecked(nrows, ncols, offsets, indices, data) }
}

/// Converts a COO matrix to a CsMatrix, resolving duplicates with the provided combinator.
fn convert_coo_cs<T, C, F>(
    coo: CooMatrix<T>,
    combinator: F,
) -> CsMatrix<F::Output, Vec<usize>, Vec<usize>, Vec<F::Output>, C>
where
    T: Scalar,
    C: Compression,
    F: Fn(T, T) -> T,
{
    let nrows = coo.nrows();
    let ncols = coo.ncols();

    let nmajor = C::nmajor(nrows, ncols);

    let (coo_rows, coo_cols, coo_data) = coo.disassemble();

    let mut triplets = coo_rows
        .into_iter()
        .zip(coo_cols)
        .map(|(r, c)| {
            let nmajor = C::nmajor(r, c);
            let nminor = C::nminor(r, c);

            (nmajor, nminor)
        })
        .zip(coo_data)
        .collect::<Vec<_>>();

    // Sort the triplets according to their index positions, lexicographically.
    //
    // This gives us the triplets in the correct ordering, because we already mapped every "index"
    // pair as (nmajor, nminor), and tuples sort lexicographically.
    //
    // In particular, we should expect it to be sorted according to major -> minor so that we get
    // e.g.
    //
    // - (0, 0)
    // - (0, 1)
    // - (1, 3)
    // - (1, 3)
    // - (1, 4)
    // - (2, 4)
    // - etc.
    //
    // Where the first number is the major axis, and the second is the minor axis.
    triplets.sort_unstable_by(|(left_idx, _), (right_idx, _)| left_idx.cmp(right_idx));

    let mut counts = vec![0usize; nmajor];
    let mut indices = Vec::with_capacity(triplets.len());
    let mut data = Vec::<T>::with_capacity(triplets.len());

    let mut i_prev = None;

    for ((i, j), val) in triplets {
        // This checks for duplicates, and resolves them with the appropriate combinator.
        //
        // We can check for duplicates merely by seeing if the last i and j are the same, since we
        // know that the triplets have been sorted.
        if let Some(i_prev) = i_prev {
            if i == i_prev {
                if let Some(j_prev) = indices.last() {
                    if j == *j_prev {
                        // We know this should exist if indices.last() exists
                        let prev_val = data.last_mut().unwrap();
                        *prev_val = combinator(prev_val.clone(), val);

                        continue;
                    }
                }
            }
        }

        counts[i] += 1;
        indices.push(j);
        data.push(val);

        i_prev = Some(i);
    }

    let offsets = utils::CountToOffsetIter::new(counts).collect();

    unsafe { CsMatrix::from_parts_unchecked(nrows, ncols, offsets, indices, data) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proptest::*;
    use matrixcompare::{assert_matrix_eq, prop_assert_matrix_eq};
    use nalgebra::SMatrix;
    use proptest::prelude::*;

    #[test]
    fn coo_from_dense_and_dense_from_coo_are_symmetric() {
        #[rustfmt::skip]
        let dense = SMatrix::<usize, 2, 3>::from_row_slice(&[
            1, 0, 3,
            0, 5, 0,
        ]);

        // The COO representation of a dense matrix is not unique.
        // Here we implicitly test that the coo matrix is indeed constructed from column-major
        // iteration of the dense matrix.
        let coo = CooMatrix::try_from_triplets(2, 3, vec![0, 1, 0], vec![0, 1, 2], vec![1, 5, 3])
            .unwrap();

        assert_eq!(convert_dense_coo(&dense), coo);
        assert_eq!(convert_coo_dense(&coo), dense);
    }

    #[test]
    fn coo_with_duplicates_from_dense_and_dense_from_coo_with_duplicates_are_symmetric() {
        #[rustfmt::skip]
        let dense = SMatrix::<i64, 2, 3>::from_row_slice(&[
            1, 0, 3,
            0, 5, 0,
        ]);

        let coo_no_dup =
            CooMatrix::try_from_triplets(2, 3, vec![0, 1, 0], vec![0, 1, 2], vec![1, 5, 3])
                .unwrap();

        let coo_dup = CooMatrix::try_from_triplets(
            2,
            3,
            vec![0, 1, 0, 1],
            vec![0, 1, 2, 1],
            vec![1, -2, 3, 7],
        )
        .unwrap();

        let converted_coo = convert_coo_dense(&coo_dup);

        assert_eq!(&converted_coo, &dense);

        let converted_coo_without_duplicates = convert_dense_coo(&converted_coo);

        assert_eq!(converted_coo_without_duplicates, coo_no_dup);
    }

    #[test]
    fn csr_from_coo_has_expected_format() {
        let coo = {
            let mut coo = CooMatrix::new(3, 4);
            coo.push(1, 3, 4);
            coo.push(0, 1, 2);
            coo.push(2, 0, 1);
            coo.push(2, 3, 2);
            coo.push(2, 2, 1);
            coo
        };

        let expected_csr = CsrMatrix::try_from_parts(
            3,
            4,
            vec![0, 1, 2],
            vec![1, 3, 0, 2, 3],
            vec![2, 4, 1, 1, 2],
        )
        .unwrap();

        let converted_csr = convert_coo_csr(coo);

        assert_matrix_eq!(converted_csr, expected_csr);
    }

    #[test]
    fn csr_from_coo_with_duplicates_has_expected_format() {
        let coo = {
            let mut coo = CooMatrix::new(3, 4);
            coo.push(1, 3, 4);
            coo.push(2, 3, 2);
            coo.push(0, 1, 2);
            coo.push(2, 0, 1);
            coo.push(2, 3, 2);
            coo.push(0, 1, 3);
            coo.push(2, 2, 1);
            coo
        };

        let expected_csr = CsrMatrix::try_from_parts(
            3,
            4,
            vec![0, 1, 2],
            vec![1, 3, 0, 2, 3],
            vec![5, 4, 1, 1, 4],
        )
        .unwrap();

        let converted_csr = convert_coo_csr(coo);

        assert_matrix_eq!(converted_csr, expected_csr);
    }

    #[test]
    fn csc_from_coo_has_expected_format() {
        let coo = {
            let mut coo = CooMatrix::new(3, 4);
            coo.push(1, 3, 4);
            coo.push(0, 1, 2);
            coo.push(2, 0, 1);
            coo.push(2, 3, 2);
            coo.push(2, 2, 1);
            coo
        };

        let expected_csc = CscMatrix::try_from_parts(
            3,
            4,
            vec![0, 1, 2, 3],
            vec![2, 0, 2, 1, 2],
            vec![1, 2, 1, 4, 2],
        )
        .unwrap();

        let converted_csc = convert_coo_csc(coo);

        assert_matrix_eq!(converted_csc, expected_csc);
    }

    #[test]
    fn csc_from_coo_with_duplicates_has_expected_format() {
        let coo = {
            let mut coo = CooMatrix::new(3, 4);
            coo.push(1, 3, 4);
            coo.push(2, 3, 2);
            coo.push(0, 1, 2);
            coo.push(2, 0, 1);
            coo.push(2, 3, 2);
            coo.push(0, 1, 3);
            coo.push(2, 2, 1);
            coo
        };

        let expected_csc = CscMatrix::try_from_parts(
            3,
            4,
            vec![0, 1, 2, 3],
            vec![2, 0, 2, 1, 2],
            vec![1, 5, 1, 4, 4],
        )
        .unwrap();

        let converted_csc = convert_coo_csc(coo);

        assert_matrix_eq!(converted_csc, expected_csc);
    }

    #[test]
    fn coo_from_csr_has_expected_format() {
        let csr = CsrMatrix::try_from_parts(
            3,
            4,
            vec![0, 1, 2],
            vec![1, 3, 0, 2, 3],
            vec![5, 4, 1, 1, 4],
        )
        .unwrap();

        let expected_coo = CooMatrix::try_from_triplets(
            3,
            4,
            vec![0, 1, 2, 2, 2],
            vec![1, 3, 0, 2, 3],
            vec![5, 4, 1, 1, 4],
        )
        .unwrap();

        assert_matrix_eq!(convert_csr_coo(&csr), expected_coo);
    }

    #[test]
    fn coo_from_csc_has_expected_format() {
        let csc = CscMatrix::try_from_parts(
            3,
            4,
            vec![0, 1, 2, 3],
            vec![2, 0, 2, 1, 2],
            vec![1, 2, 1, 4, 2],
        )
        .unwrap();

        let expected_coo = CooMatrix::try_from_triplets(
            3,
            4,
            vec![2, 0, 2, 1, 2],
            vec![0, 1, 2, 3, 3],
            vec![1, 2, 1, 4, 2],
        )
        .unwrap();

        assert_matrix_eq!(convert_csc_coo(&csc), expected_coo);
    }

    // FIXME: This test isn't _exactly_ correct. Explicit zeros stored in the matrix will be
    // removed when convert_dense_csr is called (kind of the whole point of the format).
    //
    // Anyways, be careful as dense <-> cs matrix conversions can imply this sort of thing, and
    // you'll wonder why e.g. proptest can generate matrices for where this isn't true.
    #[test]
    fn csr_dense_conversion_is_reflective() {
        let csr = CsrMatrix::try_from_parts(
            3,
            4,
            vec![0, 3, 4],
            vec![1, 2, 3, 0, 1, 3],
            vec![5, 3, 2, 2, 1, 4],
        )
        .unwrap();

        #[rustfmt::skip]
        let dense = DMatrix::from_row_slice(3, 4, &[
            0, 5, 3, 2,
            2, 0, 0, 0,
            0, 1, 0, 4
        ]);

        assert_matrix_eq!(convert_csr_dense(&csr), dense);

        let final_csr = convert_dense_csr(&dense);

        assert_matrix_eq!(csr, final_csr);
    }

    // FIXME: Same as previous test, this can fail when explicit zeros are stored.
    #[test]
    fn csc_dense_conversion_is_reflective() {
        let csc = CscMatrix::try_from_parts(
            3,
            4,
            vec![0, 1, 3, 4],
            vec![1, 0, 2, 0, 0, 2],
            vec![2, 5, 1, 3, 2, 4],
        )
        .unwrap();

        #[rustfmt::skip]
        let dense = DMatrix::from_row_slice(3, 4, &[
            0, 5, 3, 2,
            2, 0, 0, 0,
            0, 1, 0, 4
        ]);

        assert_matrix_eq!(convert_csc_dense(&csc), dense);

        let final_csc = convert_dense_csc(&dense);

        assert_matrix_eq!(csc, final_csc);
    }

    proptest! {
        #[test]
        fn csc_csr_csc_conversion_is_reflective(csc in csc_strategy()) {
            let csr = convert_csc_csr(&csc);
            let final_csc = convert_csr_csc(&csr);

            prop_assert_matrix_eq!(csc, final_csc);
        }

        #[test]
        fn csr_csc_csr_conversion_is_reflective(csr in csr_strategy()) {
            let csc = convert_csr_csc(&csr);
            let final_csr = convert_csc_csr(&csc);

            prop_assert_matrix_eq!(csr, final_csr);
        }

        #[test]
        fn dense_coo_dense_is_reflective(dense in dense_strategy()) {
            let coo = convert_dense_coo(&dense);
            let final_dense = convert_coo_dense(&coo);

            prop_assert_matrix_eq!(dense, final_dense);
        }

        #[test]
        fn coo_dense_coo_conversion_is_reflective(coo in coo_strategy()) {
            // We cannot compare the result of the roundtrip coo -> dense -> coo directly for
            // two reasons:
            //  1. the COO matrices will generally have different ordering of elements
            //  2. explicitly stored zero entries in the original matrix will be discarded
            //     when converting back to COO
            // Therefore we instead compare the results of converting the COO matrix
            // at the end of the roundtrip with its dense representation
            let dense = convert_coo_dense(&coo);
            let coo2 = convert_dense_coo(&dense);
            let dense2 = convert_coo_dense(&coo2);
            prop_assert_matrix_eq!(dense, dense2);
        }

        #[test]
        fn coo_csr_agrees_with_csr_dense(coo in coo_strategy()) {
            let coo_dense = convert_coo_dense(&coo);
            let csr = convert_coo_csr(coo.clone());
            let csr_dense = convert_csr_dense(&csr);

            prop_assert_matrix_eq!(csr_dense, coo_dense);
        }

        #[test]
        fn coo_csr_equal_nnz_when_no_duplicates(coo in coo_no_duplicates_strategy()) {
            // Check that the NNZ are equal when converting from a CooMatrix without
            // duplicates to a CSR matrix
            let csr = convert_coo_csr(coo.clone());
            prop_assert_eq!(csr.nnz(), coo.nnz());
        }

        #[test]
        fn coo_csc_agrees_with_csc_dense(coo in coo_strategy()) {
            let coo_dense = convert_coo_dense(&coo);
            let csc = convert_coo_csc(coo.clone());
            let csc_dense = convert_csc_dense(&csc);

            prop_assert_matrix_eq!(csc_dense, coo_dense);
        }

        #[test]
        fn coo_csc_equal_nnz_when_no_duplicates(coo in coo_no_duplicates_strategy()) {
            // Check that the NNZ are equal when converting from a CooMatrix without
            // duplicates to a csc matrix
            let csc = convert_coo_csc(coo.clone());
            prop_assert_eq!(csc.nnz(), coo.nnz());
        }

        #[test]
        fn csr_coo_csr_is_reflective(csr in csr_strategy()) {
            let coo = convert_csr_coo(&csr);
            let final_csr = convert_coo_csr(coo);

            prop_assert_matrix_eq!(csr, final_csr);
        }

        #[test]
        fn csc_coo_csc_is_reflective(csc in csc_strategy()) {
            let coo = convert_csc_coo(&csc);
            let final_csc = convert_coo_csc(coo);

            prop_assert_matrix_eq!(csc, final_csc);
        }

        // Test is only valid if there are no explicit zeros in the matrix
        #[test]
        fn csr_dense_csr_is_reflective(csr in non_zero_csr_strategy()) {
            let dense = convert_csr_dense(&csr);
            let final_csr = convert_dense_csr(&dense);

            prop_assert_matrix_eq!(csr, final_csr);
        }

        // Test is only valid if there are no explicit zeros in the matrix
        #[test]
        fn csc_dense_csc_is_reflective(csc in non_zero_csc_strategy()) {
            let dense = convert_csc_dense(&csc);
            let final_csc = convert_dense_csc(&dense);

            prop_assert_matrix_eq!(csc, final_csc);
        }
    }
}
