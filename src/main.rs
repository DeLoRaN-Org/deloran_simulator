use std::{fs, net::{Ipv4Addr, SocketAddr, SocketAddrV4}, sync::Arc, time::Duration};

use deloran_simulator::physical_simulator::{network_controller_bridge::NetworkControllerBridgeConfig, node::{NodeConfig, NodeState}, path_loss::PathLossModel, world::{World, NUM_DEVICES}};
use lorawan::{device::{Device, DeviceClass, LoRaWANVersion}, encryption::key::Key, physical_parameters::{CodeRate, DataRate, LoRaBandwidth, SpreadingFactor}, regional_parameters::region::{Region, RegionalParameters}, utils::eui::EUI64};
use lorawan_device::{communicator::Position, configs::RadioDeviceConfig};

use deloran_simulator::physical_simulator::world::{NUM_PACKETS, RANDOM_JOIN_DELAY, FIXED_JOIN_DELAY, FIXED_PACKET_DELAY, RANDOM_PACKET_DELAY, _CONFIRMED_AVERAGE_SEND, STARTING_DEV_NONCE};
use tokio::sync::{Mutex, RwLock};

fn make_device_config(position: Position, sf: SpreadingFactor, freq: f32, bandwidth: LoRaBandwidth) -> NodeConfig {
    NodeConfig {
        position,
        transmission_power_dbm: 14.0,
        receiver_sensitivity: -120.0,
        tx_consumption: 0.0,
        rx_consumption: 0.0,
        idle_consumption: 0.0,
        sleep_consumption: 0.0,
        node_state: Arc::new(Mutex::new(NodeState::Idle)),
        radio_config: RadioDeviceConfig {
            region: Region::EU863_870,
            spreading_factor: sf,
            data_rate: DataRate::DR5,
            bandwidth,
            freq,
            sample_rate: 1.0,
            rx_chan_id: 1,
            tx_chan_id: 1,
            code_rate: CodeRate::CR4_5
        },
    }
}

fn make_nc_config(nc_addr: SocketAddr, position: Position) -> NetworkControllerBridgeConfig {
    NetworkControllerBridgeConfig {
        network_controller_address: nc_addr, 
        node_config: NodeConfig {
            position,
            transmission_power_dbm: 14.0,
            receiver_sensitivity: -120.0,
            tx_consumption: 0.0,
            rx_consumption: 0.0,
            idle_consumption: 0.0,
            sleep_consumption: 0.0,
            node_state: Arc::new(Mutex::new(NodeState::Receiving)),
            radio_config: RadioDeviceConfig {
                region: Region::EU863_870,
                spreading_factor: Default::default(),   //not important
                data_rate: DataRate::DR5,
                bandwidth: Default::default(),          //not important
                freq: 0.0,                              //not important
                sample_rate: 1.0,
                rx_chan_id: 1,
                tx_chan_id: 1,
                code_rate: CodeRate::CR4_5
            },
        }
    }
}

fn random_position_between(x1: f32, x2: f32, y1: f32, y2: f32, z1: f32, z2: f32) -> Position {
    Position {
        x: rand::random::<f32>() * (x2 - x1) + x1,
        y: rand::random::<f32>() * (y2 - y1) + y1,
        z: rand::random::<f32>() * (z2 - z1) + z1,
    }
}

#[tokio::main]
async fn main() {

    let path_loss = PathLossModel::FreeSpace;
    let mut w = World::new(path_loss);

    //let mut base_frequency = 868_000_000.0;

    let channels = [
        868_100_000.0,
        868_300_000.0,
        868_500_000.0,
        867_100_000.0,
        867_300_000.0,
        867_500_000.0,
        867_700_000.0,
        867_900_000.0,
    ];

    let file_content = fs::read_to_string("./devices_augmented.csv").unwrap();
    file_content.split('\n').take(NUM_DEVICES).enumerate().for_each(|(i, line)| {
        let splitted = line.split(',').collect::<Vec<&str>>();
        let dev_eui = EUI64::from_hex(splitted[0]).unwrap();
        let join_eui = EUI64::from_hex(splitted[1]).unwrap();
        let key = Key::from_hex(splitted[2]).unwrap();
        
        let d = Device::new(DeviceClass::A, Some(RegionalParameters::new(Region::EU863_870)), dev_eui, join_eui, key, key, LoRaWANVersion::V1_0_4);
        let position = random_position_between(0.0, 10000.0, 0.0, 10000.0, 0.0, 100.0);

        
        let frequency = channels[i % channels.len()];
        w.add_node(d, make_device_config(position, SpreadingFactor::SF7, frequency, LoRaBandwidth::BW125));
    });

    w.add_network_controller(make_nc_config(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 207, 19, 155), 9090)), Position { x: 1000.0,      y:-1000.0,      z: 100.0 }));
    w.add_network_controller(make_nc_config(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 207, 19, 20 ), 9090)), Position { x: 1000.0,      y: 1000.0,      z: 100.0 }));
    w.add_network_controller(make_nc_config(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 207, 19, 81 ), 9090)), Position { x:-1000.0,      y:-1000.0,      z: 100.0 }));
    w.add_network_controller(make_nc_config(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 207, 19, 223), 9090)), Position { x:-1000.0,      y: 1000.0,      z: 100.0 }));

    let duration = 30000;

    println!("PARAMETERS: ");
    println!("Number of devices: {}", w.node_counter());
    println!("Network controllers: {}", w.nc_counter());
    println!("Duration: {duration} seconds");
    println!("Path loss model: {path_loss:?}");
    println!("NUM_PACKETS: {NUM_PACKETS}");
    println!("RANDOM_JOIN_DELAY: {RANDOM_JOIN_DELAY}");
    println!("FIXED_JOIN_DELAY: {FIXED_JOIN_DELAY}");
    println!("FIXED_PACKET_DELAY: {FIXED_PACKET_DELAY}");
    println!("RANDOM_PACKET_DELAY: {RANDOM_PACKET_DELAY}");
    println!("CONFIRMED_AVERAGE_SEND: {_CONFIRMED_AVERAGE_SEND}");
    println!("STARTING_DEV_NONCE: {STARTING_DEV_NONCE}");


    println!("Simulation of {duration}s starting in 5 seconds...");
    for i in (0..5).rev() {
        println!("{i}...");
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    
    //w.run(Some(Duration::from_secs(duration))).await;
    w.run(None).await;
}