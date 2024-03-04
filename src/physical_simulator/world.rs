use std::{sync::Arc, time::{Duration, SystemTime, UNIX_EPOCH}};

use lorawan::{device::Device, physical_parameters::{CodeRate, DataRate, LoRaBandwidth, SpreadingFactor}, regional_parameters::region::Region, utils::PrettyHexSlice};
use lorawan_device::{communicator::{ArrivalStats, Position, ReceivedTransmission, Transmission}, configs::{RadioDeviceConfig, UDPNCConfig}, devices::lorawan_device::LoRaWANDevice};
use network_controller::network_controller::NetworkController;
use tokio::sync::{mpsc::Sender, Mutex, Notify, RwLock};

use super::{network_controller_bridge::NetworkControllerBridge, node::{Node, NodeCommunicator, NodeConfig, NodeState}, path_loss::PathLossModel};



const NUM_DEVICES: usize = 8000;
const NUM_PACKETS: usize = 100;
const RANDOM_JOIN_DELAY:   u64 = 18000;
const FIXED_JOIN_DELAY: u64 = 600;
const FIXED_PACKET_DELAY: u64 = 600;
const RANDOM_PACKET_DELAY: u64 = 17400;
const _CONFIRMED_AVERAGE_SEND: u8 = 10;
const DEVICES_TO_SKIP: usize = 0;
const JUST_CREATE_DEVICE: bool = false;
const STARTING_DEV_NONCE: u32 = 30;

const _NODE_CONFIG: NodeConfig = NodeConfig {
    position: Position {
        x: 0.0, 
        y: 0.0, 
        z: 0.0
    },
    transmission_power_dbm: 0.0,
    receiver_sensitivity: -120.0,
    tx_consumption: 0.0,
    rx_consumption: 0.0,
    idle_consumption: 0.0,
    sleep_consumption: 0.0,
    radio_config: RadioDeviceConfig {
        region: Region::EU863_870,
        spreading_factor: SpreadingFactor::SF7,
        data_rate: DataRate::DR5,
        bandwidth: LoRaBandwidth::BW125,
        sample_rate: 1_000_000.0,
        rx_freq: 990_000_000.0,
        tx_freq: 1_010_000_000.0,
        rx_chan_id: 0,
        tx_chan_id: 1,
        code_rate: CodeRate::CR4_5
    }
};

pub struct World {
    path_loss_model: PathLossModel,

    nodes: Vec<(Arc<RwLock<Node>>, Sender<ReceivedTransmission>)>,
    network_controllers: Vec<(Arc<RwLock<NetworkControllerBridge>>, Sender<ReceivedTransmission>)>,
    transmissions_on_air: Arc<Mutex<Vec<Transmission>>>,
 
    sender: Sender<Transmission>,
    incoming_message: Arc<Notify>,
}

impl World {
    pub fn new(path_loss_model: PathLossModel) -> World {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(100);
        let transmissions_on_air = Arc::new(Mutex::new(Vec::new()));
        let toac = transmissions_on_air.clone();
        let incoming_message = Arc::new(Notify::new());
        let imc = incoming_message.clone();

        tokio::spawn(async move {
            loop {
                let t = receiver.recv().await.unwrap();
                toac.lock().await.push(t);
                imc.notify_one();
            }
        });

        World {
            nodes: Vec::new(),
            network_controllers: Vec::new(),
            path_loss_model,
            transmissions_on_air,
            sender,
            incoming_message
        }
    }

    pub fn get_milliseconds_from_epoch() -> u128 {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()
    }

    pub async fn device_routine(node: Arc<RwLock<Node>>) {
        node.write().await.set_dev_nonce(STARTING_DEV_NONCE);
        let dev_eui = *node.read().await.dev_eui();
        let sleep_time = rand::random::<u64>() % RANDOM_JOIN_DELAY;
        tokio::time::sleep(Duration::from_secs(FIXED_JOIN_DELAY + sleep_time)).await;
        if let Err(e) = node.write().await.send_join_request().await {
            panic!("Error joining: {e:?}");
        };
        println!("Initialized: {}", PrettyHexSlice(&*dev_eui));
        tokio::time::sleep(Duration::from_secs(FIXED_JOIN_DELAY + RANDOM_JOIN_DELAY - sleep_time)).await;            
        for i in 0..NUM_PACKETS {
            let sleep_time = rand::random::<u64>() % RANDOM_PACKET_DELAY;
            tokio::time::sleep(Duration::from_secs(FIXED_PACKET_DELAY + sleep_time)).await;
            //let before = Instant::now();                
            
            let confirmed = true;
            node.write().await.send_uplink(Some(format!("###  {}confirmed {i} message  ###", if confirmed {"un"} else {""}).as_bytes()), confirmed, Some(1), None).await.unwrap();
            //let rtt = before.elapsed().as_millis();
            println!("Device {} sent and received {i}-th message", PrettyHexSlice(&*dev_eui));

            //if true {
            //    let mut file = OpenOptions::new()
            //    .append(true)
            //    .create(true)
            //    .open("/root/rtt_response_times.csv")
            //    .expect("Failed to open file");
            //    writeln!(file, "{},{}", World::get_milliseconds_from_epoch(), rtt).expect("Error while logging time to file");
            //}
        }
    }

