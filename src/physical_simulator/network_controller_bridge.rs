use std::{net::SocketAddr, sync::Arc};

use lorawan_device::communicator::{CommunicatorError, Position, ReceivedTransmission, Transmission};
use tokio::{net::UdpSocket, sync::mpsc::{Receiver, Sender}};

use super::{node::NodeConfig, utils::get_sensitivity, world::World};


#[derive(Clone)]
pub struct NetworkControllerBridgeConfig {
    pub network_controller_address: SocketAddr,
    pub node_config: NodeConfig,
}

impl NetworkControllerBridgeConfig {
    pub fn can_receive_transmission(&self, t: &ReceivedTransmission) -> bool {
        self.node_config.position != t.transmission.start_position &&
        t.transmission.uplink &&                                       //is uplink
        t.arrival_stats.rssi > get_sensitivity(&t.transmission)        //signal strength is greater than receiver sensitivity
    }
}


pub struct NetworkControllerBridge {
    id: u32,
    network_controller_addr: SocketAddr,
    node_config: NodeConfig,
    sender: Sender<Transmission>,
    receiver: Receiver<ReceivedTransmission>,
}

impl NetworkControllerBridge {
    pub fn new(id: u32, sender: Sender<Transmission>, receiver: Receiver<ReceivedTransmission>, config: NetworkControllerBridgeConfig) -> Self {
        Self {
            id,
            network_controller_addr: config.network_controller_address,
            node_config: config.node_config,
            sender,
            receiver,
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn radio_sensitivity(&self) -> f32 {
        self.node_config.receiver_sensitivity
    }

    pub fn get_position(&self) -> Position {
        self.node_config.position
    }

    pub fn can_receive_transmission(&self, t: &ReceivedTransmission) -> bool {
        self.node_config.position != t.transmission.start_position &&
        t.transmission.uplink &&                                       //is uplink
        t.arrival_stats.rssi > get_sensitivity(&t.transmission)        //signal strength is greater than receiver sensitivity
    }

    pub async fn start(mut self) {
        let udp_socket = UdpSocket::bind("0.0.0.0:0").await.unwrap();
        udp_socket.connect(self.network_controller_addr).await.unwrap();

        let udp_socket = Arc::new(udp_socket);
        let udp_socket_clone = udp_socket.clone();

        let t1 = tokio::spawn(async move {
            loop {
                let received_transmission = self.receiver.recv().await.ok_or(CommunicatorError::Radio("Receiver channel closed unexpectedly".to_string())).unwrap();

                //println!("[NC{}] Received uplink transmission with rssi {}", self.id, received_transmission.arrival_stats.rssi);

                let bytes = serde_json::to_string(&received_transmission).unwrap();
                //println!("{bytes} - {}", bytes.as_bytes().len());

                udp_socket.send(bytes.as_bytes()).await.map_err(|e| CommunicatorError::Radio(e.to_string())).unwrap();
            }
        });
        

        let t2 = tokio::spawn(async move {
            let mut buffer = [0u8; 1024];
            loop {
                let size = udp_socket_clone.recv(&mut buffer).await.unwrap();
    
                
                let transmission_bytes = &buffer[..size];
                let mut transmission = serde_json::from_slice::<Transmission>(transmission_bytes).map_err(|e| CommunicatorError::Radio(e.to_string())).unwrap();
                
                transmission.start_position = self.node_config.position;
                transmission.start_time = World::now();
                transmission.starting_power = self.node_config.transmission_power_dbm;
    
                //println!("[NC{}] Received downlink transmission", self.id);
                self.sender.send(transmission).await.map_err(|_| CommunicatorError::Radio("Error sending message to world".to_string())).unwrap();
            }
        });

        let (_r1,_r2) = tokio::join!(t1, t2);
    }
}