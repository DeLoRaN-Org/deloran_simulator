#![allow(dead_code,unused)]

use std::{
    fs,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::Arc,
    time::Duration,
};

use deloran_simulator::{
    chirpstack::{ChirpstackActivation, ChirpstackDevice, ChirpstackListDeviceAns, DeviceAns},
    physical_simulator::{
        chirpstack_bridge::ChirpstackBridgeConfig,
        network_controller_bridge::NetworkControllerBridgeConfig,
        node::{NodeConfig, NodeState},
        path_loss::PathLossModel,
        world::{World, WorldConfig},
    }, traffic_models::{REGULAR_TRAFFIC_DISTRIBUTION, UNREGULAR_TRAFFIC_DISTRIBUTION},
};
use lazy_static::lazy_static;
use lorawan::{
    device::{session_context::{ApplicationSessionContext, NetworkSessionContext, SessionContext}, Device, DeviceClass, LoRaWANVersion},
    encryption::key::Key,
    physical_parameters::{CodeRate, DataRate, LoRaBandwidth, SpreadingFactor},
    regional_parameters::region::{Region, RegionalParameters},
    utils::eui::EUI64,
};
use lorawan_device::{communicator::Position, configs::RadioDeviceConfig};

#[allow(unused)]
use rand::distributions::Distribution;

use tokio::sync::Mutex;

use deloran_simulator::constants::*;

lazy_static! {
    pub static ref RADIO_PARAMETERS: Vec<(SpreadingFactor, LoRaBandwidth, f32)> = {
        let mut vec = Vec::new();
        for sf in [
            SpreadingFactor::SF7,
            //SpreadingFactor::SF8,
            //SpreadingFactor::SF9,
        ] {
            for bw in [
                LoRaBandwidth::BW125,
                //LoRaBandwidth::BW250,
                //LoRaBandwidth::BW500,
            ] {
                for freq in [
                    868_100_000.0,
                    868_300_000.0,
                    //868_500_000.0,
                    //867_100_000.0,
                    //867_300_000.0,
                    //867_500_000.0,
                    //867_700_000.0,
                    //867_900_000.0,
                ] {
                    vec.push((sf, bw, freq))
                }
            }
        }
        vec
    };
}

//lazy_static! {
//    pub static ref RADIO_PARAMETERS: Vec<(SpreadingFactor, LoRaBandwidth, f32)> = {
//        let f: [f32; 8] = [868_100_000.0, 868_300_000.0, 868_500_000.0, 867_100_000.0, 867_300_000.0, 867_500_000.0, 867_700_000.0, 867_900_000.0];
//        let vec = Vec::from([(SpreadingFactor::SF7, LoRaBandwidth::BW125),(SpreadingFactor::SF8, LoRaBandwidth::BW125),(SpreadingFactor::SF9, LoRaBandwidth::BW125)]);
//
//        let mut v = Vec::new();
//        for freq in f.iter() {
//            for (sf, bw) in vec.iter() {
//                v.push((*sf, *bw, *freq));
//            }
//        }
//        v
//    };
//}

fn make_device_config(
    position: Position,
    sf: SpreadingFactor,
    freq: f32,
    bandwidth: LoRaBandwidth,
) -> NodeConfig {
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
            code_rate: CodeRate::CR4_5,
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
                spreading_factor: Default::default(), //not important
                data_rate: DataRate::DR5,
                bandwidth: Default::default(), //not important
                freq: 0.0,                     //not important
                sample_rate: 1.0,
                rx_chan_id: 1,
                tx_chan_id: 1,
                code_rate: CodeRate::CR4_5,
            },
        },
    }
}

fn make_chirpstack_config(gwid: &'static str, position: Position) -> ChirpstackBridgeConfig {
    ChirpstackBridgeConfig {
        gwid,
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
                spreading_factor: Default::default(), //not important
                data_rate: DataRate::DR5,
                bandwidth: Default::default(), //not important
                freq: 0.0,                     //not important
                sample_rate: 1.0,
                rx_chan_id: 1,
                tx_chan_id: 1,
                code_rate: CodeRate::CR4_5,
            },
        },
    }
}

fn random_position_between(x1: f32, x2: f32, y1: f32, y2: f32, z1: f32, z2: f32) -> Position {
    Position {
        x: rand::random::<f32>() * (x2 - x1) + x1,
        y: rand::random::<f32>() * (y2 - y1) + y1,
        z: rand::random::<f32>() * (z2 - z1) + z1,
    }
}

