use std::net::SocketAddr;

use lorawan_device::communicator::{CommunicatorError, Position, ReceivedTransmission, Transmission};
use tokio::{net::UdpSocket, sync::{mpsc::{Receiver, Sender}, Mutex}};

use super::{node::NodeConfig, world::World};


pub struct NetworkControllerBridgeConfig {
    pub network_controller_address: SocketAddr,
    pub node_config: NodeConfig,
}


pub struct NetworkControllerBridge {
    network_controller_addr: SocketAddr,
    node_config: NodeConfig,
    sender: Sender<Transmission>,
    receiver: Mutex<Receiver<ReceivedTransmission>>,
    udp_socket: Option<UdpSocket>,
}

impl NetworkControllerBridge {
    pub fn new(sender: Sender<Transmission>, receiver: Receiver<ReceivedTransmission>, config: NetworkControllerBridgeConfig) -> Self {
        Self {
            network_controller_addr: config.network_controller_address,
            node_config: config.node_config,
            sender,
            udp_socket: None,
            receiver: Mutex::new(receiver),
        }
    }

    pub fn radio_sensitivity(&self) -> f32 {
        self.node_config.receiver_sensitivity
    }

    pub fn get_position(&self) -> Position {
        self.node_config.position
    }

    pub fn can_receive_transmission(&self, t: &ReceivedTransmission) -> bool {
        t.transmission.uplink &&                                                             //is downlink
        t.arrival_stats.rssi > self.node_config.receiver_sensitivity                         //signal strength is greater than receiver sensitivity
    }
}

impl NetworkControllerBridge {
    pub async fn wait_and_forward_uplink(&self) -> Result<(), CommunicatorError> {
        let mut receiver = self.receiver.lock().await;
        let received_transmission = receiver.recv().await.ok_or(CommunicatorError::Radio("Receiver channel closed unexpectedly".to_string()))?;
        let bytes = serde_json::to_vec(&received_transmission).unwrap();
        self.udp_socket.as_ref().unwrap().send(&bytes).await.map_err(|e| CommunicatorError::Radio(e.to_string()))?;
        Ok(())
    }
    
    pub async fn wait_and_forward_downlink(&self) -> Result<(), CommunicatorError> {
        let mut buffer = [0u8; 256];
        let size = self.udp_socket.as_ref().unwrap().recv(&mut buffer).await.unwrap();
        let transmission_bytes = &buffer[..size];
        let mut transmission = serde_json::from_slice::<Transmission>(transmission_bytes).map_err(|e| CommunicatorError::Radio(e.to_string()))?;

        transmission.start_position = self.node_config.position;
        transmission.start_time = World::now();
        transmission.starting_power = self.node_config.transmission_power_dbm;

        self.sender.send(transmission).await.map_err(|_| CommunicatorError::Radio("Error sending message to world".to_string()))
    }

    pub async fn start(&mut self) {
        let udp_socket = UdpSocket::bind("0.0.0.0:0").await.unwrap();
        udp_socket.connect(self.network_controller_addr).await.unwrap();
        self.udp_socket = Some(udp_socket);
    }
}