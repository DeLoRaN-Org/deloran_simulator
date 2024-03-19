use rand::Rng;
use rand::distributions::Distribution;

// Define your custom distribution
pub struct LoEDDistribution {
    values: Vec<f64>,
    probabilities: Vec<f64>,
}

impl LoEDDistribution {
    pub fn mean(&self) -> f64 {
        let mut sum = 0.0;
        for (&value, &probability) in self.values.iter().zip(&self.probabilities) {
            // Assuming the values are numeric for simplicity
            // Convert the value to a numeric type if necessary
            sum += value * probability;
        }
        sum
    }

    // Calculate the variance of the distribution
    pub fn variance(&self) -> f64 {
        let mean = self.mean();
        let mut sum_of_squares = 0.0;
        for (value, &probability) in self.values.iter().zip(&self.probabilities) {
            // Assuming the values are numeric for simplicity
            // Convert the value to a numeric type if necessary
            let diff = value - mean;
            sum_of_squares += diff * diff * probability;
        }
        sum_of_squares
    }

    pub fn standard_deviation(&self) -> f64 {
        self.variance().sqrt()
    }
}

impl Default for LoEDDistribution {
    fn default() -> Self {
        let content = std::fs::read_to_string("./loed_traffic_distribution.csv").unwrap();
        let mut values = Vec::new();
        let mut probabilities = Vec::new();

        content.split('\n').for_each(|line| {
            let splitted = line.split('\t').collect::<Vec<&str>>();
            if splitted.len() == 2 {
                values.push(splitted[0].parse::<f64>().unwrap());
                probabilities.push(splitted[1].parse::<f64>().unwrap());
            }
        });

        assert_eq!(values.len(), probabilities.len(), "Values and probabilities must have the same length");
        let total: f64 = probabilities.iter().sum();
        assert!((total - 1.0).abs() < 1e-9, "Probabilities must sum to 1");
        LoEDDistribution { values, probabilities }
    }
}

impl Distribution<f64> for LoEDDistribution {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> f64 {
        let mut cumulative_probability = 0.0;
        let random_value = rng.gen_range(0.0..1.0);
        for (&value, &probability) in self.values.iter().zip(&self.probabilities) {
            cumulative_probability += probability;
            if random_value < cumulative_probability {
                return value;
            }
        }
        panic!("Failed to sample from custom distribution, means that the sum of probabilities is not 1.");
    }
}

#[test]
fn test() {
    let mut rng = rand::thread_rng();
    let distribution = LoEDDistribution::default();
    distribution.sample(&mut rng);
    
    println!("Mean: {}", distribution.mean());
    println!("Variance: {}", distribution.variance());
    println!("Standard deviation: {}", distribution.standard_deviation());
}