use std::{collections::HashMap, ops::{Deref, DerefMut}, time::{Duration, Instant, SystemTime}};

use lorawan::{physical_parameters::SpreadingFactor, utils::eui::EUI64};
use lorawan_device::{communicator::{CommunicatorError, LoRaPacket, LoRaWANCommunicator}, configs::RadioDeviceConfig, devices::lorawan_device::LoRaWANDevice};
use tokio::sync::mpsc::Sender;

use super::{path_loss::Position, world::Transmission};


#[derive(Clone, Debug, Copy)]
pub struct NodeConfig {
    pub position: Position,

    pub transmission_power_dbm: f64,  //14 dbm for indoor devices and 27dbm for outdoor devices
    pub path_loss_exponent: f64,
    pub constant: f64,                

    pub tx_consumption: f64,
    pub rx_consumption: f64,
    pub idle_consumption: f64,
    pub sleep_consumption: f64,

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
}

impl Node {
    pub fn new(device: LoRaWANDevice<NodeCommunicator>) -> Node {
        Node {
            device,
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

    pub fn calculate_signal_strength(&self, distance: f64, path_loss_exponent: f64, constant: f64) -> f64 {
        self.config.transmission_power_dbm - 10.0 * path_loss_exponent * distance.log10() - constant
    }

    pub fn calculate_energy_consumption(&self, duration: Duration) -> f64 {
        let seconds = duration.as_secs_f64();
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
        _bytes: &[u8],
        _src: Option<EUI64>,
        _dest: Option<EUI64>,
    ) -> Result<(), CommunicatorError> {
        self.sender.send(Transmission {
            start_position: self.config.position,
            start_time: SystemTime::UNIX_EPOCH.elapsed().unwrap().as_millis(),
            frequency: self.config.,
            bandwidth: self.config.freq,
            spreading_factor: self.config.freq,
            coding_rate: self.config.freq,
            payload: self.config.freq,
            arrival_stats: None,
        }).await.map_err(|_| CommunicatorError::Radio("Error sending message to channel".to_owned()))
    }

    async fn receive_downlink(
        &self,
        _timeout: Option<Duration>,
    ) -> Result<HashMap<SpreadingFactor, LoRaPacket>, CommunicatorError> {
        todo!()
    }
}