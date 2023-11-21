 use std::sync::Arc;
use vulkano::VulkanLibrary;
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::device::{Device, DeviceCreateInfo, QueueCreateInfo, QueueFlags};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator};
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage, BufferContents};

fn main() {
   
    let library = VulkanLibrary::new().expect("failed to load vulkan library");
    let instance = Instance::new(library, InstanceCreateInfo::default())
                            .expect("failed to create instance");


    let physical_device =  instance
                                .enumerate_physical_devices()
                                .expect("Could not enumerat physical devices")
                                .next()
                                .expect("No physical devices available");

    for family in physical_device.queue_family_properties() {
        println!("Found a queue family with {:?} queue(s)", family.queue_count);
    }

    let queue_family_index = physical_device
                                .queue_family_properties()
                                .iter()
                                .enumerate()
                                .position(|(_queue_family_index, queue_family_properties)| {
                                    queue_family_properties.queue_flags.contains(QueueFlags::GRAPHICS)
                                })
                                .expect("Could not find a graphical queue family.") as u32;

    let (device, mut queues) = Device::new(
        physical_device,
        DeviceCreateInfo {
            queue_create_infos: vec![QueueCreateInfo {
                queue_family_index, 
                ..Default::default()
            }],
            ..Default::default()
        },
    )
    .expect("failed to create device");
    
    let queue = queues.next().unwrap();
    let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));
    
    #[derive(BufferContents)]
    #[repr(C)]
    struct MyStruct {
        a: u32,
        b: u32,
    }

    let data = MyStruct { a: 5, b: 69 };
    let iter = (0..128).map(|_| 5u8);

    let buffer = Buffer::from_iter(
        memory_allocator.clone(),
        BufferCreateInfo {
            usage: BufferUsage::UNIFORM_BUFFER,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        },
        iter,
    )
    .unwrap();
    
}
