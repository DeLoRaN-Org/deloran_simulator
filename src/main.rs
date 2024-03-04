use std::fs;

use deloran_simulator::physical_simulator::{node::NodeConfig, path_loss::PathLossModel, world::World};
use lorawan::{device::{Device, DeviceClass, LoRaWANVersion}, encryption::key::Key, physical_parameters::{CodeRate, DataRate, LoRaBandwidth, SpreadingFactor}, regional_parameters::region::{Region, RegionalParameters}, utils::eui::EUI64};
use lorawan_device::{communicator::Position, configs::RadioDeviceConfig};

fn main() {

    let mut w = World::new(PathLossModel::FreeSpace);

    let file_content = fs::read_to_string("./devices_augmented.csv").unwrap();
    file_content.split('\n').take(2).enumerate().for_each(|(_i, line)| {
        let splitted = line.split(',').collect::<Vec<&str>>();
        let dev_eui = EUI64::from_hex(splitted[0]).unwrap();
        let join_eui = EUI64::from_hex(splitted[1]).unwrap();
        let key = Key::from_hex(splitted[2]).unwrap();
        

        let d = Device::new(DeviceClass::A, Some(RegionalParameters::new(Region::EU863_870)), dev_eui, join_eui, key, key, LoRaWANVersion::V1_0_4);
        let c = NodeConfig {
            position: Position {
                x: 0.0,
                y: 0.0,
                z: 0.0
            },
            transmission_power_dbm: 14.0,
            receiver_sensitivity: -120.0,
            tx_consumption: 0.0,
            rx_consumption: 0.0,
            idle_consumption: 0.0,
            sleep_consumption: 0.0,
            radio_config: RadioDeviceConfig {
                region: Region::EU863_870,
                spreading_factor: SpreadingFactor::SF7,
                data_rate: DataRate::DR5,
                bandwidth: LoRaBandwidth::BW125,
                rx_freq: 868_000_000.0,
                tx_freq: 868_000_000.0,
                sample_rate: 1.0,
                rx_chan_id: 1,
                tx_chan_id: 1,
                code_rate: CodeRate::CR4_5
            },
        };
        w.add_node(d, c);
    });
}