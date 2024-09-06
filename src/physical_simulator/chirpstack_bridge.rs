use std::collections::HashMap;

use lorawan::physical_parameters::{CodeRate, LoRaBandwidth, SpreadingFactor};
use lorawan_device::communicator::{Position, ReceivedTransmission, Transmission};
use paho_mqtt::AsyncClient;
use prost::Message;
use tokio::sync::mpsc::{Receiver, Sender};
use crate::compiled::gw::{modulation::Parameters, DownlinkFrame, LoraModulationInfo, Modulation, UplinkFrame, UplinkRxInfo, UplinkTxInfo};

use super::{node::NodeConfig, utils::get_sensitivity, world::World};


#[derive(Clone, Debug)]
pub struct ChirpstackBridgeConfig {
    pub gwid: &'static str,
    pub node_config: NodeConfig,
}

impl ChirpstackBridgeConfig {
    pub fn can_receive_transmission(&self, t: &ReceivedTransmission) -> bool {
        self.node_config.position != t.transmission.start_position &&
        t.transmission.uplink &&                                       //is uplink
        t.arrival_stats.rssi > get_sensitivity(&t.transmission)        //signal strength is greater than receiver sensitivity
    }
}

#[derive(Debug)]
pub struct ChirpstackBridge {
    id: u32,
    gwid: &'static str,
    node_config: NodeConfig,
    sender: Sender<Transmission>,
    receiver: Receiver<ReceivedTransmission>,
}

impl ChirpstackBridge {
    pub fn new(id: u32, sender: Sender<Transmission>, receiver: Receiver<ReceivedTransmission>, config: ChirpstackBridgeConfig) -> Self {
        Self {
            id,
            gwid: config.gwid,
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

    fn round_fq_u32(fq: f64) -> u32 {
        let f: [u32; 8] = [868_100_000, 868_300_000, 868_500_000, 867_100_000, 867_300_000, 867_500_000, 867_700_000, 867_900_000];
        for ff in f {
            if (fq - ff as f64).abs() < 1000.0 {
                return ff;
            }
        }
        f[0]
    }

    fn round_fq_f64(fq: u32) -> f64 {
        let f: [f64; 8] = [868_100_000.0, 868_300_000.0, 868_500_000.0, 867_100_000.0, 867_300_000.0, 867_500_000.0, 867_700_000.0, 867_900_000.0];
        for ff in f {
            if (fq as f64 - ff).abs() < 1000.0 {
                return ff;
            }
        }
        f[0]
    }

    fn create_uplink(gwid: &str, t: &ReceivedTransmission) -> UplinkFrame {
        UplinkFrame {
            phy_payload: t.transmission.payload.clone(),
            tx_info_legacy: None,
            rx_info_legacy: None,
            tx_info: Some(UplinkTxInfo {
                frequency: Self::round_fq_u32(t.transmission.frequency),
                modulation: Some(Modulation {
                    parameters: Some(Parameters::Lora(LoraModulationInfo {
                        bandwidth: t.transmission.bandwidth.hz() as u32,
                        spreading_factor: t.transmission.spreading_factor.value() as u32,
                        code_rate_legacy: String::new(),
                        code_rate: 1, //CR 4/5
                        polarization_inversion: false,
                    })),
                }),
            }),
            rx_info: Some(UplinkRxInfo {
                gateway_id: gwid.to_string(),
                uplink_id: rand::random(),
                time: None,
                time_since_gps_epoch: None,
                fine_time_since_gps_epoch: None,
                rssi: t.arrival_stats.rssi as i32,
                snr: t.arrival_stats.snr,
                channel: 1,
                rf_chain: 1,
                board: 1,
                antenna: 1,
                context: vec![1,2,3,4],
                metadata: HashMap::new(),
                crc_status: 0,
                location: None,
            }),
        }
    }

    pub async fn start(mut self) {
        let mut client = AsyncClient::new("tcp://169.254.189.196:1883").unwrap();
        let down_topic = format!("eu868/gateway/{}/command/down", self.gwid);
        let up_topic = format!("eu868/gateway/{}/event/up", self.gwid);

        let receiver = client.get_stream(1024);

        client.connect(None).await.unwrap();
        client.subscribe(&down_topic, paho_mqtt::QOS_2).await.unwrap();

        println!("ChirpstackBridge {} started", self.id);
        
        let t1 = tokio::spawn(async move {
            while let Some(received_transmission) = self.receiver.recv().await {
                //let received_transmission = self.receiver.recv().await.ok_or(CommunicatorError::Radio("Receiver channel closed unexpectedly".to_string())).unwrap();
                println!("[NC{}] Received uplink transmission with rssi {}", self.id, received_transmission.arrival_stats.rssi);
                let content = Self::create_uplink(self.gwid, &received_transmission);
                let v = content.encode_to_vec();
                
                client
                    .publish(paho_mqtt::Message::new(&up_topic, v, paho_mqtt::QOS_2));
            }
            panic!("ChirpstackBridge t1 {} stopped", self.id); 
        });
        

        let t2 = tokio::spawn(async move {
            //let mut buffer = [0u8; 1024];
            while let Ok(Some(msg)) = receiver.recv().await {
                let dwn = DownlinkFrame::decode(msg.payload()).unwrap();
                let t = &dwn.items[0];
                
                //let mut transmission = serde_json::from_slice::<Transmission>(transmission_bytes).map_err(|e| CommunicatorError::Radio(e.to_string())).unwrap();
                
                //println!("[NC{}] Received downlink transmission", self.id);

                let info = t.tx_info.as_ref().unwrap();
                let lora_modulation = match info.modulation.clone().unwrap().parameters.unwrap() {
                    Parameters::Lora(l) => l,
                    Parameters::Fsk(_) => todo!(),
                    Parameters::LrFhss(_) => todo!(),
                };

                let l = LoRaBandwidth::from(lora_modulation.bandwidth as f32);
                let transmission = Transmission {
                    start_position: self.node_config.position,
                    start_time: World::now(),
                    frequency: Self::round_fq_f64(info.frequency),
                    bandwidth: l,
                    spreading_factor: SpreadingFactor::new(lora_modulation.spreading_factor as u8),
                    code_rate: CodeRate::default(),
                    starting_power: self.node_config.transmission_power_dbm,
                    uplink: false,
                    payload: t.phy_payload.clone(),
                };

                //transmission.start_position = self.node_config.position;
                //transmission.start_time = World::now();
                //transmission.starting_power = self.node_config.transmission_power_dbm;
    
                if let Err(e) = self.sender.send(transmission).await {
                    eprintln!("Error sending message to world: {:?}", e);
                }

                println!("Sent downlink transmission to world");
            }       
            panic!("ChirpstackBridge t2 {} stopped", self.id); 
        });

        let (_r1,_r2) = tokio::join!(t1, t2);
        panic!("ChirpstackBridge {} stopped", self.id);
    }
}