
use std::{ops::{Deref, DerefMut}, time::Duration};
use lorawan::utils::eui::EUI64;
use lorawan_device::{communicator::{CommunicatorError, LoRaWANCommunicator, Position, ReceivedTransmission, Transmission}, configs::RadioDeviceConfig, devices::{debug_device::{DebugCommunicator, DebugDevice}, lorawan_device::LoRaWANDevice}};
use tokio::{sync::{mpsc::{Receiver, Sender}, Mutex}, time::Instant};

use super::world::World;

#[derive(Clone, Debug, Copy, PartialEq)]
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

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum NodeState {
    Idle,
    Sleep,
    Transmitting,
    Receiving,
}

#[derive(Debug)]
pub struct Node {
    pub device: LoRaWANDevice<DebugCommunicator<NodeCommunicator>>,
}

impl Node {
    pub fn new(device: LoRaWANDevice<NodeCommunicator>) -> Node {
        Node {
            device: DebugDevice::from(device),
        }
    }

    pub fn get_position(&self) -> Position {
        self.communicator().config.position
    }
    
    pub fn get_state(&self) -> NodeState {
        self.communicator().state
    }
    
    pub fn can_receive_transmission(&self, t: &ReceivedTransmission) -> bool {
        self.get_state() == NodeState::Receiving &&
        t.transmission.frequency == self.communicator().config.radio_config.freq &&                    //same frequency
        t.transmission.bandwidth == self.communicator().config.radio_config.bandwidth &&               //same bandwidth
        t.transmission.spreading_factor == self.communicator().config.radio_config.spreading_factor && //same spreading factor
        !t.transmission.uplink &&                                                                      //is downlink
        t.arrival_stats.rssi > self.communicator().config.receiver_sensitivity                         //signal strength is greater than receiver sensitivity
    }
}

 
impl Deref for Node {
    type Target = LoRaWANDevice<DebugCommunicator<NodeCommunicator>>;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

impl DerefMut for Node {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.device
    }
}


#[derive(Debug)]
pub struct NodeCommunicator {
    sender: Sender<Transmission>,
    receiver: Mutex<Receiver<ReceivedTransmission>>,

    config: NodeConfig,
    state: NodeState,
    last_status_change: Instant,

    tx_time: Duration,
    rx_time: Duration,
    idle_time: Duration,
    sleep_time: Duration,
}

impl NodeCommunicator {
    pub fn new(sender: Sender<Transmission>, receiver: Receiver<ReceivedTransmission>,  config: NodeConfig) -> NodeCommunicator {
    NodeCommunicator {
            sender,
            receiver: Mutex::new(receiver),
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

    pub fn get_config(&self) -> &NodeConfig {
        &self.config
    }

    //pub async fn send_uplink(&mut self, bytes: &[u8]) -> Result<(), CommunicatorError> {
    //    self.change_state(NodeState::Transmitting);
    //    let ret = <Self as LoRaWANCommunicator>::send(self, bytes, None, None).await;
    //    self.change_state(NodeState::Idle);
    //    ret
    //}
    //
    //pub async fn receive_downlink(&mut self, timeout: Option<Duration>) -> Result<ReceivedTransmission, CommunicatorError> {
    //    self.change_state(NodeState::Receiving);
    //    let ret = <Self as LoRaWANCommunicator>::receive(self, timeout).await?;
    //    self.change_state(NodeState::Idle);
    //    ret.into_iter().next().ok_or(CommunicatorError::Radio("No downlink received".to_owned()))
    //}
}


#[async_trait::async_trait]
impl LoRaWANCommunicator for NodeCommunicator {
    type Config=NodeConfig;

    async fn from_config(_config: &Self::Config) -> Result<Self, CommunicatorError> {
        todo!()
    }
    
    async fn send(
        &self,
        bytes: &[u8],
        _src: Option<EUI64>,
        _dest: Option<EUI64>,
    ) -> Result<(), CommunicatorError> {
        let t = Transmission {
            start_position: self.config.position,
            start_time: World::now(),
            frequency: self.config.radio_config.freq,
            bandwidth: self.config.radio_config.bandwidth,
            spreading_factor: self.config.radio_config.spreading_factor,
            code_rate: self.config.radio_config.code_rate,
            starting_power: self.config.transmission_power_dbm,
            uplink: true,
            payload: bytes.to_vec(),
        };

        let toa = t.time_on_air();
        let ret = self.sender.send(t).await.map_err(|_| CommunicatorError::Radio("Error sending message to channel".to_owned()));
        tokio::time::sleep(Duration::from_millis(toa as u64)).await;
        ret
    }

    async fn receive(
        &self,
        timeout: Option<Duration>,
    ) -> Result<Vec<ReceivedTransmission>, CommunicatorError> {
        let mut receiver = self.receiver.lock().await;
        let ret = if let Some(timeout) = timeout {
            tokio::time::timeout(timeout, receiver.recv()).await
        } else {
            Ok(receiver.recv().await)
        };
        
        if let Ok(Some(v)) = ret {
            Ok(vec![v])
        } else {
            Err(CommunicatorError::Radio("Error receiving message from channel".to_owned()))
        }
    }
}