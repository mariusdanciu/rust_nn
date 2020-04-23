#![allow(dead_code)]

use nalgebra::{DMatrix, DVector};
use nalgebra::*;
use rand_distr::{Distribution, Normal};
use serde_json::json;

pub enum ActivationType {
    Sigmoid,
    Relu,
    Tanh,
    SoftMax,
}


pub struct TrainingInfo {
    pub hyper_params: HyperParams,
    pub num_epochs_used: u32,
    pub num_iterations_used: u32,
    pub loss: f32,
}

pub struct NNModel {
    pub num_features: usize,
    pub num_classes: usize,
    pub layers: Vec<Layer>,
    pub training_info: Option<TrainingInfo>,
}


pub enum OptimizationType {
    MBGD,
    Momentum,
    RMSProp,
    Adam,
}

pub struct HyperParams {
    pub max_accuracy_threshold: f32,
    pub max_epochs: u32,
    pub momentum_beta: f32,
    pub rms_prop_beta: f32,
    pub mini_batch_size: usize,
    pub learning_rate: f32,
    pub optimization_type: OptimizationType,
    pub l2_regularization: Option<f32>,
}

impl Default for HyperParams {
    fn default() -> Self {
        HyperParams {
            max_accuracy_threshold: 0.95,
            max_epochs: 3,
            momentum_beta: 0.9,
            rms_prop_beta: 0.999,
            mini_batch_size: 200,
            learning_rate: 0.01,
            optimization_type: OptimizationType::Adam,
            l2_regularization: None,
        }
    }
}

#[derive(Debug)]
pub struct LabeledData<'a> {
    pub features: DMatrixSlice<'a, f32>,
    pub labels: DMatrixSlice<'a, f32>,
}

pub trait Json {
    fn to_json(&self, pretty: bool) -> String;
}

#[derive(Debug)]
pub struct TrainingEval {
    pub confusion_matrix_dim: usize,
    pub confusion_matrix: DMatrix<usize>,
    pub labels_accuracies: Vec<f32>,
    pub accuracy: f32,
}

#[derive(Debug)]
pub struct Metrics {
    pub loss: f32,
    pub train_eval: TrainingEval,
    pub test_eval: TrainingEval,
}

#[derive(Debug)]
pub struct TrainingMessage {
    pub message: String,
    pub iteration: u32,
    pub epoch: u32,
    pub batch_start: u32,
    pub metrics: Option<Metrics>,
}

impl Json for TrainingMessage {
    fn to_json(&self, pretty: bool) -> String {
        let mut v = json!({
          "message": self.message,
          "epoch" : self.epoch,
          "iteration": self.iteration,
          "batch_start": self.batch_start
        });

        let map = v.as_object_mut().unwrap();

        match &self.metrics {
            Some(metrics) => {
                map.insert("loss".to_string(), json!(metrics.loss));
                map.insert("train_eval".to_string(),
                           json!({
                                  "confusion_matrix" : json!({
                                     "data": json!(metrics.train_eval.confusion_matrix.data.as_vec()),
                                     "predictions_on_cols" : json!(true),
                                     "data_orientation_per_col" : json!(true),
                                     "dimension" : json!(metrics.train_eval.confusion_matrix_dim)
                                  }),
                                  "label_accuracies" : json!(metrics.train_eval.labels_accuracies),
                                  "accuracy" : json!(metrics.train_eval.accuracy),
                                  }));
                map.insert("test_eval".to_string(),
                           json!({
                                  "confusion_matrix" : json!({
                                     "data": json!(metrics.test_eval.confusion_matrix.data.as_vec()),
                                     "predictions_on_cols" : json!(true),
                                     "data_orientation_per_col" : json!(true),
                                     "dimension" : json!(metrics.test_eval.confusion_matrix_dim)
                                  }),
                                  "label_accuracies" : json!(metrics.test_eval.labels_accuracies),
                                   "accuracy" : json!(metrics.test_eval.accuracy),
                                  }));
            }
            _ => (),
        }


        if pretty {
            serde_json::to_string_pretty(&v).unwrap()
        } else {
            v.to_string()
        }
    }
}


impl Default for TrainingMessage {
    fn default() -> Self {
        TrainingMessage {
            message: "".to_string(),
            iteration: 0,
            epoch: 0,
            batch_start: 0,
            metrics: None,
        }
    }
}


