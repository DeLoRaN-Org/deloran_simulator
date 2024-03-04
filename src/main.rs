use std::fs;

use deloran_simulator::physical_simulator::{path_loss::PathLossModel, world::World};
use lorawan::{device::{Device, DeviceClass, LoRaWANVersion}, encryption::key::Key, regional_parameters::region::{Region, RegionalParameters}, utils::eui::EUI64};
use lorawan_device::communicator::Position;

fn main() {

    let mut w = World::new(PathLossModel::FreeSpace);

    let file_content = fs::read_to_string("./devices_augmented.csv").unwrap();
    file_content.split('\n').take(10).enumerate().for_each(|(_i, line)| {
        let splitted = line.split(',').collect::<Vec<&str>>();
        let dev_eui = EUI64::from_hex(splitted[0]).unwrap();
        let join_eui = EUI64::from_hex(splitted[1]).unwrap();
        let key = Key::from_hex(splitted[2]).unwrap();
        

        let d = Device::new(DeviceClass::A, Some(RegionalParameters::new(Region::EU863_870)), dev_eui, join_eui, key, key, LoRaWANVersion::V1_0_4);
        w.add_node(d, Position { x: 0.0, y: 0.0, z: 0.0 });
    });
}