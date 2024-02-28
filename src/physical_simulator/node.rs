use std::{collections::HashMap, ops::{Deref, DerefMut}, time::{Duration, SystemTime}};

use lorawan::{physical_parameters::SpreadingFactor, utils::eui::EUI64};
use lorawan_device::{communicator::{CommunicatorError, LoRaPacket, LoRaWANCommunicator}, configs::RadioDeviceConfig, devices::lorawan_device::LoRaWANDevice};
use tokio::{sync::mpsc::Sender, time::Instant};

use super::{path_loss::Position, world::{ReceivedTransmission, Transmission}};


#[derive(Clone, Debug, Copy)]
pub struct NodeConfig {
    pub position: Position,

    pub transmission_power_dbm: f32,  //14 dbm for indoor devices and 27dbm for outdoor devices
    pub receiver_sensitivity: f32,    //-137 dbm for SF12 and 125kHz --> generato automaticamente lol      

    pub tx_consumption: f32,
    pub rx_consumption: f32,
    pub idle_consumption: f32,
    pub sleep_consumption: f32,

    pub radio_config: RadioDeviceConfig,
}

#[derive(Clone, Debug, Copy)]
pub enum NodeState {
    Idle,
    Sleep,
    Transmitting,
    Receiving,
}

#[derive(Debug)]
pub struct Node {
    pub device: LoRaWANDevice<NodeCommunicator>,
    pub received_transmissions: Vec<ReceivedTransmission>,
}

impl Node {
    pub fn new(device: LoRaWANDevice<NodeCommunicator>) -> Node {
        Node {
            device,
            received_transmissions: Vec::new(),
        }
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

    fn power_collision(&self, t1: &ReceivedTransmission, t2: &ReceivedTransmission) -> (bool, bool) {
        let power_threshold = 6.0;  //dB

        //TODO togliere unwrap
        if (t1.arrival_stats.rssi - t2.arrival_stats.rssi).abs() < power_threshold {
            (true, true)
        } else if t1.arrival_stats.rssi - t2.arrival_stats.rssi < power_threshold {
            (true, false)
        } else {
            (false, true)
        }
    }

    fn check_collisions(&mut self) {
        for i in  0..self.received_transmissions.len() {
            for j in i +  1..self.received_transmissions.len() {
                let t1 = &self.received_transmissions[i];
                let t2 = &self.received_transmissions[j];
    
                let t1_toa = t1.time_on_air();
                let t2_toa = t2.time_on_air();

                if t1.transmission.start_time < (t2.transmission.start_time + t2_toa) && t2.transmission.start_time < (t1.transmission.start_time + t1_toa) { //time overlap
                    if t1.transmission.frequency == t2.transmission.frequency { //frequency overlap
                        if t1.transmission.spreading_factor == t2.transmission.spreading_factor { // spreading factor collision
                            //TODO togliere unwrap
                            self.received_transmissions[i].arrival_stats.collided = true;
                            self.received_transmissions[j].arrival_stats.collided = true;
                        }
                    }
                }
            }
        }
    }

    pub fn tick(&mut self) {

    }
}

 
impl Deref for Node {
    type Target = LoRaWANDevice<NodeCommunicator>;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

impl DerefMut for Node {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.device
    }
}


pub struct NodeCommunicator {
    sender: Sender<Transmission>,

    config: NodeConfig,
    state: NodeState,
    last_status_change: Instant,

    tx_time: Duration,
    rx_time: Duration,
    idle_time: Duration,
    sleep_time: Duration,
}

impl NodeCommunicator {

    pub fn new(sender: Sender<Transmission>, config: NodeConfig) -> NodeCommunicator {
        NodeCommunicator {
            sender,
            config,
            state: NodeState::Idle,
            last_status_change: Instant::now(),
            tx_time: Duration::from_secs(0),
            rx_time: Duration::from_secs(0),
            idle_time: Duration::from_secs(0),
            sleep_time: Duration::from_secs(0),
        }
    }

    pub fn calculate_signal_strength(&self, distance: f32, path_loss_exponent: f32, constant: f32) -> f32 {
        self.config.transmission_power_dbm - 10.0 * path_loss_exponent * distance.log10() - constant
    }

    pub fn calculate_energy_consumption(&self, duration: Duration) -> f32 {
        let seconds = duration.as_secs_f32();
        let energy_consumption = match self.state {
            NodeState::Idle => self.config.idle_consumption,
            NodeState::Sleep => self.config.sleep_consumption,
            NodeState::Transmitting => self.config.tx_consumption,
            NodeState::Receiving => self.config.rx_consumption,
        };
        energy_consumption * seconds
    }

    pub fn change_state(&mut self, new_state: NodeState) {
        let now = Instant::now();
        let duration = now.duration_since(self.last_status_change);
        match self.state {
            NodeState::Idle => self.idle_time += duration,
            NodeState::Sleep => self.sleep_time += duration,
            NodeState::Transmitting => self.tx_time += duration,
            NodeState::Receiving => self.rx_time += duration,
        }
        self.state = new_state;
        self.last_status_change = Instant::now();
    }
}


#[async_trait::async_trait]
impl LoRaWANCommunicator for NodeCommunicator {
    type Config=NodeConfig;

    async fn from_config(_config: &Self::Config) -> Result<Self, CommunicatorError> {
        todo!()
    }
    
    async fn send_uplink(
        &self,
        bytes: &[u8],
        _src: Option<EUI64>,
        _dest: Option<EUI64>,
    ) -> Result<(), CommunicatorError> {
        self.sender.send(Transmission {
            start_position: self.config.position,
            start_time: SystemTime::UNIX_EPOCH.elapsed().unwrap().as_millis(),
            frequency: self.config.radio_config.tx_freq,
            bandwidth: self.config.radio_config.bandwidth,
            spreading_factor: self.config.radio_config.spreading_factor,
            coding_rate: self.config.radio_config.code_rate,
            payload: bytes.to_vec(),
        }).await.map_err(|_| CommunicatorError::Radio("Error sending message to channel".to_owned()))
    }

    async fn receive_downlink(
        &self,
        _timeout: Option<Duration>,
    ) -> Result<HashMap<SpreadingFactor, LoRaPacket>, CommunicatorError> {
        todo!()
    }
}