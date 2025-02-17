/*
Path loss: L=10n\log _{10}(d)+C
L: path loss in decibels (dB)
d: distance between the transmitter and receiver
n: path loss exponent
C: a constant that depends on the environment
*/

use rand::Rng;


#[derive(Default, Debug, Clone, Copy)]
pub enum PathLossModel {
    #[default]
    FreeSpace,
    LogDistanceNormalShadowing
}




impl PathLossModel {
    fn normal(mean: f64, sd: f64) -> f64 {
        let mut rng = rand::thread_rng();
        let u1: f64 = rng.gen();
        let u2: f64 = rng.gen();
        ((-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()) as f64 * sd + mean
    }
    
    pub fn get_path_loss(&self, distance: f64, frequency: f64) -> f32 {
        match self {
            PathLossModel::FreeSpace => (20.0 * distance.log10() + 20.0 * frequency.log10() - 147.55) as f32,
            PathLossModel::LogDistanceNormalShadowing => {
                //from Do LoRa Low-Power Wide-Area Networks Scale?
                let d0 = 40.0;
                let gamma: f64 = 2.08; 
                let sigma: f64 = 3.57;
                //let pl_d0_db: f64 = 127.41;
                
                let pl_d0_db: f64 = 87.41; //custom value to better fit the simulation

                //classic formula
                (pl_d0_db + 10.0 * gamma * (distance / d0).log10() + Self::normal(0.0, sigma)) as f32
            },
            
        }
    }
}


#[test]
fn test_path_loss() {
    let frequency = 868_000_000.0;
    
    let path_loss = PathLossModel::LogDistanceNormalShadowing;
 
    let distance = 1.0;
    let pl = path_loss.get_path_loss(distance, frequency);
    println!("Path loss: {}", pl);
 
    let distance = 500.0;
    let pl = path_loss.get_path_loss(distance, frequency);
    println!("Path loss: {}", pl);
    
    let distance = 1000.0;
    let pl = path_loss.get_path_loss(distance, frequency);
    println!("Path loss: {}", pl);

    let distance = 2000.0;
    let pl = path_loss.get_path_loss(distance, frequency);
    println!("Path loss: {}", pl);
    
    let distance = 3000.0;
    let pl = path_loss.get_path_loss(distance, frequency);
    println!("Path loss: {}", pl);
}