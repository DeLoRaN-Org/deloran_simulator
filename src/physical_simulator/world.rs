use std::{sync::Arc, time::{SystemTime, UNIX_EPOCH}};

use lorawan::{device::Device, physical_parameters::{CodeRate, DataRate, SpreadingFactor}, regional_parameters::region::Region};
use lorawan_device::{configs::RadioDeviceConfig, devices::lorawan_device::LoRaWANDevice};
use tokio::sync::{mpsc::{Receiver, Sender}, Mutex};

use super::{node::{Node, NodeCommunicator, NodeConfig}, path_loss::{PathLossModel, Position}};


const SPEED_OF_LIGHT: f64 = 299_792_458.0;

const NODE_CONFIG: NodeConfig = NodeConfig {
    position: Position {
        x: 0.0, 
        y: 0.0, 
        z: 0.0
    },
    transmission_power_dbm: 0.0,
    receiver_sensitivity: -120.0,
    tx_consumption: 0.0,
    rx_consumption: 0.0,
    idle_consumption: 0.0,
    sleep_consumption: 0.0,
    radio_config: RadioDeviceConfig {
        region: Region::EU863_870,
        spreading_factor: SpreadingFactor::SF7,
        data_rate: DataRate::DR5,
        rx_gain: 10,
        tx_gain: 20,
        bandwidth: 125_000.0,
        sample_rate: 1_000_000.0,
        rx_freq: 990_000_000.0,
        tx_freq: 1_010_000_000.0,
        rx_chan_id: 0,
        tx_chan_id: 1,
        code_rate: CodeRate::CR4_5
    }
};

#[derive(Debug, Clone, Copy)]
pub struct ArrivalStats {
    pub time: u128,
    pub rssi: f32,
    pub snr: f32,
}

#[derive(Debug, Clone)]
pub struct Transmission {
    pub start_position: Position,
    pub start_time: u128,
    pub frequency: f32,
    pub bandwidth: f32,
    pub spreading_factor: SpreadingFactor,
    pub coding_rate: CodeRate,
    pub collided: bool,
    pub starting_power: f32,

    pub payload: Vec<u8>,
}

impl Transmission {
    //https://github.com/avbentem/airtime-calculator/blob/master/doc/LoraDesignGuide_STD.pdf
    pub fn time_on_air(&self) -> u128 {
        let header_disabled = 0_u32; // implicit header disabled (H=0) or not (H=1), can only have implicit header with SF6
        let mut data_rate_optimization = 0_u32; // low data rate optimization enabled (=1) or not (=0)
        if self.bandwidth == 125.0 && (self.spreading_factor == SpreadingFactor::SF11 || self.spreading_factor == SpreadingFactor::SF12) {
            data_rate_optimization = 1; // low data rate optimization mandated for BW125 with SF11 and SF12
        }

        let npream = 8_u32; // number of preamble symbol (12.25 from Utz paper)
        let tsym = ((2.0f32).powi(self.spreading_factor.value() as i32) / (self.bandwidth * 1000.0)) * 1000.0;
        let tpream = (npream as f32 + 4.25) * tsym;

        let cr = match self.coding_rate {
            CodeRate::CR4_5 => 5,
            CodeRate::CR4_6 => 6,
            CodeRate::CR5_7 => 7,
            CodeRate::CR4_8 => 8,
        } - 4;


        let v1 = ((8 * (self.payload.len()) - 4 * (self.spreading_factor.value() as usize) + 44 - 20 * header_disabled as usize)  //28 + 16 = 44(? -->     payloadSymbNB = 8 + max(math.ceil((8.0*pl-4.0*sf+28+16-20*H)/(4.0*(sf-2*DE)))*(cr+4),0))
            / (4 * ((self.spreading_factor.value() as usize) - 2 * data_rate_optimization as usize))) * (cr + 4);
        let payload_symb_nb = 8 + (if v1 > 0 { v1 } else { 0 });
        let tpayload = (payload_symb_nb as f32) * tsym;
        (tpream + tpayload).round() as u128
    }

    pub fn ended(&self) -> bool {
        World::get_milliseconds_from_epoch() > self.start_time + self.time_on_air()
    }
}


#[derive(Debug, Clone)]
pub struct ReceivedTransmission {
    pub transmission: Transmission,
    pub arrival_stats: ArrivalStats,
}

impl ReceivedTransmission {
    pub fn time_on_air(&self) -> u128 {
        self.transmission.time_on_air()
    }
}


pub struct World {
    path_loss_model: PathLossModel,
    epochs: u64,

    nodes: Vec<Node>,
    transmissions_on_air: Arc<Mutex<Vec<Transmission>>>,
 
    sender: Sender<Transmission>,
}

