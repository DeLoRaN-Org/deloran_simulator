use lorawan::physical_parameters::{LoRaBandwidth, SpreadingFactor};
use lorawan_device::communicator::Transmission;

//fn dbmw2mw(dbm: f64) -> f64 {
//    // Conversion formula: P(mW) = 1mW * 10^(P(dBm)/10)
//    10f64.powf(dbm / 10.0)
//}


//function returns sensitivity -- according to LoRa documentation, it changes with LoRa parameters
//Sensitivity values from Semtech SX1272/73 datasheet, table 10, Rev 3.1, March 2017
pub fn get_sensitivity(transmission: &Transmission) -> f32 {
    let sensitivity;
    let sf = transmission.spreading_factor;
    let bw = transmission.bandwidth;

    match sf {
        SpreadingFactor::SF7 => match bw {
            LoRaBandwidth::BW125 => sensitivity = -124.0,
            LoRaBandwidth::BW250 => sensitivity = -122.0,
            LoRaBandwidth::BW500 => sensitivity = -116.0,
        },
        SpreadingFactor::SF8 => match bw {
            LoRaBandwidth::BW125 => sensitivity = -127.0,
            LoRaBandwidth::BW250 => sensitivity = -125.0,
            LoRaBandwidth::BW500 => sensitivity = -119.0,
        },
        SpreadingFactor::SF9 => match bw {
            LoRaBandwidth::BW125 => sensitivity = -130.0,
            LoRaBandwidth::BW250 => sensitivity = -128.0,
            LoRaBandwidth::BW500 => sensitivity = -122.0,
        },
        SpreadingFactor::SF10 => match bw {
            LoRaBandwidth::BW125 => sensitivity = -133.0,
            LoRaBandwidth::BW250 => sensitivity = -130.0,
            LoRaBandwidth::BW500 => sensitivity = -125.0,
        },
        SpreadingFactor::SF11 => match bw {
            LoRaBandwidth::BW125 => sensitivity = -135.0,
            LoRaBandwidth::BW250 => sensitivity = -132.0,
            LoRaBandwidth::BW500 => sensitivity = -128.0,
        },
        SpreadingFactor::SF12 => match bw {
            LoRaBandwidth::BW125 => sensitivity = -137.0,
            LoRaBandwidth::BW250 => sensitivity = -135.0,
            LoRaBandwidth::BW500 => sensitivity = -129.0,
        },
    }
    sensitivity //-120.0
}