use std::{collections::HashMap, net::SocketAddr};

use lorawan::{lorawan_packet::{payload::Payload, LoRaWANPacket}, utils::PrettyHexSlice};
use lorawan_device::communicator::{CommunicatorError, LoRaPacket, Position, ReceivedTransmission, Transmission};
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
    last_uplink_received: HashMap<String, ReceivedTransmission>
}

impl NetworkControllerBridge {
    pub fn new(sender: Sender<Transmission>, receiver: Receiver<ReceivedTransmission>, config: NetworkControllerBridgeConfig) -> Self {
        Self {
            network_controller_addr: config.network_controller_address,
            node_config: config.node_config,
            sender,
            udp_socket: None,
            receiver: Mutex::new(receiver),
            last_uplink_received: HashMap::new()
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
    pub async fn wait_for_uplink(&self) -> Result<ReceivedTransmission, CommunicatorError> {
        let mut receiver = self.receiver.lock().await;
        receiver.recv().await.ok_or(CommunicatorError::Radio("Receiver channel closed unexpectedly".to_string()))
    }

    pub async fn upload_transmission(&mut self, transmission: &ReceivedTransmission) -> Result<(), CommunicatorError> {
        self.udp_socket.as_ref().unwrap().send(&transmission.transmission.payload).await.unwrap();
        let packet = LoRaWANPacket::from_bytes(&transmission.transmission.payload, None, true).unwrap(); //cannot fail without context
        let key = match packet.payload() {
            Payload::JoinRequest(join) => join.dev_eui().to_string(),
            Payload::MACPayload(mc) => PrettyHexSlice(&mc.fhdr().dev_addr()).to_string(),
            _ => unreachable!(),
        };
        self.last_uplink_received.insert(key, transmission.clone());
        Ok(())
    }
    
    pub async fn wait_for_downlink(&self) -> Result<Vec<u8>, CommunicatorError> {
        let mut buffer = [0u8; 256];
        let size = self.udp_socket.as_ref().unwrap().recv(&mut buffer).await.unwrap();
        Ok(buffer[..size].to_vec())
    }

    pub async fn send_downlink(&self, payload: Vec<u8>) -> Result<(), CommunicatorError> {
        self.sender.send(Transmission {
            payload,
            start_time: World::get_milliseconds_from_epoch(),
            start_position: self.node_config.position,
            frequency: self.node_config.radio_config.freq,
            bandwidth: self.node_config.radio_config.bandwidth,
            spreading_factor: self.node_config.radio_config.spreading_factor,
            code_rate: self.node_config.radio_config.code_rate,
            starting_power: self.node_config.transmission_power_dbm,
            uplink: false,
        }).await.unwrap();
        Ok(())
    }

    pub async fn start(&mut self) {
        let udp_socket = UdpSocket::bind("0.0.0.0:0").await.unwrap();
        udp_socket.connect(self.network_controller_addr).await.unwrap();
        self.udp_socket = Some(udp_socket);
    }
}