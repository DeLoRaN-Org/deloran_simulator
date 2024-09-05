use std::{
    cmp::Reverse, collections::{BinaryHeap, HashMap}, mem, ops::DerefMut, sync::Arc, time::{Duration, SystemTime, UNIX_EPOCH}
};

use lazy_static::lazy_static;
use lorawan::utils::eui::EUI64;
use lorawan_device::{
    communicator::Transmission,
    split_communicator::{LoRaReceiver, LoRaSender, SplitCommunicator},
};
use rand::{prelude::Distribution, Rng, SeedableRng};
use tokio::{
    sync::Mutex,
    time::Instant,
};

use crate::{
    logger::Logger, physical_simulator::world::World, traffic_models::UNREGULAR_TRAFFIC_DISTRIBUTION
};

use super::node::{Node, NodeReceiver, NodeSender};

lazy_static!(
    static ref ERROR_LOGGER: Logger = Logger::new("./Multinode_log.txt", true, true);
    static ref SESSIONS: Logger = Logger::new("./node_sessions.txt", true, false);
    static ref RESPONSE_TIMES: Logger = Logger::new("./response_times.csv", true, false);
);

#[derive(Debug)]
struct MultiNodeTransmission {
    dev_eui: EUI64,
    transmission: Transmission,
}

impl PartialEq for MultiNodeTransmission {
    fn eq(&self, other: &Self) -> bool {
        self.transmission == other.transmission
    }
}

impl Eq for MultiNodeTransmission {}

impl PartialOrd for MultiNodeTransmission {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MultiNodeTransmission {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.transmission.cmp(&other.transmission)
    }
}

#[derive(Debug, Default)]
pub struct MultiNode {
    nodes: Vec<(Node, Duration)>,
    senders_map: HashMap<EUI64, NodeSender>,
    receivers_map: HashMap<EUI64, Arc<NodeReceiver>>,
    transmissions: BinaryHeap<Reverse<MultiNodeTransmission>>,
}

impl MultiNode {
    pub fn add_node(&mut self, node: Node, duration: Duration) {
        self.nodes.push((node, duration));
    }

    pub async fn prepare(&mut self) {
        //self.join_devices().await;
        //println!("Joined all devices!!");
        self.prepare_transmissions().await;
        //println!("Prepared all the transmissions!!");
        let nodes = mem::take(&mut self.nodes);
        for (node, _) in nodes.into_iter() {
            let dev_eui = *node.dev_eui();
            let (sender, receiver) = node.into_device().into_communicator().split_communicator().await.unwrap();
            self.senders_map.insert(dev_eui, sender);
            self.receivers_map.insert(dev_eui, Arc::new(receiver));
        }
    }

