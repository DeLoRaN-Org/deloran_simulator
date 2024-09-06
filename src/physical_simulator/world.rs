use std::{
    collections::HashSet,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use lazy_static::lazy_static;
use lorawan::{device::Device, physical_parameters::LoRaBandwidth};
use lorawan_device::{
    communicator::{ArrivalStats, Position, ReceivedTransmission, Transmission},
    devices::lorawan_device::LoRaWANDevice,
};
use rand::{prelude::Distribution, Rng, SeedableRng};
use tokio::sync::{
    mpsc::{self, Sender},
    Mutex, Notify,
};

use crate::{
    constants::{ACTIVE_LOGGER, LOGGER_PRINTLN, PRINT_LOG_PATH, RTT_LOG_PATH, STARTING_DEV_NONCE},
    logger::Logger, traffic_models::REGULAR_TRAFFIC_DISTRIBUTION,
};

use super::{
    chirpstack_bridge::{ChirpstackBridge, ChirpstackBridgeConfig},
    multi_node::MultiNode,
    network_controller_bridge::{NetworkControllerBridge, NetworkControllerBridgeConfig},
    node::{Node, NodeCommunicator, NodeConfig},
    path_loss::PathLossModel,
};

lazy_static! {
    pub static ref LOGGER: Logger = Logger::new(RTT_LOG_PATH, ACTIVE_LOGGER, LOGGER_PRINTLN);
    pub static ref PRINTER_LOGGER: Logger = Logger::new(PRINT_LOG_PATH, ACTIVE_LOGGER, LOGGER_PRINTLN);
    //pub static ref LOGGER_DEVICES: Logger = Logger::new("devices_complete.csv");
}

#[derive(Debug)]
pub enum EntityConfig {
    Node(NodeConfig),
    NetworkController(NetworkControllerBridgeConfig),
    ChipstackBridge(ChirpstackBridgeConfig),
}

#[derive(Debug)]
pub enum Entity {
    Node(Node),
    NetworkController(NetworkControllerBridge),
    ChipstackBridge(ChirpstackBridge),
}

impl EntityConfig {
    pub async fn get_position(&self) -> Position {
        match self {
            EntityConfig::Node(node) => node.position,
            EntityConfig::NetworkController(nc) => nc.node_config.position,
            EntityConfig::ChipstackBridge(c) => c.node_config.position,
        }
    }

    pub async fn can_receive_transmission(&self, t: &ReceivedTransmission) -> bool {
        match self {
            EntityConfig::Node(node) => node.can_receive_transmission(t).await,
            EntityConfig::NetworkController(nc) => nc.can_receive_transmission(t),
            EntityConfig::ChipstackBridge(c) => c.can_receive_transmission(t),
        }
    }
}

pub struct WorldConfig {
    pub path_loss_model: PathLossModel,
    //TODO: add more configuration options
}

#[derive(Debug)]
pub struct World {
    path_loss_model: PathLossModel,

    entity_configs: Vec<(EntityConfig, Sender<ReceivedTransmission>)>,
    entities: Vec<Entity>,
    //join_handlers: Vec<tokio::task::JoinHandle<()>>,

    transmissions_on_air: Arc<Mutex<Vec<Transmission>>>,

    sender: Sender<Transmission>,

    //incoming_message: Arc<Notify>,
    start_notifier: Arc<Notify>,

    nc_counter: u32,
    node_counter: u32,

    collision_counter: u32,
    successful_upload_counter: u32,
}

impl World {
    pub fn new(config: WorldConfig) -> World {
        let (sender, mut receiver) = mpsc::channel::<Transmission>(10000);
        let transmissions_on_air = Arc::new(Mutex::new(Vec::new()));
        let toac = transmissions_on_air.clone();
        let start_notifier = Arc::new(Notify::new());
        //let incoming_message = Arc::new(Notify::new());

        //let imc = incoming_message.clone();
        let snc = start_notifier.clone();

        tokio::spawn(async move {
            snc.notified().await;
            loop {
                //println!("Waiting for transmissionssssssssss...........");
                let t = receiver.recv().await.unwrap();
                //let toa = t.time_on_air();
                //println!("Transmission on air for {toa} ms");
                toac.lock().await.push(t);
                //println!("[World] added transmission to transmissions_on_air");
            }
        });

        World {
            entity_configs: Vec::new(),
            entities: Vec::new(),
            //join_handlers: Vec::new(),
            path_loss_model: config.path_loss_model,
            transmissions_on_air,
            sender,
            //incoming_message,
            start_notifier,
            nc_counter: 0,
            node_counter: 0,
            collision_counter: 0,
            successful_upload_counter: 0,
        }
    }

    pub fn node_counter(&self) -> u32 {
        self.node_counter
    }

    pub fn nc_counter(&self) -> u32 {
        self.nc_counter
    }

    pub fn now() -> u128 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
    }

    pub async fn multi_node_routine(mut multi_node: MultiNode) {
        //println!("BEGINNING MULTI NODE ROUTINE");
        multi_node.prepare().await;
        //println!("PREPARED MULTI NODE");
        multi_node.run().await;
    }

    pub async fn device_routine(mut node: Node) {
        node.run().await;
    }

    pub fn add_node(&mut self, device: Device, config: NodeConfig, regular_traffic_model: bool) {
        let (sender, receiver) = mpsc::channel(1000);

        let c2 = config.clone();

        let mut node = Node::new(
            self.node_counter,
            LoRaWANDevice::new(
                device,
                NodeCommunicator::new(self.sender.clone(), receiver, config),
            ),
            regular_traffic_model,
        );
        node.set_dev_nonce(STARTING_DEV_NONCE);

        self.entities.push(Entity::Node(node));
        self.node_counter += 1;

        let node_config = EntityConfig::Node(c2);
        self.entity_configs.push((node_config, sender));
    }

    pub async fn network_controller_routine(nc: NetworkControllerBridge) {
        nc.start().await;
    }

    pub fn add_network_controller(&mut self, nc_config: NetworkControllerBridgeConfig) {
        let (sender, receiver) = tokio::sync::mpsc::channel::<ReceivedTransmission>(10000);
        let nc = NetworkControllerBridge::new(
            self.nc_counter,
            self.sender.clone(),
            receiver,
            nc_config.clone(),
        );
        self.entities.push(Entity::NetworkController(nc));

        let nc = EntityConfig::NetworkController(nc_config);
        self.nc_counter += 1;
        self.entity_configs.push((nc, sender));
    }

    pub fn add_chirpstack_gw(&mut self, c_config: ChirpstackBridgeConfig) {
        let (sender, receiver) = tokio::sync::mpsc::channel::<ReceivedTransmission>(10000);
        let cb = ChirpstackBridge::new(
            self.nc_counter,
            self.sender.clone(),
            receiver,
            c_config.clone(),
        );

        self.entities.push(Entity::ChipstackBridge(cb));

        let nc = EntityConfig::ChipstackBridge(c_config);
        self.nc_counter += 1;
        self.entity_configs.push((nc, sender));
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
        t1.start_time > t2.start_time && t1.start_time < (t2.start_time + t2_toa)
            || t2.start_time > t1.start_time && t2.start_time < (t1.start_time + t1_toa)
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

    fn power_collision<'t>(
        t1: &'t Transmission,
        t2: &'t Transmission,
        device_position: Position,
        path_loss_model: &PathLossModel,
    ) -> Option<&'t Transmission> {
        let power_threshold = 6.0; //dB, it is hardcoded both in lorasim and flora
        let t1_rssi = t1.starting_power
            - path_loss_model
                .get_path_loss(device_position.distance(&t1.start_position).into(), t1.frequency);
        let t2_rssi = t2.starting_power
            - path_loss_model
                .get_path_loss(device_position.distance(&t2.start_position).into(), t2.frequency);
        if (t1_rssi - t2_rssi).abs() < power_threshold {
            None
        } else if t1_rssi - t2_rssi < power_threshold {
            Some(t2)
        } else {
            Some(t1)
        }
    }

    fn full_collision_check(t1: &Transmission, t2: &Transmission) -> bool {
        World::timing_collision(t1, t2)
            && World::direction_collision(t1, t2)
            && World::channel_collision(t1, t2)
            && World::sf_collision(t1, t2)
    }

    async fn create_received_transmission(
        &self,
        t: &Transmission,
        entity: &EntityConfig,
    ) -> Option<ReceivedTransmission> {
        let t_rssi = t.starting_power - self.path_loss_model.get_path_loss(entity.get_position().await.distance(&t.start_position).into(),t.frequency,);
        let t_rx: ReceivedTransmission = ReceivedTransmission {
            transmission: t.clone(),
            arrival_stats: ArrivalStats {
                time: World::now(),
                rssi: t_rssi,
                snr: 0.0,
            },
        };
        if entity.can_receive_transmission(&t_rx).await {
            Some(t_rx)
        } else {
            None
        }
    }

    async fn check_collisions_and_upload(&mut self) {
        let ended_transmissions = {
            let mut transmissions = self.transmissions_on_air.lock().await;
            let (ended_transmissions, not_endend_transmission) =
                transmissions.iter().cloned().partition(|t| t.ended());
            *transmissions = not_endend_transmission;
            ended_transmissions
        };

        let mut potentially_collided_transmissions = Vec::new();

        //println!(
        //    "Checking for collisions, number of ended transmissions: {}",
        //    ended_transmissions.len()
        //);

        for (i, t1) in ended_transmissions.iter().enumerate() {
            let mut collided = false;
            for t2 in ended_transmissions.iter().skip(i + 1) {
                if World::full_collision_check(t1, t2) {
                    potentially_collided_transmissions.push((t1.clone(), t2.clone()));
                    collided = true;
                }
            }

            if !collided {
                //println!("[World] Transmission not collided, can upload to entities");
                for (entity, sender) in self.entity_configs.iter() {
                    let device_position = entity.get_position().await;
                    if device_position == t1.start_position {
                        continue;
                    }
                    if let Some(t1_rx) = self.create_received_transmission(t1, entity).await {
                        sender.send(t1_rx).await.unwrap();
                    }
                }
            }
        }

        //println!(
        //    "Number of collisions: {}",
        //    potentially_collided_transmissions.len()
        //);
        self.collision_counter += potentially_collided_transmissions.len() as u32;

        for (entity, sender) in self.entity_configs.iter() {
            let mut survived_transmissions = HashSet::new();
            for (t1, t2) in potentially_collided_transmissions.iter() {
                let device_position = entity.get_position().await;
                let t = World::power_collision(t1, t2, device_position, &self.path_loss_model);
                if let Some(t) = t {
                    if let Some(t_rx) = self.create_received_transmission(t, entity).await {
                        survived_transmissions.insert(t_rx);
                    }
                }
            }

            for t in survived_transmissions.into_iter() {
                sender.send(t).await.unwrap();
            }
        }
    }

    pub async fn run(&mut self, duration: Option<Duration>) {
        self.start_notifier.notify_waiters();
        let mut multi_node = MultiNode::default();

        let entities = std::mem::take(&mut self.entities);

        for entity in entities {
            match entity {
                Entity::Node(node) => {
                    let mut rng = rand::rngs::StdRng::from_entropy();
                    
                    let node_delay = if rng.gen_range(0.0..1.0) < 0.86 {
                        let mut delay = 0.0;
                        while delay < 30.0 {
                            delay = REGULAR_TRAFFIC_DISTRIBUTION.sample(&mut rng)
                        }
                        Duration::from_secs_f64(delay)
                    } else {
                        Duration::ZERO
                    };
                    multi_node.add_node(node, node_delay);
                }
                Entity::NetworkController(nc) => {
                    tokio::spawn(World::network_controller_routine(nc));
                }
                Entity::ChipstackBridge(c) => {
                    tokio::spawn(c.start());
                }
            }
        }

        tokio::spawn(World::multi_node_routine(multi_node));

        tokio::spawn(async move {
            use tokio::runtime::Handle;
            let handle = Handle::current().metrics();

            loop {
                println!("Alive {} tasks", handle.num_alive_tasks());
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
            
        });

        let now = Instant::now();
        loop {
            //self.incoming_message.notified().await;
            //println!("[World] Checking for collisions");
            tokio::time::sleep(Duration::from_millis(19)).await;
            self.check_collisions_and_upload().await;
            //println!("[World] Checked for collisions");

            if let Some(duration) = duration {
                if now.elapsed() > duration {
                    break;
                }
            }
        }

        println!("END STATS: ");
        println!("Number of collisions: {}", self.collision_counter);
        println!(
            "Number of successful uploads: {}",
            self.successful_upload_counter
        );
        println!("Simulation ended");
    }
}

#[test]
fn simulated_transmission() {
    let t1 = Transmission {
        start_position: Position {
            x: 100000.0,
            y: 0.0,
            z: 0.0,
        },
        start_time: World::now(),
        frequency: 868_100_000.0,
        bandwidth: LoRaBandwidth::BW125,
        spreading_factor: lorawan::physical_parameters::SpreadingFactor::SF7,
        code_rate: lorawan::physical_parameters::CodeRate::CR4_5,
        starting_power: 14.0,
        uplink: true,
        payload: vec![0; 24],
    };

    println!("Time on air: {}", t1.time_on_air());

    let path_loss = PathLossModel::FreeSpace;

    let origin = Position {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    let rssi = t1.starting_power
        - path_loss.get_path_loss(t1.start_position.distance(&origin).into(), t1.frequency);
    println!("RSSI: {}", rssi);
}
