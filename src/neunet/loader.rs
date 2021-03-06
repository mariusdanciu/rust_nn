use std::io::*;

use nalgebra::*;

pub trait DataLoader {
    fn load_data(self, data_path: String, labels_path: String) -> Result<(DMatrix<f32>, DVector<u8>)>;
}
