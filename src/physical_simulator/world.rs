use std::{collections::HashSet, sync::Arc, time::{Duration, Instant, SystemTime, UNIX_EPOCH}};

use lorawan::{device::Device, physical_parameters::{CodeRate, DataRate, LoRaBandwidth, SpreadingFactor}, regional_parameters::region::Region, utils::PrettyHexSlice};
use lorawan_device::{communicator::{ArrivalStats, Position, ReceivedTransmission, Transmission}, configs::RadioDeviceConfig, devices::lorawan_device::LoRaWANDevice};
use tokio::sync::{mpsc::Sender, Mutex, Notify, RwLock};

use super::{network_controller_bridge::{NetworkControllerBridge, NetworkControllerBridgeConfig}, node::{Node, NodeCommunicator, NodeConfig}, path_loss::PathLossModel};

//const NUM_DEVICES: usize = 8000;
//const DEVICES_TO_SKIP: usize = 0;

const NUM_PACKETS: usize = 100;
const RANDOM_JOIN_DELAY:   u64 = 60;
const FIXED_JOIN_DELAY: u64 = 60;
const FIXED_PACKET_DELAY: u64 = 60;
const RANDOM_PACKET_DELAY: u64 = 60;
const _CONFIRMED_AVERAGE_SEND: u8 = 10;
const STARTING_DEV_NONCE: u32 = 0;

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
        freq: 868_000_000.0,
        rx_chan_id: 0,
        tx_chan_id: 1,
        code_rate: CodeRate::CR4_5
    }
};

pub enum Entity {
    Node(Arc<RwLock<Node>>),
    NetworkController(Arc<RwLock<NetworkControllerBridge>>),
}

impl Entity {
    pub async fn get_position(&self) -> Position {
        match self {
            Entity::Node(node) => node.read().await.get_position(),
            Entity::NetworkController(nc) => nc.read().await.get_position(),
        }
    }
    
    pub async fn can_receive_transmission(&self, t: &ReceivedTransmission) -> bool {
        match self {
            Entity::Node(node) => node.read().await.can_receive_transmission(t),
            Entity::NetworkController(nc) => nc.read().await.can_receive_transmission(t),
        }
    }
}

pub struct World {
    path_loss_model: PathLossModel,

    //nodes: Vec<(Arc<RwLock<Node>>, Sender<ReceivedTransmission>)>,
    //network_controllers: Vec<(Arc<RwLock<NetworkControllerBridge>>, Sender<ReceivedTransmission>)>,
    
    entities: Vec<(Entity, Sender<ReceivedTransmission>)>,
    
    transmissions_on_air: Arc<Mutex<Vec<Transmission>>>,
 
    sender: Sender<Transmission>,

    incoming_message: Arc<Notify>,
    start_notifier: Arc<Notify>,
}

impl World {
    pub fn new(path_loss_model: PathLossModel) -> World {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(100);
        let transmissions_on_air = Arc::new(Mutex::new(Vec::new()));
        let toac = transmissions_on_air.clone();
        let start_notifier = Arc::new(Notify::new());
        let incoming_message = Arc::new(Notify::new());
        
        
        let imc = incoming_message.clone();
        let snc = start_notifier.clone();

        tokio::spawn(async move {
            snc.notified().await;
            loop {
                let t = receiver.recv().await.unwrap();
                toac.lock().await.push(t);
                imc.notify_waiters();
            }
        });

        World {
            entities: Vec::new(),
            path_loss_model,
            transmissions_on_air,
            sender,
            incoming_message,
            start_notifier
        }
    }

    pub fn now() -> u128 {
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
            
            node.write().await.send_uplink(Some(format!("###  confirmed {i} message  ###").as_bytes()), true, Some(1), None).await.unwrap();
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
        let node = Entity::Node(Arc::new(RwLock::new(node)));
        self.entities.push((node, sender));
    }


    pub async fn networ_controller_routine(nc: Arc<RwLock<NetworkControllerBridge>>) {        
        let nc_receiver = Arc::clone(&nc);
        nc.write().await.start().await;
        
        tokio::spawn(async move {
            loop {
                nc_receiver.read().await.wait_and_forward_uplink().await.unwrap()
            }
        });
        
        loop {
            nc.read().await.wait_and_forward_downlink().await.unwrap()
        }
    }

    pub fn add_network_controller(&mut self, nc_config: NetworkControllerBridgeConfig) {
        let (sender, receiver) = tokio::sync::mpsc::channel::<ReceivedTransmission>(10);
        let nc = Entity::NetworkController(Arc::new(RwLock::new(NetworkControllerBridge::new(self.sender.clone(), receiver, nc_config))));
        self.entities.push((nc, sender));
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
        if t1.bandwidth == LoRaBandwidth::BW500 || t2.bandwidth == LoRaBandwidth::BW500 {
            (t1.frequency - t2.frequency).abs() <= 120_000.0
        } else if t1.bandwidth == LoRaBandwidth::BW250 || t2.bandwidth == LoRaBandwidth::BW250 {
            (t1.frequency - t2.frequency).abs() <= 60_000.0
        } else {
            (t1.frequency - t2.frequency).abs() <= 30_000.0
        }
    }

