use std::{collections::HashMap, time::{Duration, Instant}};

use lorawan::{physical_parameters::SpreadingFactor, utils::eui::EUI64};
use lorawan_device::{communicator::{CommunicatorError, LoRaPacket, LoRaWANCommunicator}, devices::lorawan_device::LoRaWANDevice};
use tokio::sync::oneshot;

use super::path_loss::Position;

pub struct NodeConfig {
    pub position: Position,

    pub transmission_power_dbm: f64,  //14 dbm for indoor devices and 27dbm for outdoor devices
    pub path_loss_exponent: f64,
    pub constant: f64,                

    pub tx_consumption: f64,
    pub rx_consumption: f64,
    pub idle_consumption: f64,
    pub sleep_consumption: f64,
}

pub enum NodeState {
    Idle,
    Sleep,
    Transmitting,
    Receiving,
}

pub struct Msg {
    bytes: Vec<u8>,
}

pub struct NodeCommunicator {
    sender: oneshot::Sender<Msg>,

    config: NodeConfig,
    state: NodeState,
    last_status_change: Instant,

    tx_time: Duration,
    rx_time: Duration,
    idle_time: Duration,
    sleep_time: Duration,
}

pub struct Node {
    pub device: LoRaWANDevice<NodeCommunicator>,
}

impl NodeCommunicator {
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
        todo!()
    }

    async fn receive_downlink(
        &self,
        _timeout: Option<Duration>,
    ) -> Result<HashMap<SpreadingFactor, LoRaPacket>, CommunicatorError> {
        todo!()
    }
}