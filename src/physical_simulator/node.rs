
use std::{fs::OpenOptions, ops::{Deref, DerefMut}, sync::Arc, time::Duration};
use lorawan::utils::{eui::EUI64, PrettyHexSlice};
use lorawan_device::{communicator::{CommunicatorError, LoRaWANCommunicator, Position, ReceivedTransmission, Transmission}, configs::RadioDeviceConfig, devices::{debug_device::{DebugCommunicator, DebugDevice}, lorawan_device::LoRaWANDevice}};
use tokio::{sync::{mpsc::{Receiver, Sender}, Mutex, RwLock}, time::Instant};

use crate::physical_simulator::world::{FIXED_JOIN_DELAY, FIXED_PACKET_DELAY, LOGGER, NUM_PACKETS, RANDOM_JOIN_DELAY, RANDOM_PACKET_DELAY};

use super::{utils::get_sensitivity, world::World};
use std::io::Write;

#[derive(Clone, Debug)]
pub struct NodeConfig {
    pub position: Position,

    pub transmission_power_dbm: f32,  //14 dbm standard, and 27dbm is the maximum allowed
    pub receiver_sensitivity: f32,    

    pub tx_consumption: f32,
    pub rx_consumption: f32,
    pub idle_consumption: f32,
    pub sleep_consumption: f32,

    pub node_state: Arc<Mutex<NodeState>>,
    pub radio_config: RadioDeviceConfig,
}

impl PartialEq for NodeConfig {
    fn eq(&self, other: &Self) -> bool {
        self.position == other.position && self.radio_config == other.radio_config
    }
}

impl NodeConfig {
    pub async fn get_state(&self) -> NodeState {
        *self.node_state.lock().await
    }

    pub async fn can_receive_transmission(&self, t: &ReceivedTransmission) -> bool {
        self.position != t.transmission.start_position &&
        self.get_state().await == NodeState::Receiving &&
        !t.transmission.uplink &&                                                //is downlink
        t.transmission.frequency == self.radio_config.freq &&                    //same frequency
        t.transmission.bandwidth == self.radio_config.bandwidth &&               //same bandwidth
        t.transmission.spreading_factor == self.radio_config.spreading_factor && //same spreading factor
        t.arrival_stats.rssi > get_sensitivity(&t.transmission)                  //signal strength is greater than receiver sensitivity
    }
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
    pub node_id: u32,
    pub device: LoRaWANDevice<DebugCommunicator<NodeCommunicator>>,
}

impl Node {
    pub fn new(node_id: u32, device: LoRaWANDevice<NodeCommunicator>) -> Node {
        Node {
            node_id,
            device: DebugDevice::from(device),
        }
    }

    pub fn get_position(&self) -> Position {
        self.communicator().config.position
    }
    
    pub async fn get_state(&self) -> NodeState {
        self.communicator().config.get_state().await
    }
    
    //pub async fn can_receive_transmission(&self, t: &ReceivedTransmission) -> bool {
    //    self.get_position() != t.transmission.start_position &&
    //    self.get_state().await == NodeState::Receiving &&
    //    t.transmission.frequency == self.communicator().config.radio_config.freq &&                    //same frequency
    //    t.transmission.bandwidth == self.communicator().config.radio_config.bandwidth &&               //same bandwidth
    //    t.transmission.spreading_factor == self.communicator().config.radio_config.spreading_factor && //same spreading factor
    //    !t.transmission.uplink &&                                                                      //is downlink
    //    t.arrival_stats.rssi > get_sensitivity(&t.transmission)                                        //signal strength is greater than receiver sensitivity
    //}

    pub async fn run(&mut self) {
        let sleep_time = rand::random::<u64>() % RANDOM_JOIN_DELAY;
        if let Err(e) = self.join(Some(10), Some(Duration::from_secs(FIXED_JOIN_DELAY + sleep_time))).await {
            panic!("Error joining: {e:?}");
        };

        println!("Initialized: {}", PrettyHexSlice(&**self.dev_eui()));
        tokio::time::sleep(Duration::from_secs(FIXED_JOIN_DELAY + RANDOM_JOIN_DELAY - sleep_time)).await;            
        
        for i in 0..NUM_PACKETS {
            let sleep_time = rand::random::<u64>() % RANDOM_PACKET_DELAY;
            tokio::time::sleep(Duration::from_secs(FIXED_PACKET_DELAY + sleep_time)).await;
            
            let before = Instant::now();                
            match self.send_uplink(Some(format!("###  confirmed {i} message  ###").as_bytes()), true, Some(1), None).await {
                Ok(_) => {
                    println!("Device {} sent and received {i}-th message", PrettyHexSlice(&**self.dev_eui()));
                    let rtt = before.elapsed().as_millis();
                    if true {
                        LOGGER.write(&format!("{},{}", self.dev_eui(), rtt))
                    }
                },
                Err(e) => {
                    println!("Error sending confirmed message: {e:?}");
                },
            }

        }
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
    receiver: RwLock<Receiver<ReceivedTransmission>>,

    config: NodeConfig,
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
            receiver: RwLock::new(receiver),
            config,
            last_status_change: Instant::now(),
            tx_time: Duration::from_secs(0),
            rx_time: Duration::from_secs(0),
            idle_time: Duration::from_secs(0),
            sleep_time: Duration::from_secs(0),
        }
    }

    //pub fn calculate_energy_consumption(&self, duration: Duration) -> f32 {
    //    let seconds = duration.as_secs_f32();
    //    let energy_consumption = match self.state {
    //        NodeState::Idle => self.config.idle_consumption,
    //        NodeState::Sleep => self.config.sleep_consumption,
    //        NodeState::Transmitting => self.config.tx_consumption,
    //        NodeState::Receiving => self.config.rx_consumption,
    //    };
    //    energy_consumption * seconds
    //}

    //pub fn change_state(&mut self, new_state: NodeState) {
    //    let now = Instant::now();
    //    let duration = now.duration_since(self.last_status_change);
    //    match self.state {
    //        NodeState::Idle => self.idle_time += duration,
    //        NodeState::Sleep => self.sleep_time += duration,
    //        NodeState::Transmitting => self.tx_time += duration,
    //        NodeState::Receiving => self.rx_time += duration,
    //    }
    //    self.state = new_state;
    //    self.last_status_change = Instant::now();
    //}

    pub fn get_config(&self) -> &NodeConfig {
        &self.config
    }
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
        *self.config.node_state.lock().await = NodeState::Transmitting;        
        
        let ret = self.sender.send(t).await.map_err(|_| CommunicatorError::Radio("Error sending message to channel".to_owned()));
        tokio::time::sleep(Duration::from_millis(toa as u64)).await;
        
        *self.config.node_state.lock().await = NodeState::Idle;
        ret
    }

    async fn receive(
        &self,
        timeout: Option<Duration>,
    ) -> Result<Vec<ReceivedTransmission>, CommunicatorError> {
        
        *self.config.node_state.lock().await = NodeState::Receiving;
        
        let ret = if let Some(timeout) = timeout {
            tokio::time::timeout(timeout, self.receiver.write().await.recv()).await
        } else {
            Ok(self.receiver.write().await.recv().await)
        };
        
        *self.config.node_state.lock().await = NodeState::Idle;

        if let Ok(Some(v)) = ret {
            Ok(vec![v])
        } else {
            Err(CommunicatorError::Radio("Error receiving message from channel".to_owned()))
        }
    }
}