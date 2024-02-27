use lorawan::{device::Device, physical_parameters::CodeRate};
use lorawan_device::devices::lorawan_device::LoRaWANDevice;
use tokio::sync::mpsc::{Receiver, Sender};

use super::{node::{Node, NodeCommunicator, NodeConfig}, path_loss::{PathLossModel, Position}};


const SPEED_OF_LIGHT: f64 = 299_792_458.0;

const SF7:  [f64; 4] = [7.0,-126.5,-124.25,-120.75];
const SF8:  [f64; 4] = [8.0,-127.25,-126.75,-124.0];
const SF9:  [f64; 4] = [9.0,-131.25,-128.25,-127.5];
const SF10: [f64; 4] = [10.0,-132.75,-130.25,-128.75];
const SF11: [f64; 4] = [11.0,-134.5,-132.75,-128.75];
const SF12: [f64; 4] = [12.0,-133.25,-132.25,-132.25];

const NODE_CONFIG: NodeConfig = NodeConfig {
    position: Position {
        x: 0.0, 
        y: 0.0, 
        z: 0.0
    },
    transmission_power_dbm: 14.0,
    path_loss_exponent: 2.0,
    constant: 20.0,
    tx_consumption: 0.0,
    rx_consumption: 0.0,
    idle_consumption: 0.0,
    sleep_consumption: 0.0,
};

/*
    Collision checks taken from:
    repo: https://github.com/mcbor/lorasim
    file: https://github.com/mcbor/lorasim/blob/main/loraDir.py
*/

pub struct ArrivalStats {
    pub time: u128,
    pub rssi: f64,
    pub snr: f64,
    pub collided: bool,
}

pub struct Transmission {
    pub start_position: Position,
    pub start_time: u128,
    pub frequency: f64,
    pub bandwidth: f64,
    pub spreading_factor: u8,
    pub coding_rate: CodeRate,
    pub payload: Vec<u8>,

    pub arrival_stats: Option<ArrivalStats>,
}


impl Transmission {
    //https://github.com/avbentem/airtime-calculator/blob/master/doc/LoraDesignGuide_STD.pdf
    fn time_on_air(&self) -> u128 {
        let mut header_disabled = 0_u32; // implicit header disabled (H=0) or not (H=1), can only have implicit header with SF6
        let mut data_rate_optimization = 0_u32; // low data rate optimization enabled (=1) or not (=0)
        if self.bandwidth == 125.0 && (self.spreading_factor == 11 || self.spreading_factor == 12) {
            data_rate_optimization = 1; // low data rate optimization mandated for BW125 with SF11 and SF12
        }

        let npream = 8_u32; // number of preamble symbol (12.25 from Utz paper)
        let tsym = ((2.0f64).powi(self.spreading_factor as i32) / (self.bandwidth * 1000.0)) * 1000.0;
        let tpream = (npream as f64 + 4.25) * tsym;

        let cr = match self.coding_rate {
            CodeRate::CR4_5 => 5,
            CodeRate::CR4_6 => 6,
            CodeRate::CR5_7 => 7,
            CodeRate::CR4_8 => 8,
        } - 4;


        let v1 = ((8 * (self.payload.len()) - 4 * (self.spreading_factor as usize) + 44 - 20 * header_disabled as usize)  //28 + 16 = 44(? -->     payloadSymbNB = 8 + max(math.ceil((8.0*pl-4.0*sf+28+16-20*H)/(4.0*(sf-2*DE)))*(cr+4),0))
            / (4 * ((self.spreading_factor as usize) - 2 * data_rate_optimization as usize))) * (cr + 4);
        let payload_symb_nb = 8 + (if v1 > 0 { v1 } else { 0 });
        let tpayload = (payload_symb_nb as f64) * tsym;
        (tpream + tpayload).round() as u128
    }

}

pub struct World {
    path_loss_model: PathLossModel,
    epochs: u64,

    nodes: Vec<Node>,
    transmissions: Vec<Transmission>,
    sender: Sender<Transmission>,


    receiver: Receiver<Transmission>,
}

impl World {
    pub fn new(nodes: Vec<Node>, path_loss_model: PathLossModel) -> World {
        let (sender, receiver) = tokio::sync::mpsc::channel(100);
        World {
            nodes,
            epochs: 0,
            path_loss_model,
            transmissions: Vec::new(),
            sender,
            receiver
        }
    }

    pub fn add_node(&mut self, device: Device) {
        let node_communicator = NodeCommunicator::new(self.sender.clone(),NodeConfig {
            position: Position {
                x: 0.0, 
                y: 0.0, 
                z: 0.0
            },
            transmission_power_dbm: 14.0,
            path_loss_exponent: 2.0,
            constant: 20.0,
            tx_consumption: 0.0,
            rx_consumption: 0.0,
            idle_consumption: 0.0,
            sleep_consumption: 0.0,
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

    fn bandwidth_collision(&self, t1: &Transmission, t2: &Transmission) -> bool {
        if t1.frequency == 500.0 || t2.frequency == 500.0 {
            (t1.frequency - t2.frequency).abs() <= 120.0
        } else if t1.frequency == 250.0 || t2.frequency == 250.0 {
            (t1.frequency - t2.frequency).abs() <= 60.0
        } else {
            (t1.frequency - t2.frequency).abs() <= 30.0
        }
    }

    fn sf_collision(&self, t1: &Transmission, t2: &Transmission) -> bool {
        t1.spreading_factor == t2.spreading_factor
    }

    fn power_collision(&self, t1: &Transmission, t2: &Transmission) -> (bool, bool) {
        let power_threshold = 6.0;  //dB

        //TODO togliere unwrap
        if (t1.arrival_stats.unwrap().rssi - t2.arrival_stats.unwrap().rssi).abs() < power_threshold {
            (true, true)
        } else if t1.arrival_stats.unwrap().rssi - t2.arrival_stats.unwrap().rssi < power_threshold {
            (true, false)
        } else {
            (false, true)
        }
    }

    fn check_collisions(&mut self) {
        for i in  0..self.transmissions.len() {
            for j in i +  1..self.transmissions.len() {
                let t1 = &self.transmissions[i];
                let t2 = &self.transmissions[j];
    
                let t1_toa = t1.time_on_air();
                let t2_toa = t2.time_on_air();

                if t1.start_time < (t2.start_time + t2_toa) && t2.start_time < (t1.start_time + t1_toa) { //time overlap
                    if t1.frequency == t2.frequency { //frequency overlap
                        if t1.spreading_factor == t2.spreading_factor { // spreading factor collision
                            //TODO togliere unwrap
                            self.transmissions[i].arrival_stats.unwrap().collided = true;
                            self.transmissions[j].arrival_stats.unwrap().collided = true;
                        }
                    }
                }
            }
        }
    }

    pub fn tick(&mut self) {
        self.epochs += 1;
        self.check_collisions();
        for node in self.nodes.iter_mut() {
            node.tick();
        }
    }

}