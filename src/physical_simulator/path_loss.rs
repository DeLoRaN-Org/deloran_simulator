/*
Path loss: L=10n\log _{10}(d)+C
L: path loss in decibels (dB)
d: distance between the transmitter and receiver
n: path loss exponent
C: a constant that depends on the environment
*/


#[derive(Default, Debug, Clone, Copy)]
pub enum PathLossModel {
    #[default]
    FreeSpace,
    Urban,
    Suburban,
    CentralUrban,
}




impl PathLossModel {

    pub fn path_loss_exponent(&self) -> f32 {
        match self {
            PathLossModel::FreeSpace => 2.0,
            PathLossModel::Suburban => 3.0,
            PathLossModel::Urban => 4.0,
            PathLossModel::CentralUrban => 5.0,
        }
    }

    pub fn get_path_loss(&self, distance: f32, frequency: f32) -> f32 {
        match self {
            PathLossModel::FreeSpace => 20.0 * distance.log10() + 20.0 * frequency.log10() - 147.55,
            PathLossModel::Urban |
            PathLossModel::Suburban |
            PathLossModel::CentralUrban => todo!() /*PLd0 + 10*self.path_loss_exponent()*(distance / d0) + (random_value for shadowning? forse 0)*/,
        }
    }
}