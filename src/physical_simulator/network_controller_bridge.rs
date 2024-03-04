use std::net::SocketAddr;

use lorawan_device::communicator::{CommunicatorError, ReceivedTransmission, Transmission};
use tokio::{net::UdpSocket, sync::broadcast::Sender};

use super::world::World;

pub struct NetworkControllerBridge {
    network_controller_address: SocketAddr,
    radio_sensitivity: f32,
    sender: Sender<Transmission>,
    udp_socket: UdpSocket,
}

impl NetworkControllerBridge {
    pub async fn new(network_controller_address: SocketAddr, sender: Sender<Transmission>) -> Self {
    let udp_socket = UdpSocket::bind("0.0.0.0:0").await.unwrap();
        udp_socket.connect(network_controller_address).await.unwrap();
        Self {
            network_controller_address,
            radio_sensitivity: -120.0,
            sender,
            udp_socket
        }
    }

    pub fn radio_sensitivity(&self) -> f32 {
        self.radio_sensitivity
    }
}

impl NetworkControllerBridge {
    pub async fn upload_transmission(&self, transmission: &ReceivedTransmission) -> Result<(), CommunicatorError> {
        self.udp_socket.send(&transmission.transmission.payload).await.unwrap();
        Ok(())
    }
    
    pub async fn wait_for_downlink(&self) -> Result<Vec<u8>, CommunicatorError> {
        let mut buffer = Vec::with_capacity(256);
        let size = self.udp_socket.recv_buf(&mut buffer).await.unwrap();
        Ok(buffer[..size].to_vec())
    }

    pub async fn send_downlink(&self, payload: Vec<u8>) -> Result<(), CommunicatorError> {
        self.sender.send(Transmission {
            payload,
            start_time: World::get_milliseconds_from_epoch(),
            start_position: todo!(),
            frequency: todo!(),
            bandwidth: todo!(),
            spreading_factor: todo!(),
            coding_rate: todo!(),
            starting_power: todo!(),
            uplink: todo!(),
        }).unwrap();
        Ok(())
    }
}