async fn chirpstack_create_device_configuration(d: &DeviceAns) -> Device {
    let client = reqwest::Client::new();
    let ans = client.get(format!("http://169.254.189.196:8090/api/devices/{}/keys", d.devEui))
        .header("Accept", "application/json")
        //.header("Grpc-Metadata-Authorization", "Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJhdWQiOiJjaGlycHN0YWNrIiwiaXNzIjoiY2hpcnBzdGFjayIsInN1YiI6ImFlYzhmNzE5LWI0Y2MtNDNhYi05ZWEyLWQ0YWZmYWY3MzNlYSIsInR5cCI6ImtleSJ9.Rx5-zIhjZSeUPCEqFfZkjll7acfjc-4cyFOLPnrNPS8")
        .header("Grpc-Metadata-Authorization", "Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJhdWQiOiJjaGlycHN0YWNrIiwiaXNzIjoiY2hpcnBzdGFjayIsInN1YiI6Ijk5ZjdjYjE0LTk0ODQtNDExMS05OGI0LTU2MGM3YzZmZTQ4ZSIsInR5cCI6ImtleSJ9.Nrbo22pgioYA9IGrQ6EnGTVjbfo0RNcJxMRrpG7LO-8")
        .send().await.unwrap();

    let text = ans.text().await.unwrap();
    //println!("{text}");
    let device: ChirpstackDevice = serde_json::from_str(&text).unwrap();
    //println!("{device:#?}");

    let dev_eui = EUI64::from_hex(&d.devEui).unwrap();
    let join_eui = EUI64::default();
    let nwk_key = Key::from_hex(&device.deviceKeys.nwkKey).unwrap();
    let app_key = Key::from_hex(&device.deviceKeys.appKey).unwrap();

    Device::new(
        DeviceClass::A,
        Some(RegionalParameters::new(Region::EU863_870)),
        dev_eui,
        join_eui,
        nwk_key,
        app_key,
        LoRaWANVersion::V1_0_4,
    )
}

async fn chirpstack_get_device_session(d: &DeviceAns) -> SessionContext {
    let client = reqwest::Client::new();

    let ans = client.get(format!("http://169.254.189.196:8090/api/devices/{}/activation", d.devEui))
        .header("Accept", "application/json")
        //.header("Grpc-Metadata-Authorization", "Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJhdWQiOiJjaGlycHN0YWNrIiwiaXNzIjoiY2hpcnBzdGFjayIsInN1YiI6ImFlYzhmNzE5LWI0Y2MtNDNhYi05ZWEyLWQ0YWZmYWY3MzNlYSIsInR5cCI6ImtleSJ9.Rx5-zIhjZSeUPCEqFfZkjll7acfjc-4cyFOLPnrNPS8")
        .header("Grpc-Metadata-Authorization", "Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJhdWQiOiJjaGlycHN0YWNrIiwiaXNzIjoiY2hpcnBzdGFjayIsInN1YiI6Ijk5ZjdjYjE0LTk0ODQtNDExMS05OGI0LTU2MGM3YzZmZTQ4ZSIsInR5cCI6ImtleSJ9.Nrbo22pgioYA9IGrQ6EnGTVjbfo0RNcJxMRrpG7LO-8")
        .send().await.unwrap();

    let text = ans.text().await.unwrap();
    //println!("{text}");
    let activation: ChirpstackActivation = serde_json::from_str(&text).unwrap();

    let fnwk_s_int_key = Key::from_hex(&activation.deviceActivation.fNwkSIntKey).unwrap();
    let snwk_s_int_key = Key::from_hex(&activation.deviceActivation.sNwkSIntKey).unwrap();
    let nwk_s_enc_key = Key::from_hex(&activation.deviceActivation.nwkSEncKey).unwrap();
    let home_net_id = [1,2,3];
    let dev_addr: [u8; 4] = u32::from_str_radix(&activation.deviceActivation.devAddr, 16).unwrap().to_be_bytes();

    let f_cnt_up = activation.deviceActivation.fCntUp;
    let nf_cnt_dwn = activation.deviceActivation.nFCntDown;

    let network_session = NetworkSessionContext::new(fnwk_s_int_key, snwk_s_int_key, nwk_s_enc_key, home_net_id, dev_addr, f_cnt_up, nf_cnt_dwn, 0);

    let app_s_key = Key::from_hex(&activation.deviceActivation.appSKey).unwrap();
    let af_cnt_dwn = activation.deviceActivation.aFCntDown;
    let application_session = ApplicationSessionContext::new(app_s_key, af_cnt_dwn);

    SessionContext::new(application_session, network_session)
}