    pub fn add_node(&mut self, device: Device, config: NodeConfig) {
        let (sender, receiver) = tokio::sync::mpsc::channel(10);
        let node = Node::new(LoRaWANDevice::new(device, NodeCommunicator::new(self.sender.clone(), receiver, config)));
        let node = Arc::new(RwLock::new(node));
        self.nodes.push((Arc::clone(&node), sender));
        tokio::spawn(async move {
            World::device_routine(node).await;
        });
    }

    pub fn add_network_controller(&mut self, nc: NetworkControllerBridge) { //TODO riguardare un po' sto codice
        let (sender, mut receiver) = tokio::sync::mpsc::channel(10);
        let nc_uploader = Arc::new(RwLock::new(nc));

        let nc_receiver = Arc::clone(&nc_uploader);
        self.network_controllers.push((Arc::clone(&nc_uploader), sender));

        tokio::spawn(async move {
            loop {
                let transmission = receiver.recv().await.unwrap();
                if transmission.arrival_stats.rssi > nc_uploader.read().await.radio_sensitivity() {
                    nc_uploader.read().await.upload_transmission(&transmission).await.unwrap();
                }
            }
        });
        
        tokio::spawn(async move {
            loop {
                let content = nc_receiver.read().await.wait_for_downlink().await.unwrap();
                nc_receiver.read().await.send_downlink(content).await.unwrap();
            }
        });
    }

    pub fn path_loss_model(&self) -> &PathLossModel {
        &self.path_loss_model
    }

    /*
    Collision checks taken from:
    repo: https://github.com/mcbor/lorasim
    repo: https://github/florasim/flora

    file: https://github.com/mcbor/lorasim/blob/main/loraDir.py
    file: https://github/florasim/flora/blob/main/src/LoRaPhy/LoRaReceiver.cc
    */

    fn timing_collision(t1: &Transmission, t2: &Transmission) -> bool {
        let t1_toa = t1.time_on_air();
        let t2_toa = t2.time_on_air();
        t1.start_time > t2.start_time && t1.start_time < (t2.start_time + t2_toa) || t2.start_time > t1.start_time && t2.start_time < (t1.start_time + t1_toa)
    }

    fn channel_collision(t1: &Transmission, t2: &Transmission) -> bool {
        if t1.frequency == 500_000.0 || t2.frequency == 500_000.0 {
            (t1.frequency - t2.frequency).abs() <= 120_000.0
        } else if t1.frequency == 250_000.0 || t2.frequency == 250_000.0 {
            (t1.frequency - t2.frequency).abs() <= 60_000.0
        } else {
            (t1.frequency - t2.frequency).abs() <= 30_000.0
        }
    }

    fn sf_collision(t1: &Transmission, t2: &Transmission) -> bool {
        t1.spreading_factor == t2.spreading_factor
    }
    
    fn direction_collision(t1: &Transmission, t2: &Transmission) -> bool {
        t1.uplink == t2.uplink
    }

    fn power_collision(t1_rssi: f32, t2_rssi: f32) -> bool {
        let power_threshold = 6.0;  //dB, it is hardcoded both in lorasim and flora
        (t1_rssi - t2_rssi).abs() < power_threshold || t1_rssi - t2_rssi < power_threshold
    }

    fn full_collision_check(t1: &Transmission, t2: &Transmission) -> bool {
        World::timing_collision(t1, t2) && World::direction_collision(t1, t2) && World::channel_collision(t1, t2) && World::sf_collision(t1, t2)
    }

    async fn check_collisions(&mut self) {
        let mut transmissions_on_air = self.transmissions_on_air.lock().await;
        for i in  0..transmissions_on_air.len() {
            let t1 = &transmissions_on_air[i];
            if t1.ended() { //if transmission ended
                for j in i + 1..transmissions_on_air.len() {
                    let t2 = &transmissions_on_air[j];
                    if World::full_collision_check(t1, t2) {
                        for (node, sender) in self.nodes.iter_mut() {
                            let device_position = node.read().await.get_position();
                            let t1_rssi = t1.starting_power - self.path_loss_model.get_path_loss(device_position.distance(&t1.start_position), t1.frequency);
                            let t2_rssi = t2.starting_power - self.path_loss_model.get_path_loss(device_position.distance(&t2.start_position), t2.frequency);
                            let t1_power_collision = World::power_collision(t1_rssi, t2_rssi);
                            if !t1_power_collision {
                                let t1_rx: ReceivedTransmission = ReceivedTransmission {
                                    transmission: t1.clone(),
                                    arrival_stats: ArrivalStats {
                                        time: World::get_milliseconds_from_epoch(),
                                        rssi: t1_rssi,
                                        snr: 0.0,
                                    }
                                };
                                let n = node.read().await;
                                if n.get_state() == NodeState::Receiving && n.can_receive_transmission(&t1_rx) {
                                    sender.send(t1_rx).await.unwrap();
                                }
                            }
                        }
                    }
                }
            }
        }
        transmissions_on_air.retain(|t| !t.ended());
    }


    pub async fn routine(&mut self) {
        loop {
            self.incoming_message.notified().await;
            self.check_collisions().await;        }
    }
}