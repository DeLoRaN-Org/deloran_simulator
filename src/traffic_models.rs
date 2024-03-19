use core::fmt;
use std::fmt::{Debug, Formatter};
use std::path::Path;

use rand::Rng;
use rand::distributions::Distribution;

// Define your custom distribution
pub struct TrafficDistribution {
    name: String,
    values: Vec<f64>,
    probabilities: Vec<f64>,
}

impl Debug for TrafficDistribution {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "CustomDistribution: {}", self.name)
    }
}

impl TrafficDistribution {
    pub fn new<T>(path: T, name: String) -> Self
    where T: AsRef<Path> {
        let content = std::fs::read_to_string(path).unwrap();
        let mut values = Vec::new();
        let mut probabilities = Vec::new();

        content.split('\n').for_each(|line| {
            let splitted = line.split(',').collect::<Vec<&str>>();
            if splitted.len() == 2 {
                values.push(splitted[0].parse::<f64>().unwrap());
                probabilities.push(splitted[1].parse::<f64>().unwrap());
            }
        });

        assert_eq!(values.len(), probabilities.len(), "Values and probabilities must have the same length");
        let total: f64 = probabilities.iter().sum();
        assert!((total - 1.0).abs() < 1e-9, "Probabilities must sum to 1");
        TrafficDistribution { values, probabilities, name }
    
    }

    pub fn mean(&self) -> f64 {
        let mut sum = 0.0;
        for (&value, &probability) in self.values.iter().zip(&self.probabilities) {
            sum += value * probability;
        }
        sum
    }

    pub fn variance(&self) -> f64 {
        let mean = self.mean();
        let mut sum_of_squares = 0.0;
        for (value, &probability) in self.values.iter().zip(&self.probabilities) {
            let diff = value - mean;
            sum_of_squares += diff * diff * probability;
        }
        sum_of_squares
    }

    pub fn standard_deviation(&self) -> f64 {
        self.variance().sqrt()
    }
}

impl Distribution<f64> for TrafficDistribution {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> f64 {
        let mut cumulative_probability = 0.0;
        let random_value = rng.gen_range(0.0..1.0);
        for (&value, &probability) in self.values.iter().zip(&self.probabilities) {
            cumulative_probability += probability;
            if random_value < cumulative_probability {
                return value;
            }
        }
        panic!("Failed to sample from loed distribution, means that the sum of probabilities is not 1.");
    }
}

#[derive(Debug)]
pub enum TrafficModel {
    Custom(TrafficDistribution),
    Periodic(f64),
}


impl Distribution<f64> for TrafficModel {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> f64 {
        match self {
            TrafficModel::Custom(distribution) => distribution.sample(rng),
            TrafficModel::Periodic(period) => *period,
        }
    }
}