async fn chirpstack_load_devices(ncs: &[&ChirpstackBridgeConfig], w: &mut World) {
    let client = reqwest::Client::new();
    let ans = client.get("http://169.254.189.196:8090/api/devices")
        .query(&[
            //("applicationId","17272d19-e169-49a4-82e7-fa8ae17439ad"),
            //("applicationId","b8d129fe-1d37-4944-beed-ad500e00aa95"),
            ("applicationId","7980e124-1ee8-4907-8ffb-bf10a93e3cc7"),
            ("limit", &format!("{}", NUM_DEVICES)),
        ])
        .header("Accept", "application/json")
        .header("Grpc-Metadata-Authorization", "Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJhdWQiOiJjaGlycHN0YWNrIiwiaXNzIjoiY2hpcnBzdGFjayIsInN1YiI6Ijk5ZjdjYjE0LTk0ODQtNDExMS05OGI0LTU2MGM3YzZmZTQ4ZSIsInR5cCI6ImtleSJ9.Nrbo22pgioYA9IGrQ6EnGTVjbfo0RNcJxMRrpG7LO-8")
        //.header("Grpc-Metadata-Authorization", "Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJhdWQiOiJjaGlycHN0YWNrIiwiaXNzIjoiY2hpcnBzdGFjayIsInN1YiI6IjE0NzA3ZWQ2LTU4YzYtNDdkMS04OWQ0LTgzNjRiMjkzMDllYSIsInR5cCI6ImtleSJ9.B7m6iadZRCfr5mH7v5V1ig79GVr8X8Aw7RpboovZ7ow")
        //.header("applicationId", "52f14cd4-c6f1-4fbd-8f87-4025e1d49242")
        //.header("tenantId", "917c0850-0ae2-4a1b-b716-f6df37bb732b")
        .send().await.unwrap();

    let ans = &ans.text().await.unwrap();
    //println!("{}", ans);
    let content: ChirpstackListDeviceAns = serde_json::from_str(ans).unwrap();

    for (i, dev) in content.result.iter().enumerate() {
        
        let mut d = chirpstack_create_device_configuration(dev).await;
        let context = chirpstack_get_device_session(dev).await;
        d.set_activation_abp(context);

        let assigned_nc = &ncs[i % ncs.len()];
        let position = random_position_between(
            assigned_nc.node_config.position.x,
            assigned_nc.node_config.position.x + 50.0,
            assigned_nc.node_config.position.y,
            assigned_nc.node_config.position.y + 50.0,
            0.0,
            10.0,
        );

        let (sf, bw, freq) = RADIO_PARAMETERS[i % RADIO_PARAMETERS.len()];

        //w.add_node(d, make_device_config(position, sf, freq, bw), rand::random::<f32>() < 0.86);
        w.add_node(d, make_device_config(position, sf, freq, bw), true);
    }
}

async fn chirpstack_main() -> World {
    let path_loss = PathLossModel::LogDistanceNormalShadowing;
    let config = WorldConfig {
        path_loss_model: path_loss,
    };
    let mut w = World::new(config);
    let nc1 = make_chirpstack_config("00f58d99c1c10a74", Position { x: 300.0,      y:-300.0,      z: 100.0 });
    let nc2 = make_chirpstack_config("1779428d4a420632", Position { x: 300.0,      y: 300.0,      z: 100.0 });
    let nc3 = make_chirpstack_config("30e6d7f20802991a", Position { x:-300.0,      y:-300.0,      z: 100.0 });
    let nc4 = make_chirpstack_config("57aaa1005f4b068e", Position { x:-300.0,      y: 300.0,      z: 100.0 });

    let ncs = [&nc1, &nc2, &nc3, &nc4];
    chirpstack_load_devices(&ncs, &mut w).await;

    w.add_chirpstack_gw(nc1);
    w.add_chirpstack_gw(nc2);
    w.add_chirpstack_gw(nc3);
    w.add_chirpstack_gw(nc4);

    w
}

