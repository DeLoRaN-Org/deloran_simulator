use std::{fs, net::{Ipv4Addr, SocketAddr, SocketAddrV4}, sync::Arc, time::Duration};

use deloran_simulator::physical_simulator::{network_controller_bridge::NetworkControllerBridgeConfig, node::{NodeConfig, NodeState}, path_loss::PathLossModel, world::{World, REGULAR_TRAFFIC_DISTRIBUTION, UNREGULAR_TRAFFIC_DISTRIBUTION}};
use lazy_static::lazy_static;
use lorawan::{device::{Device, DeviceClass, LoRaWANVersion}, encryption::key::Key, physical_parameters::{CodeRate, DataRate, LoRaBandwidth, SpreadingFactor}, regional_parameters::region::{Region, RegionalParameters}, utils::eui::EUI64};
use lorawan_device::{communicator::Position, configs::RadioDeviceConfig};

#[allow(unused)]
use rand::distributions::Distribution;

use tokio::sync::Mutex;

use deloran_simulator::constants::*;

lazy_static! {
    pub static ref RADIO_PARAMETERS: Vec<(SpreadingFactor, LoRaBandwidth, f32)> = {
        let mut vec = Vec::new();
        for sf in [SpreadingFactor::SF7, SpreadingFactor::SF8, SpreadingFactor::SF9] {
            for bw in [LoRaBandwidth::BW125, LoRaBandwidth::BW250, LoRaBandwidth::BW500] {
                for freq in [868_100_000.0, 868_300_000.0, 868_500_000.0, 867_100_000.0, 867_300_000.0, 867_500_000.0, 867_700_000.0, 867_900_000.0] {
                    vec.push((sf, bw, freq))
                }
            }
        }
        vec
    };
}


#[test]
fn foo() {
    let mut zero_to_100 = 0;
    let mut hundred_to_200 = 0;
    let mut two_hundred_to_300 = 0;
    let mut three_hundred_to_400 = 0;
    let mut four_hundred_to_500 = 0;
    let mut more_than_1500 = 0;
    let mut other = 0;

    for _ in 0..10000 {
        let v = UNREGULAR_TRAFFIC_DISTRIBUTION.sample(&mut rand::thread_rng());
        if v < 100.0 {
            zero_to_100 += 1;
        } else if v < 200.0 {
            hundred_to_200 += 1;
        } else if v < 300.0 {
            two_hundred_to_300 += 1;
        } else if v < 400.0 {
            three_hundred_to_400 += 1;
        } else if v < 500.0 {
            four_hundred_to_500 += 1;
        }
        else if v > 1500.0 {
            more_than_1500 += 1;
        }
        else {
            other += 1;
        }
    }

    println!("0-100: {}", zero_to_100);
    println!("100-200: {}", hundred_to_200);
    println!("200-300: {}", two_hundred_to_300);
    println!("300-400: {}", three_hundred_to_400);
    println!("400-500: {}", four_hundred_to_500);
    println!("More than 1500: {}", more_than_1500);
    println!("Other: {}", other)
}


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

    let path_loss = PathLossModel::LogDistanceNormalShadowing;
    let mut w = World::new(path_loss);

    let nc1 = make_nc_config(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 207, 19, 155), 9090)), Position { x: 100000.0,      y:-100000.0,      z: 100.0 });
    let nc2 = make_nc_config(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 207, 19, 20 ), 9090)), Position { x: 100000.0,      y: 100000.0,      z: 100.0 });
    let nc3 = make_nc_config(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 207, 19, 81 ), 9090)), Position { x:-100000.0,      y:-100000.0,      z: 100.0 });
    let nc4 = make_nc_config(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 207, 19, 223), 9090)), Position { x:-100000.0,      y: 100000.0,      z: 100.0 });
    
    //let mut base_frequency = 868_000_000.0;
    //let channels = [
    //    868_100_000.0,
    //    868_300_000.0,
    //    868_500_000.0,
    //    867_100_000.0,
    //    867_300_000.0,
    //    867_500_000.0,
    //    867_700_000.0,
    //    867_900_000.0,
    //];

    let ncs = [&nc1, &nc2, &nc3, &nc4];

    let file_content = fs::read_to_string("./devices_augmented.csv").unwrap();
    file_content.split('\n').take(NUM_DEVICES).enumerate().for_each(|(i, line)| {
        let splitted = line.split(',').collect::<Vec<&str>>();
        let dev_eui = EUI64::from_hex(splitted[0]).unwrap();
        let join_eui = EUI64::from_hex(splitted[1]).unwrap();
        let key = Key::from_hex(splitted[2]).unwrap();
        
        let d = Device::new(DeviceClass::A, Some(RegionalParameters::new(Region::EU863_870)), dev_eui, join_eui, key, key, LoRaWANVersion::V1_0_4);
        
        let assigned_nc = &ncs[i % ncs.len()];

        let position = random_position_between(assigned_nc.node_config.position.x, assigned_nc.node_config.position.x + 500.0, assigned_nc.node_config.position.y, assigned_nc.node_config.position.y + 500.0, 0.0, 10.0);
        
        let (sf, bw, freq) = RADIO_PARAMETERS[i % RADIO_PARAMETERS.len()];

        //w.add_node(d, make_device_config(position, sf, freq, bw), rand::random::<f32>() < 0.86);
        w.add_node(d, make_device_config(position, sf, freq, bw), true);
    });
    
    //{
    //    let ncs = [&nc1, &nc2, &nc3, &nc4];
    //
    //    let file_content = fs::read_to_string("./devices_complete.csv").unwrap();
    //    file_content.split('\n').take(NUM_DEVICES).enumerate().for_each(|(i, line)| {
    //        let (left, right) = line.split_once(',').unwrap();
    //        let mut d = serde_json::from_str::<Device>(right).unwrap();       
    //        d.session_mut().unwrap().network_context_mut().update_f_cnt_up(STARTING_FCNT_UP);     
    //        let assigned_nc = ncs[i % ncs.len()];
    //        let position = random_position_between(assigned_nc.node_config.position.x, assigned_nc.node_config.position.x + 10000.0, assigned_nc.node_config.position.y, assigned_nc.node_config.position.y + 10000.0, 0.0, 100.0);
    //        let (sf, bw, freq) = RADIO_PARAMETERS[i % RADIO_PARAMETERS.len()];
    //
    //        w.add_node(d, make_device_config(position, sf, freq, bw));
    //    });
    //}

    w.add_network_controller(nc1);
    w.add_network_controller(nc2);
    w.add_network_controller(nc3);
    w.add_network_controller(nc4);

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
    
    println!("Traffic regular mean: {}", REGULAR_TRAFFIC_DISTRIBUTION.mean());
    println!("Traffic regular std deviation: {}", REGULAR_TRAFFIC_DISTRIBUTION.variance().sqrt());
    println!("Traffic unregular mean: {}", UNREGULAR_TRAFFIC_DISTRIBUTION.mean());
    println!("Traffic unregular std deviation: {}", UNREGULAR_TRAFFIC_DISTRIBUTION.variance().sqrt());


    println!("Simulation of {duration}s starting in 5 seconds...");
    for i in (0..5).rev() {
        println!("{i}...");
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    
    //w.run(Some(Duration::from_secs(duration))).await;
    w.run(None).await;
}