impl World {
    pub fn new(path_loss_model: PathLossModel) -> World {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(100);
        let transmissions_on_air = Arc::new(Mutex::new(Vec::new()));
        let toac = transmissions_on_air.clone();

        tokio::spawn(async move {
            loop {
                let transmission = receiver.recv().await.unwrap();
                println!("Received transmission: {:?}", transmission);
                toac.lock().await.push(transmission);
            }
        });
        
        World {
            nodes: Vec::new(),
            epochs: 0,
            path_loss_model,
            transmissions_on_air,
            sender,
        }
    }

    fn get_milliseconds_from_epoch() -> u128 {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()
    }

    pub fn add_node(&mut self, device: Device) {
        let node_communicator = NodeCommunicator::new(self.sender.clone(),NodeConfig {
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
                spreading_factor: SpreadingFactor::new(7),
                data_rate: DataRate::new(0),
                rx_gain: 0,
                tx_gain: 0,
                bandwidth: 125_000.0,
                rx_freq: 868000000.0,
                tx_freq: 868000000.0,
                sample_rate: 1.0,
                rx_chan_id: 1,
                tx_chan_id: 1,
                code_rate: CodeRate::CR4_5
            },
        });

        let node = LoRaWANDevice::new(device, node_communicator);
        self.nodes.push(Node::new(node));
    }

    pub fn get_nodes(&self) -> &Vec<Node> {
        &self.nodes
    }

    pub fn get_nodes_mut(&mut self) -> &mut Vec<Node> {
        &mut self.nodes
    }

    pub fn get_epochs(&self) -> u64 {
        self.epochs
    }

    pub fn path_loss_model(&self) -> &PathLossModel {
        &self.path_loss_model
    }

    pub async fn tick(&mut self) {
        self.epochs += 1;
        self.check_collisions().await;
        for node in self.nodes.iter_mut() {
            node.tick().await;
        }
    }

    /*
    Collision checks taken from:
    repo: https://github.com/mcbor/lorasim
    repo: https://github/florasim/flora

    file: https://github.com/mcbor/lorasim/blob/main/loraDir.py
    file: https://github/florasim/flora/blob/main/src/LoRaPhy/LoRaReceiver.cc
    */

    fn timing_collision(t1: &Transmission, t2: &Transmission) -> bool {
        let t1_toa = t1.time_on_air();
        let t2_toa = t2.time_on_air();
        t1.start_time > t2.start_time && t1.start_time < (t2.start_time + t2_toa) ||
        t2.start_time > t1.start_time && t2.start_time < (t1.start_time + t1_toa)
    }

    fn bandwidth_collision(t1: &Transmission, t2: &Transmission) -> bool {
        if t1.frequency == 500_000.0 || t2.frequency == 500_000.0 {
            (t1.frequency - t2.frequency).abs() <= 120_000.0
        } else if t1.frequency == 250_000.0 || t2.frequency == 250_000.0 {
            (t1.frequency - t2.frequency).abs() <= 60_000.0
        } else {
            (t1.frequency - t2.frequency).abs() <= 30_000.0
        }
    }

    fn sf_collision(t1: &Transmission, t2: &Transmission) -> bool {
        t1.spreading_factor == t2.spreading_factor
    }

    fn power_collision(t1_rssi: f32, t2_rssi: f32) -> bool {
        let power_threshold = 6.0;  //dB, it is hardcoded both in lorasim and flora
        (t1_rssi - t2_rssi).abs() < power_threshold || t1_rssi - t2_rssi < power_threshold
    }

    async fn check_collisions(&mut self) {
        let mut transmissions_on_air = self.transmissions_on_air.lock().await;
        for i in  0..transmissions_on_air.len() {
            let t1 = &transmissions_on_air[i];
            if t1.ended() { //if transmission ended
                for j in i + 1..transmissions_on_air.len() {

                    let t2 = &transmissions_on_air[j];
                    if World::timing_collision(t1, t2) && World::bandwidth_collision(t1, t2) && World::sf_collision(t1, t2) {
                        for node in self.nodes.iter_mut() {
                            let t1_rssi = t1.starting_power - self.path_loss_model.get_path_loss(node.get_position().distance(&t1.start_position));
                            let t2_rssi = t2.starting_power - self.path_loss_model.get_path_loss(node.get_position().distance(&t2.start_position));
                            let t1_power_collision = World::power_collision(t1_rssi, t2_rssi);
                            if !t1_power_collision {
                                let t1_rx: ReceivedTransmission = ReceivedTransmission {
                                    transmission: t1.clone(),
                                    arrival_stats: ArrivalStats {
                                        time: World::get_milliseconds_from_epoch(),
                                        rssi: t1_rssi,
                                        snr: 0.0,
                                    }
                                };
                                node.communicator_mut().receive_transmission(t1_rx);
                            }
                        }
                    }
                }
            }
        }
        transmissions_on_air.retain(|t| !t.ended());
    }
}