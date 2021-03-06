use nalgebra::*;

pub fn min_max_normalization(features: &mut DMatrix<f32>) {
    let (rows, cols) = features.shape();

    for c in 0..cols {
        let col_vec = features.column(c);
        let min = col_vec.min();
        let max = col_vec.max();

        let mut index = c * rows;
        for _ in 0..rows {
            if max > 0.0 {
                let cell = features.index_mut(index);
                *cell = (*cell - min) / { max - min };
            }
            index += 1;
        }
    }
}