pub trait TrainingObserver {
    fn emit(&self, msg: TrainingMessage);
}

pub struct ConsoleObserver;

impl TrainingObserver for ConsoleObserver {
    fn emit(&self, msg: TrainingMessage) {
        println!("{}", msg.to_json(false));
        match msg.metrics {
            Some(m) => {
                println!("\t loss {}", m.loss);
                println!("\t train accuracy {}", m.train_eval.accuracy);
                println!("\t test accuracy {}", m.test_eval.accuracy);
            }
            _ => ()
        }
    }
}

pub trait Train {
    fn train(&mut self,
             hp: HyperParams,
             observer: &dyn TrainingObserver,
             train_data: LabeledData,
             test_data: LabeledData) -> Result<NNModel, Box<dyn std::error::Error>>;
}

pub trait Prediction {
    fn predict(&mut self, data: &DMatrix<f32>) -> DMatrix<f32>;
}


#[derive(Clone)]
pub struct Layer {
    pub num_activations: usize,
    pub intercepts: DVector<f32>,
    pub weights: DMatrix<f32>,
    pub activation_type: ActivationType,

    // W * X + B
    pub z: DVector<f32>,
    // activation(z)
    pub a: DVector<f32>,
    pub dz: DVector<f32>,
    pub dw: DMatrix<f32>,
    pub db: DVector<f32>,
    pub momentum_dw: DMatrix<f32>,
    pub momentum_db: DVector<f32>,
    pub rmsp_dw: DMatrix<f32>,
    pub rmsp_db: DVector<f32>,

}


#[derive(Debug)]
pub struct LayerDefinition {
    pub activation_type: ActivationType,
    pub num_activations: usize,
}

#[derive(Debug)]
pub struct NeuralNetworkArchitecture<R: RandomInitializer> {
    pub num_features: usize,
    pub num_classes: usize,
    pub layers: Vec<LayerDefinition>,
    pub rand_initializer: R,
}

pub trait RandomInitializer {
    fn weights(self, r: usize, c: usize, rng: &mut rand_pcg::Pcg32) -> DMatrix<f32>;
}

#[derive(Debug)]
pub struct HeUniform;

#[derive(Debug)]
pub struct GlorothUniform;


impl RandomInitializer for HeUniform {
    fn weights(self, r: usize, c: usize, rng: &mut rand_pcg::Pcg32) -> DMatrix<f32> {
        let normal = Normal::new(0.0, 1.0).unwrap();

        let factor = (2.0 / c as f32).sqrt();

        let v: Vec<f32> = (0..(r * c)).map(|_| {
            let r = normal.sample(rng) as f32;
            r * factor
        }).collect();
        let m = DMatrix::from_vec(r, c, v);
        m
    }
}

impl RandomInitializer for GlorothUniform {
    fn weights(self, r: usize, c: usize, rng: &mut rand_pcg::Pcg32) -> DMatrix<f32> {
        let normal = Normal::new(0.0, 1.0).unwrap();

        let factor = (6.0 / r as f32 + c as f32).sqrt();

        let v: Vec<f32> = (0..(r * c)).map(|_| {
            let r = normal.sample(rng) as f32;
            r * factor
        }).collect();
        let m = DMatrix::from_vec(r, c, v);
        m
    }
}


impl Copy for HeUniform {}

impl Clone for HeUniform {
    fn clone(&self) -> Self {
        HeUniform {}
    }
}

impl std::fmt::Debug for ActivationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActivationType::Sigmoid => write!(f, "Sigmoid"),
            ActivationType::Relu => write!(f, "Relu"),
            ActivationType::Tanh => write!(f, "Tanh"),
            ActivationType::SoftMax => write!(f, "SoftMax"),
        }
    }
}


impl Clone for ActivationType {
    fn clone(&self) -> ActivationType {
        match self {
            ActivationType::Sigmoid => ActivationType::Sigmoid,
            ActivationType::Relu => ActivationType::Relu,
            ActivationType::Tanh => ActivationType::Tanh,
            ActivationType::SoftMax => ActivationType::SoftMax
        }
    }
}


impl std::fmt::Debug for Layer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\n\tLayer {{
        \t\tnum_activations : {:?}
        \t\tweights : {:?}
        \t\tactivation_type : {:?}
    }}", self.num_activations, self.weights.shape(), self.activation_type)
    }
}