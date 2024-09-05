#![allow(non_snake_case)]
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct DeviceStatus {
    pub batteryLevel: u32,
    pub externalPowerSource: bool,
    pub margin: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeviceAns {
    pub createdAt: String,
    pub description: String,
    pub devEui: String,
    pub deviceProfileId: String,
    pub deviceProfileName: String,
    pub deviceStatus: Option<DeviceStatus>,
    pub lastSeenAt: Option<String>,
    pub name: String,
    pub updatedAt: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChirpstackListDeviceAns {
    pub totalCount: u32,
    pub result: Vec<DeviceAns>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChirpstackDeviceKeys {
    pub devEui: String,
    pub nwkKey: String,
    pub appKey: String, //always 0
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChirpstackDevice {
    pub deviceKeys: ChirpstackDeviceKeys,
    pub createdAt: String,
    pub updatedAt: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeviceActivation {
    pub devEui: String,
    pub devAddr: String,
    pub appSKey: String,
    pub nwkSEncKey: String,
    pub sNwkSIntKey: String,
    pub fNwkSIntKey: String,
    pub fCntUp: u32,
    pub nFCntDown: u32,
    pub aFCntDown: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChirpstackActivation {
    pub deviceActivation: DeviceActivation,
}