    fn sf_collision(t1: &Transmission, t2: &Transmission) -> bool {
        t1.spreading_factor == t2.spreading_factor
    }
    
    fn direction_collision(t1: &Transmission, t2: &Transmission) -> bool {
        t1.uplink == t2.uplink //it should be iq check (uplink and downlink have inverted iq so they dont collide and gateways dont receive each other)
    }

    fn power_collision(t1: &Transmission, t2: &Transmission, device_position: Position, path_loss_model: &PathLossModel) -> bool {
        let power_threshold = 6.0;  //dB, it is hardcoded both in lorasim and flora
        let t1_rssi = t1.starting_power - path_loss_model.get_path_loss(device_position.distance(&t1.start_position), t1.frequency);
        let t2_rssi = t2.starting_power - path_loss_model.get_path_loss(device_position.distance(&t2.start_position), t2.frequency);
        (t1_rssi - t2_rssi).abs() < power_threshold || t1_rssi - t2_rssi < power_threshold
    }

    fn full_collision_check(t1: &Transmission, t2: &Transmission) -> bool {
        World::timing_collision(t1, t2) && World::direction_collision(t1, t2) && World::channel_collision(t1, t2) && World::sf_collision(t1, t2)
    }

    async fn check_collisions_and_upload(&mut self) {
        let ended_transmissions = self.transmissions_on_air.lock().await.iter().filter(|t| t.ended()).cloned().collect::<Vec<Transmission>>();  
        let mut potentially_collided_transmissions = Vec::new();

        for (i, t1) in ended_transmissions.iter().enumerate() {
            let mut collided = false;
            for t2 in ended_transmissions.iter().skip(i + 1) {
                if World::full_collision_check(t1, t2) {
                    potentially_collided_transmissions.push((t1.clone(), t2.clone()));
                    collided = true;
                }
            }

            if !collided {
                for (entity, sender) in self.entities.iter() {
                    let device_position = entity.get_position().await;
                    let t1_rssi = t1.starting_power - self.path_loss_model.get_path_loss(device_position.distance(&t1.start_position), t1.frequency);
                    let t1_rx: ReceivedTransmission = ReceivedTransmission {
                        transmission: t1.clone(),
                        arrival_stats: ArrivalStats {
                            time: World::now(),
                            rssi: t1_rssi,
                            snr: 0.0,
                        }
                    };
                    if entity.can_receive_transmission(&t1_rx).await {
                        sender.send(t1_rx).await.unwrap();
                    }
                }
            }
        }

        for (entity, sender) in self.entities.iter() {
            let mut received = HashSet::new();
            for (t1, t2) in potentially_collided_transmissions.iter() {
                let device_position = entity.get_position().await;
                if !World::power_collision(t1, t2, device_position, &self.path_loss_model) {
                    let t1_rssi = t1.starting_power - self.path_loss_model.get_path_loss(device_position.distance(&t1.start_position), t1.frequency);
                    let t1_rx: ReceivedTransmission = ReceivedTransmission {
                        transmission: t1.clone(),
                        arrival_stats: ArrivalStats {
                            time: World::now(),
                            rssi: t1_rssi,
                            snr: 0.0,
                        }
                    };
                    if entity.can_receive_transmission(&t1_rx).await {
                        received.insert(t1_rx);
                    }
                }
            }

            for t in received.into_iter() {
                sender.send(t).await.unwrap();
            }
        }
        self.transmissions_on_air.lock().await.retain(|t| !t.ended());
    }


    pub async fn run(&mut self, duration: Option<Duration>) {
        self.start_notifier.notify_waiters();

        for (entity, _) in self.entities.iter() {
            match entity {
                Entity::Node(node) => {
                    let node_clone = Arc::clone(node);
                    tokio::spawn(World::device_routine(node_clone));
                },
                Entity::NetworkController(nc) => {
                    let nc_clone = Arc::clone(nc);
                    tokio::spawn(World::networ_controller_routine(nc_clone));
                },
            }
        }
        //Entity::NetworkController(nc) => tokio::spawn(World::networ_controller_routine(Arc::clone(nc))),
        //Entity::Node(node) => tokio::spawn(World::device_routine(Arc::clone(node))),

        let now = Instant::now();

        loop {
            self.incoming_message.notified().await;
            self.check_collisions_and_upload().await;

            if let Some(duration) = duration {
                if now.elapsed() > duration {
                    break;
                }
            }
        }

        println!("Simulation ended");
    }
}