async fn deloran_main() -> World {
    let path_loss = PathLossModel::LogDistanceNormalShadowing;
    let config = WorldConfig {
        path_loss_model: path_loss,
    };
    let mut w = World::new(config);

    let nc1 = make_nc_config(
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10,111,77,109), 9090)),
        Position {
            x: 100.0,
            y: -100.0,
            z: 100.0,
        },
    );
    let nc2 = make_nc_config(
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 207, 19, 234), 9090)),
        Position {
            x: 100.0,
            y: 100.0,
            z: 100.0,
        },
    );
    let nc3 = make_nc_config(
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 207, 19, 196), 9090)),
        Position {
            x: -100.0,
            y: -100.0,
            z: 100.0,
        },
    );
    let nc4 = make_nc_config(
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 207, 19, 70), 9090)),
        Position {
            x: -100.0,
            y: 100.0,
            z: 100.0,
        },
    );
    let ncs = [&nc1, &nc2, &nc3, &nc4];

    let file_content: String = fs::read_to_string("./node_sessions_loed.txt").unwrap();
    file_content.split('\n').take(NUM_DEVICES).enumerate().for_each(|(i, line)| {
            let mut d: Device = serde_json::from_str(line).unwrap();
            d.session_mut().expect("Session should be there thanks to node_sessions.txt").network_context_mut().update_f_cnt_up(STARTING_FCNT_UP);
            let assigned_nc = &ncs[i % ncs.len()];  
            let position = random_position_between(
                assigned_nc.node_config.position.x,
                assigned_nc.node_config.position.x + 500.0,
                assigned_nc.node_config.position.y,
                assigned_nc.node_config.position.y + 500.0,
                0.0,
                10.0,
            );
    
            let (sf, bw, freq) = RADIO_PARAMETERS[i % RADIO_PARAMETERS.len()];
    
            //w.add_node(d, make_device_config(position, sf, freq, bw), rand::random::<f32>() < 0.86);
            w.add_node(d, make_device_config(position, sf, freq, bw), true);
    });

    w.add_network_controller(nc1);
    w.add_network_controller(nc2);
    w.add_network_controller(nc3);
    w.add_network_controller(nc4);

    w
}



#[tokio::main]
async fn main() {
    let path_loss = PathLossModel::LogDistanceNormalShadowing;
    //let mut w = chirpstack_main().await;
    let mut w = deloran_main().await;

    /*let file_content = fs::read_to_string("./devices_augmented.csv").unwrap();
    file_content.split('\n').take(NUM_DEVICES).enumerate().for_each(|(i, line)| {
            let splitted = line.split(',').collect::<Vec<&str>>();
            let dev_eui = EUI64::from_hex(splitted[0]).unwrap();
            let join_eui = EUI64::from_hex(splitted[1]).unwrap();
            let key = Key::from_hex(splitted[2]).unwrap();

            let d = Device::new(
                DeviceClass::A,
                Some(RegionalParameters::new(Region::EU863_870)),
                dev_eui,
                join_eui,
                key,
                key,
                LoRaWANVersion::V1_0_4,
            );

            let assigned_nc = &ncs[i % ncs.len()];

            let position = random_position_between(
                assigned_nc.node_config.position.x,
                assigned_nc.node_config.position.x + 500.0,
                assigned_nc.node_config.position.y,
                assigned_nc.node_config.position.y + 500.0,
                0.0,
                10.0,
            );

            let (sf, bw, freq) = RADIO_PARAMETERS[i % RADIO_PARAMETERS.len()];

            //w.add_node(d, make_device_config(position, sf, freq, bw), rand::random::<f32>() < 0.86);
            w.add_node(d, make_device_config(position, sf, freq, bw), true);
    });*/

    let duration = 30000;

    println!("PARAMETERS: ");
    println!("Number of devices: {}", w.node_counter());
    println!("Network controllers: {}", w.nc_counter());
    println!("Duration: {duration} seconds");
    println!("Path loss model: {:?}", path_loss);
    println!("NUM_PACKETS: {NUM_PACKETS}");
    println!("RANDOM_JOIN_DELAY: {RANDOM_JOIN_DELAY}");
    println!("FIXED_JOIN_DELAY: {FIXED_JOIN_DELAY}");
    println!("FIXED_PACKET_DELAY: {FIXED_PACKET_DELAY}");
    println!("RANDOM_PACKET_DELAY: {RANDOM_PACKET_DELAY}");
    println!("CONFIRMED_AVERAGE_SEND: {_CONFIRMED_AVERAGE_SEND}");
    println!("STARTING_DEV_NONCE: {STARTING_DEV_NONCE}");

    println!("Traffic regular mean: {}",REGULAR_TRAFFIC_DISTRIBUTION.mean());
    println!("Traffic regular std deviation: {}",REGULAR_TRAFFIC_DISTRIBUTION.variance().sqrt());
    println!("Traffic unregular mean: {}",UNREGULAR_TRAFFIC_DISTRIBUTION.mean());
    println!("Traffic unregular std deviation: {}",UNREGULAR_TRAFFIC_DISTRIBUTION.variance().sqrt());

    println!("Simulation of {duration}s starting in 5 seconds...");
    for i in (0..5).rev() {
        println!("{i}...");
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    //w.run(Some(Duration::from_secs(duration))).await;
    w.run(None).await;
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
        } else if v > 1500.0 {
            more_than_1500 += 1;
        } else {
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