    pub async fn join_devices(&mut self) {
        let mut nodes = mem::take(&mut self.nodes)
            .into_iter()
            .map(|v| Arc::new(Mutex::new(v)))
            .collect::<Vec<_>>();

        for (i, el) in nodes.iter_mut().enumerate() {
            let cloned = el.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(251 * i as u64)).await;
                let mut mutex_lock = cloned.lock().await;
                let (node, _duration) = mutex_lock.deref_mut();
                match  node.join(Some(10), Some(Duration::from_secs(1))).await {
                    Ok(_) => {
                        SESSIONS.write(&serde_json::to_string(&*node.device).unwrap());
                    },
                    Err(e) => ERROR_LOGGER.write(&format!("Device {i} failed to join: {e:?}")),
                }
            });

            // Limit to 100 concurrent tasks
            //if (i + 1) % 500 == 0 {
            //    tokio::time::sleep(Duration::from_secs(30)).await
            //}
        }
        //println!("BEFORE 30S SLEEP");
        // Wait for any remaining tasks
        tokio::time::sleep(Duration::from_secs(60)).await;

        //println!("AFTER 30S SLEEP");
        for node in nodes.into_iter() {
            while Arc::strong_count(&node) > 1 {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            self.nodes.push(Arc::into_inner(node).unwrap().into_inner())
        }
        //println!("END OF JOINING PROCESSES")
    }

    pub async fn prepare_transmissions(&mut self) {
        let mut rng = rand::rngs::StdRng::from_entropy();
        for (node, node_delay) in &mut self.nodes {
            let trans_power = node.communicator().get_config().transmission_power_dbm;
            let position = node.communicator().get_config().position;
            let radio_config = node.communicator().get_config().radio_config;

            //node.session_mut().expect("Session should be there thanks to node_sessions.txt").network_context_mut().update_f_cnt_up(STARTING_FCNT_UP);

            let mut start = SystemTime::now();
            //Sstart += Duration::from_secs_f64(rng.gen_range(0..600) as f64); // Random start time

            for i in 0..100 {
                let payload = node
                    .create_uplink(
                        Some(format!("###  confirmed {i} message  ###").into_bytes()).as_deref(),
                        true,
                        Some(1),
                        None,
                    )
                    .unwrap();

                //let delay = Duration::from_secs(rng.gen_range(FIXED_JOIN_DELAY..RANDOM_JOIN_DELAY));
                let complete_delay = if node_delay.is_zero() {
                    Duration::from_secs_f64(UNREGULAR_TRAFFIC_DISTRIBUTION.sample(&mut rng))
                } else {
                    *node_delay
                };
                let random_delay = Duration::from_secs(rng.gen_range(0..5));

                let transmission = Transmission {
                    start_position: position,
                    start_time: (start + complete_delay + random_delay)
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_millis(),
                    frequency: radio_config.freq,
                    bandwidth: radio_config.bandwidth,
                    spreading_factor: radio_config.spreading_factor,
                    code_rate: radio_config.code_rate,
                    starting_power: trans_power,
                    uplink: true,
                    payload,
                };

                start += complete_delay + random_delay;
                self.transmissions.push(Reverse(MultiNodeTransmission {
                    dev_eui: *node.dev_eui(),
                    transmission,
                }));
            }
        }
    }

    fn instant_from_u128(timestamp: u128) -> Instant {
        // Convert the u128 timestamp to a Duration since the Unix epoch
        let duration_since_epoch = Duration::from_millis(timestamp as u64);

        // Get the Unix epoch as an Instant
        let unix_epoch = Instant::now() - SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

        // Add the duration to the Unix epoch to get the target Instant
        unix_epoch + duration_since_epoch
    }

    pub async fn run(mut self) {
        // println!("MULTIDEVICE IS RUNNING!!");
        let (sender, mut receiver) = tokio::sync::mpsc::channel::<EUI64>(1000);
        let t_send = tokio::task::spawn(async move {
            loop {
                if let Some(transmission) = self.transmissions.pop() {
                    let transmission = transmission.0;
                    let lora_sender = self.senders_map.get_mut(&transmission.dev_eui).unwrap();
                    if World::now() < transmission.transmission.start_time {
                        tokio::time::sleep_until(Self::instant_from_u128(transmission.transmission.start_time,)).await;
                    }
                    lora_sender
                        .send(&transmission.transmission.payload, None)
                        .await
                        .unwrap();
                    sender.send(transmission.dev_eui).await.unwrap();
                }
            }
        });

        let t_recv = tokio::task::spawn(async move {
            while let Some(dev_eui) = receiver.recv().await {
                let lora_receiver = self.receivers_map.get_mut(&dev_eui).unwrap().clone();
                tokio::spawn(async move {
                    let now = Instant::now();
                    match lora_receiver.receive(Some(Duration::from_secs(2))).await {
                        Ok(_received) => {
                            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
                            RESPONSE_TIMES.write(&format!("{},{}", timestamp, now.elapsed().as_millis()));
                            //println!("Device {dev_eui} received {:?}", received);
                        }
                        Err(e) => {
                            ERROR_LOGGER.write(&format!("Device {dev_eui} didnt receive an answer: {:?} #########################",e));
                            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
                            RESPONSE_TIMES.write(&format!("{},{}", timestamp, now.elapsed().as_millis()));
                            //eprintln!("########################### Device {dev_eui} didnt receive an answer: {:?} #########################",e);
                        }
                    }
                });
            }
        });

        let (r1, r2) = tokio::join!(t_send, t_recv);
        r1.unwrap();
        r2.unwrap();
    }
}
