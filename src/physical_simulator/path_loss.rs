/*
Path loss: L=10n\log _{10}(d)+C
L: path loss in decibels (dB)
d: distance between the transmitter and receiver
n: path loss exponent
C: a constant that depends on the environment
*/

#[derive(Clone, Copy, Debug)]
pub struct Position {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Position {
    pub fn distance(&self, other: &Position) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2) + (self.z - other.z).powi(2)).sqrt()
    }
}


#[derive(Default)]
pub enum PathLossModel {
    #[default]
    FreeSpace,

    Urban,
    Suburban,
    Rural,
}

impl PathLossModel {
    pub fn get_path_loss_constant(&self) -> f64 {
        match self {
            PathLossModel::FreeSpace => 20.0,
            PathLossModel::Urban => 35.0,
            PathLossModel::Suburban => 40.0,
            PathLossModel::Rural => 45.0,
        }
    }

    pub fn get_path_loss_exponent(&self) -> f64 {
        match self {
            PathLossModel::FreeSpace => 2.0,
            PathLossModel::Urban => 2.7,
            PathLossModel::Suburban => 3.0,
            PathLossModel::Rural => 3.5,
        }
    }
}