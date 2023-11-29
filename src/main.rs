 use std::process::Command;
use std::sync::Arc;
use vulkano::{VulkanLibrary, descriptor_set};
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::device::{Device, DeviceCreateInfo, QueueCreateInfo, QueueFlags};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator};
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage, BufferContents};
use vulkano::command_buffer::allocator::{
    StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo,
};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, ClearColorImageInfo, CopyImageToBufferInfo};
use vulkano::pipeline::PipelineBindPoint;
use vulkano::sync::{self, GpuFuture};
use vulkano::pipeline::compute::ComputePipelineCreateInfo;
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::{ComputePipeline, PipelineLayout, PipelineShaderStageCreateInfo};
use vulkano::pipeline::Pipeline;
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;

use vulkano::image::{Image, ImageCreateInfo, ImageType, ImageUsage};
use vulkano::format::{Format, ClearColorValue};

use image::{ImageBuffer, Rgba};


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
    
    // Create allocators
    let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));
     let command_buffer_allocator = StandardCommandBufferAllocator::new(
        device.clone(),
        StandardCommandBufferAllocatorCreateInfo::default(),
    );
    let descriptor_set_allocator = 
                StandardDescriptorSetAllocator::new(device.clone(), Default::default());


    let data_iter = 0..65536u32;
    let data_buffer = Buffer::from_iter(
            memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::STORAGE_BUFFER,
                ..Default::default()
            },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
            },
        data_iter,
    )
    .expect("failed to create buffer");

    let image = Image::new(
        memory_allocator.clone(),
        ImageCreateInfo {
            image_type: ImageType::Dim2d,
            format: Format::R8G8B8A8_UNORM,
            extent: [1024, 1024, 1],
            usage: ImageUsage::TRANSFER_DST | ImageUsage::TRANSFER_SRC,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
            ..Default::default()
        },
    )
    .unwrap();

    let buf = Buffer::from_iter(
        memory_allocator.clone(),
        BufferCreateInfo {
            usage: BufferUsage::TRANSFER_DST,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE | MemoryTypeFilter::HOST_RANDOM_ACCESS,
            ..Default::default()
        },
        (0..1024 * 1024 * 4).map(|_| 0u8),
    )
    .expect("failed to create buffer");

    mod cs {
    vulkano_shaders::shader!{
        ty: "compute",
        src: r"
            #version 460

            layout(local_size_x = 64, local_size_y = 1, local_size_z = 1) in;

            layout(set = 0, binding = 0) buffer Data {
                uint data[];
            } buf;

            void main() {
                uint idx = gl_GlobalInvocationID.x;
                buf.data[idx] *= 12;
            }
         ",
        }
    }

    let shader = cs::load(device.clone()).expect("Failed to create shader module");

    let cs = shader.entry_point("main").unwrap();
    let stage = PipelineShaderStageCreateInfo::new(cs);
    
    let layout = PipelineLayout::new(
        device.clone(),
        PipelineDescriptorSetLayoutCreateInfo::from_stages([&stage])
        .into_pipeline_layout_create_info(device.clone())
        .unwrap(),
    )
    .unwrap();

    let compute_pipeline = ComputePipeline::new(
        device.clone(),
        None,
        ComputePipelineCreateInfo::stage_layout(stage, layout),
    )
    .expect("Failed to create compute pipeline");

   
    let pipeline_layout = compute_pipeline.layout();
    let descriptor_set_layouts = pipeline_layout.set_layouts();
    let descriptor_set_layout_index = 0;
    let descriptor_set_layout = descriptor_set_layouts
                                                                .get(descriptor_set_layout_index)
                                                                .unwrap();
    let descriptor_set = PersistentDescriptorSet::new(
        &descriptor_set_allocator,
        descriptor_set_layout.clone(),
        [WriteDescriptorSet::buffer(0, data_buffer.clone())],
        [],
    )
    .unwrap();

    // let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
    //     &command_buffer_allocator,
    //     queue_family_index,
    //     CommandBufferUsage::OneTimeSubmit,
    // )
    // .unwrap();

    // let work_group_counts = [1024, 1, 1];

    // command_buffer_builder
    //     .bind_pipeline_compute(compute_pipeline.clone())
    //     .unwrap()
    //     .bind_descriptor_sets(
    //         PipelineBindPoint::Compute,
    //         compute_pipeline.layout().clone(),
    //         descriptor_set_layout_index as u32,
    //         descriptor_set,
    //     )
    //     .unwrap()
    //     .dispatch(work_group_counts)
    //     .unwrap();

    // let command_buffer = command_buffer_builder.build().unwrap();

    let mut builder = AutoCommandBufferBuilder::primary(
        &command_buffer_allocator, 
        queue.queue_family_index(), 
    CommandBufferUsage::OneTimeSubmit,
    )
    .unwrap();

    builder
        .clear_color_image(ClearColorImageInfo {
            clear_value: ClearColorValue::Float([0.0, 0.0, 1.0, 1.0]),
            ..ClearColorImageInfo::image(image.clone())
        })
        .unwrap()
        .copy_image_to_buffer(CopyImageToBufferInfo::image_buffer(
            image.clone(),
            buf.clone(),
        ))
        .unwrap();
    
    let command_buffer = builder.build().unwrap();    
    let future = sync::now(device.clone())
            .then_execute(queue.clone(), command_buffer)
            .unwrap()
            .then_signal_fence_and_flush()
            .unwrap();

    future.wait(None).unwrap();

    let buffer_content = buf.read().unwrap();
    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(1024, 1024, &buffer_content[..]).unwrap();
    
    image.save("image.png").unwrap();
    // let content = data_buffer.read().unwrap();
    // for (n, val) in content.iter().enumerate() {
    //     assert_eq!(*val, n as u32 * 12);
    // }
    // println!("Success!")

}
