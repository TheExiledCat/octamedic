use cpal::{
    Device, DevicesError,
    traits::{DeviceTrait, HostTrait},
};

/// A sound device to play through or record from. `id` picks it and stays the same between runs,
/// `name` is for showing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
}

pub fn output_devices() -> Vec<AudioDevice> {
    return listed(cpal::default_host().output_devices());
}

pub fn input_devices() -> Vec<AudioDevice> {
    return listed(cpal::default_host().input_devices());
}

fn listed(devices: Result<impl Iterator<Item = Device>, DevicesError>) -> Vec<AudioDevice> {
    let Ok(devices) = devices else {
        return Vec::new();
    };
    return devices.filter_map(|device| described(&device)).collect();
}

fn described(device: &Device) -> Option<AudioDevice> {
    let id = device.id().ok()?.to_string();
    let name = device.description().ok()?.name().to_owned();
    return Some(AudioDevice { id, name });
}
