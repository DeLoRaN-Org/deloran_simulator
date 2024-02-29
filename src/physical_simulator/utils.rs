use super::world::Transmission;
use lorawan::physical_parameters::SpreadingFactor;

fn dbmw2mw(dbm: f64) -> f64 {
    // Conversion formula: P(mW) = 1mW * 10^(P(dBm)/10)
    10f64.powf(dbm / 10.0)
}


//function returns sensitivity -- according to LoRa documentation, it changes with LoRa parameters
//Sensitivity values from Semtech SX1272/73 datasheet, table 10, Rev 3.1, March 2017
fn get_sensitivity(transmission: Transmission) -> f64 {
    let mut sensitivity = dbmw2mw(-126.5) / 1000.0;
    let sf = transmission.spreading_factor;
    let bw = transmission.bandwidth as u32;

    match sf {
        SpreadingFactor::SF7 => match bw {
            125000 => sensitivity = dbmw2mw(-124.0) / 1000.0,
            250000 => sensitivity = dbmw2mw(-122.0) / 1000.0,
            500000 => sensitivity = dbmw2mw(-116.0) / 1000.0,
            _ => (),
        },
        SpreadingFactor::SF8 => match bw {
            125000 => sensitivity = dbmw2mw(-127.0) / 1000.0,
            250000 => sensitivity = dbmw2mw(-125.0) / 1000.0,
            500000 => sensitivity = dbmw2mw(-119.0) / 1000.0,
            _ => (),
        },
        SpreadingFactor::SF9 => match bw {
            125000 => sensitivity = dbmw2mw(-130.0) / 1000.0,
            250000 => sensitivity = dbmw2mw(-128.0) / 1000.0,
            500000 => sensitivity = dbmw2mw(-122.0) / 1000.0,
            _ => (),
        },
        SpreadingFactor::SF10 => match bw {
            125000 => sensitivity = dbmw2mw(-133.0) / 1000.0,
            250000 => sensitivity = dbmw2mw(-130.0) / 1000.0,
            500000 => sensitivity = dbmw2mw(-125.0) / 1000.0,
            _ => (),
        },
        SpreadingFactor::SF11 => match bw {
            125000 => sensitivity = dbmw2mw(-135.0) / 1000.0,
            250000 => sensitivity = dbmw2mw(-132.0) / 1000.0,
            500000 => sensitivity = dbmw2mw(-128.0) / 1000.0,
            _ => (),
        },
        SpreadingFactor::SF12 => match bw {
            125000 => sensitivity = dbmw2mw(-137.0) / 1000.0,
            250000 => sensitivity = dbmw2mw(-135.0) / 1000.0,
            500000 => sensitivity = dbmw2mw(-129.0) / 1000.0,
            _ => (),
        },
    }
    sensitivity
}