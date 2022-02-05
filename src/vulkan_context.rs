use std::sync::Arc;

use vulkano::device::physical::{PhysicalDevice, QueueFamily};
use vulkano::device::{Device, DeviceExtensions, Features, Queue};
use vulkano::instance::Instance;

pub struct VulkanContext<'a> {
    pub physical_device: PhysicalDevice<'a>,
    pub queue_family: QueueFamily<'a>,
    pub device: Arc<Device>,
    pub queues: Vec<Arc<Queue>>,
}

impl<'a> VulkanContext<'a> {
    pub fn new(instance: &'a Arc<Instance>) -> Self {
        let physical_device = {
            println!("Choose physical device:");
            for device in PhysicalDevice::enumerate(instance) {
                println!(
                    "[{}]: {} ({:?})",
                    device.index(),
                    device.properties().device_name,
                    device.properties().device_type
                );
            }

            'l: loop {
                let mut device_index = String::new();
                std::io::stdin()
                    .read_line(&mut device_index)
                    .expect("Failed to read user input.");
                match device_index.trim().parse::<usize>() {
                    Ok(index) => {
                        let devices = PhysicalDevice::enumerate(instance);
                        if index < devices.len() {
                            for device in devices {
                                if index == device.index() {
                                    break 'l device;
                                }
                            }
                        } else {
                            eprintln!("Index {index} out of bounds");
                        }
                    }
                    Err(e) => eprintln!("Error: {e} (s = '{device_index}')"),
                }
            }
        };

        let queue_family = physical_device
            .queue_families()
            .find(|qf| qf.supports_graphics())
            .expect("Cannot find a queue family that supports graphics.");
        let (device, queues) = Device::new(
            physical_device,
            &Features::none(),
            &DeviceExtensions::none(),
            [(queue_family, 0.5)].iter().cloned(),
        )
        .expect("Cannot create a device.");

        Self {
            physical_device,
            queue_family,
            device,
            queues: queues.collect(),
        }
    }
}
