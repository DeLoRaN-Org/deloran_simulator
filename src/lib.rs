pub mod chirpstack;
pub mod compiled;
pub mod logger;
pub mod physical_simulator;
pub mod traffic_models;

pub mod constants {
    pub const NUM_DEVICES: usize = 25000;
    pub const DEVICES_TO_SKIP: usize = 0;
    pub const NUM_PACKETS: usize = 100;
    pub const FIXED_JOIN_DELAY: u64 = 60;
    pub const RANDOM_JOIN_DELAY: u64 = 180;
    pub const FIXED_PACKET_DELAY: u64 = 60;
    pub const RANDOM_PACKET_DELAY: u64 = 180;
    pub const _CONFIRMED_AVERAGE_SEND: u8 = 10;
    pub const STARTING_DEV_NONCE: u32 = 0;
    pub const STARTING_FCNT_UP: u32 = 730;

    pub const ACTIVE_LOGGER: bool = true;
    pub const LOGGER_PRINTLN: bool = true;

    pub const RTT_LOG_PATH: &str = "rtt_times.csv";
    pub const PRINT_LOG_PATH: &str = "log.txt";
}
