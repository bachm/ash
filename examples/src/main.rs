#![feature(conservative_impl_trait)]
#![allow(dead_code)]
extern crate owning_ref;
extern crate ash;
extern crate vk_loader2 as vk;
extern crate glfw;

use glfw::*;
use ash::instance::{Entry, Instance};
use std::ptr;
use std::ffi::{CStr, CString};
use std::mem;
use std::path::Path;
use std::fs::File;
use std::io::Read;
use std::os::raw::c_void;
macro_rules! printlndb{
    ($arg: tt) => {
        println!("{:?}", $arg);
    }
}
fn handle_window_event(window: &mut glfw::Window, event: glfw::WindowEvent) {
    match event {
        glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => window.set_should_close(true),
        _ => {}
    }
}
pub fn find_memorytype_index(memory_req: &vk::MemoryRequirements,
                             memory_prop: &vk::PhysicalDeviceMemoryProperties,
                             flags: vk::MemoryPropertyFlags)
                             -> Option<u32> {
    let mut memory_type_bits = memory_req.memory_type_bits;
    for (index, ref memory_type) in memory_prop.memory_types.iter().enumerate() {
        if (memory_type.property_flags & flags) == flags {
            return Some(index as u32);
        }
        memory_type_bits = memory_type_bits >> 1;
    }
    None
}
#[derive(Clone, Debug, Copy)]
struct Vertex {
    x: f32,
    y: f32,
    z: f32,
    w: f32,
}

fn main() {
    let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();

    let window_width = 1920;
    let window_height = 1080;

    let (mut window, events) = glfw.create_window(window_width,
                       window_height,
                       "Hello this is window",
                       glfw::WindowMode::Windowed)
        .expect("Failed to create GLFW window.");

    window.set_key_polling(true);
    window.make_current();
    let entry = Entry::load_vulkan().unwrap();
    let instance_ext_props = entry.enumerate_instance_extension_properties().unwrap();
    let app_name = CString::new("TEST").unwrap();
    let raw_name = app_name.as_ptr();

    let layer_names = [CString::new("VK_LAYER_LUNARG_standard_validation").unwrap()];
    let layers_names_raw: Vec<*const i8> = layer_names.iter()
        .map(|raw_name| raw_name.as_ptr())
        .collect();
    let extension_names = [CString::new("VK_KHR_surface").unwrap(),
                           CString::new("VK_KHR_xlib_surface").unwrap()];
    let extension_names_raw: Vec<*const i8> = extension_names.iter()
        .map(|raw_name| raw_name.as_ptr())
        .collect();
    let appinfo = vk::ApplicationInfo {
        p_application_name: raw_name,
        s_type: vk::StructureType::ApplicationInfo,
        p_next: ptr::null(),
        application_version: 0,
        p_engine_name: raw_name,
        engine_version: 0,
        api_version: 0,
    };
    let create_info = vk::InstanceCreateInfo {
        s_type: vk::StructureType::InstanceCreateInfo,
        p_application_info: &appinfo,
        p_next: ptr::null(),
        pp_enabled_layer_names: layers_names_raw.as_ptr(),
        enabled_layer_count: layers_names_raw.len() as u32,
        pp_enabled_extension_names: extension_names_raw.as_ptr(),
        enabled_extension_count: extension_names_raw.len() as u32,
        flags: 0,
    };
    let instance = entry.create_instance(create_info).expect("Instance creation error");
    let x11_display = window.glfw.get_x11_display();
    let x11_window = window.get_x11_window();
    let x11_create_info = vk::XlibSurfaceCreateInfoKHR {
        s_type: vk::StructureType::XlibSurfaceCreateInfoKhr,
        p_next: ptr::null(),
        flags: 0,
        window: x11_window as vk::Window,
        dpy: x11_display as *mut vk::Display,
    };
    let surface = instance.create_xlib_surface_khr(x11_create_info).unwrap();
    let pdevices = instance.enumerate_physical_devices().expect("Physical device error");
    let (pdevice, queue_family_index) = pdevices.iter()
        .map(|pdevice| {
            instance.get_physical_device_queue_family_properties(*pdevice)
                .iter()
                .enumerate()
                .filter_map(|(index, ref info)| {
                    let supports_graphic_and_surface =
                        info.queue_flags.subset(vk::QUEUE_GRAPHICS_BIT) &&
                        instance.get_physical_device_surface_support_khr(*pdevice,
                                                                         index as u32,
                                                                         surface);
                    match supports_graphic_and_surface {
                        true => Some((*pdevice, index)),
                        _ => None,
                    }
                })
                .nth(0)
        })
        .filter_map(|v| v)
        .nth(0)
        .expect("Couldn't find suitable device.");
    let queue_family_index = queue_family_index as u32;
    let device_extension_names = [CString::new("VK_KHR_swapchain").unwrap()];
    let device_extension_names_raw: Vec<*const i8> = device_extension_names.iter()
        .map(|raw_name| raw_name.as_ptr())
        .collect();
    let features = vk::PhysicalDeviceFeatures { shader_clip_distance: 1, ..Default::default() };
    let priorities = [1.0];
    let queue_info = vk::DeviceQueueCreateInfo {
        s_type: vk::StructureType::DeviceQueueCreateInfo,
        p_next: ptr::null(),
        flags: 0,
        queue_family_index: queue_family_index as u32,
        p_queue_priorities: priorities.as_ptr(),
        queue_count: priorities.len() as u32,
    };
    let device_create_info = vk::DeviceCreateInfo {
        s_type: vk::StructureType::DeviceCreateInfo,
        p_next: ptr::null(),
        flags: 0,
        queue_create_info_count: 1,
        p_queue_create_infos: &queue_info,
        enabled_layer_count: 0,
        pp_enabled_layer_names: ptr::null(),
        enabled_extension_count: device_extension_names_raw.len() as u32,
        pp_enabled_extension_names: device_extension_names_raw.as_ptr(),
        p_enabled_features: &features,
    };
    let device: ash::instance::Device = instance.create_device(pdevice, device_create_info)
        .unwrap();
    let present_queue = device.get_device_queue(queue_family_index as u32, 0);

    let surface_formats = instance.get_physical_device_surface_formats_khr(pdevice, surface)
        .unwrap();
    let surface_format = surface_formats.iter()
        .map(|sfmt| {
            match sfmt.format {
                vk::Format::Undefined => {
                    vk::SurfaceFormatKHR {
                        format: vk::Format::B8g8r8Unorm,
                        color_space: sfmt.color_space,
                    }
                }
                _ => sfmt.clone(),
            }
        })
        .nth(0)
        .expect("Unable to find suitable surface format.");
    let surface_capabilities =
        instance.get_physical_device_surface_capabilities_khr(pdevice, surface).unwrap();
    let desired_image_count = 2;
    assert!(surface_capabilities.min_image_count <= desired_image_count &&
            surface_capabilities.max_image_count >= desired_image_count,
            "Image count err");
    let surface_resoultion = match surface_capabilities.current_extent.width {
        std::u32::MAX => {
            vk::Extent2D {
                width: window_width,
                height: window_height,
            }
        }
        _ => surface_capabilities.current_extent,
    };

    let pre_transform = if surface_capabilities.supported_transforms
        .subset(vk::SURFACE_TRANSFORM_IDENTITY_BIT_KHR) {
        vk::SURFACE_TRANSFORM_IDENTITY_BIT_KHR
    } else {
        surface_capabilities.current_transform
    };
    let present_modes = instance.get_physical_device_surface_present_modes_khr(pdevice, surface)
        .unwrap();
    let present_mode = present_modes.iter()
        .cloned()
        .find(|&mode| mode == vk::PresentModeKHR::Mailbox)
        .unwrap_or(vk::PresentModeKHR::Fifo);
    let swapchain_create_info = vk::SwapchainCreateInfoKHR {
        s_type: vk::StructureType::SwapchainCreateInfoKhr,
        p_next: ptr::null(),
        flags: 0,
        surface: surface,
        min_image_count: desired_image_count,
        image_color_space: surface_format.color_space,
        image_format: surface_format.format,
        image_extent: surface_resoultion.clone(),
        image_usage: vk::IMAGE_USAGE_COLOR_ATTACHMENT_BIT,
        image_sharing_mode: vk::SharingMode::Exclusive,
        pre_transform: pre_transform,
        composite_alpha: vk::COMPOSITE_ALPHA_OPAQUE_BIT_KHR,
        present_mode: present_mode,
        clipped: 1,
        old_swapchain: vk::SwapchainKHR::null(),
        // FIX ME: What is this?
        image_array_layers: 1,
        p_queue_family_indices: ptr::null(),
        queue_family_index_count: 0,
    };
    let swapchain = device.create_swapchain_khr(swapchain_create_info).unwrap();
    let pool_create_info = vk::CommandPoolCreateInfo {
        s_type: vk::StructureType::CommandPoolCreateInfo,
        p_next: ptr::null(),
        flags: vk::COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT,
        queue_family_index: queue_family_index,
    };
    let pool = device.create_command_pool(pool_create_info).unwrap();

    let command_buffer_allocate_info = vk::CommandBufferAllocateInfo {
        s_type: vk::StructureType::CommandBufferAllocateInfo,
        p_next: ptr::null(),
        command_buffer_count: 2,
        command_pool: pool,
        level: vk::CommandBufferLevel::Primary,
    };

    let command_buffers = device.allocate_command_buffers(command_buffer_allocate_info).unwrap();
    let setup_command_buffer = command_buffers[0];
    let draw_command_buffer = command_buffers[1];

    let present_images = device.get_swapchain_images_khr(swapchain).unwrap();
    let present_image_views: Vec<vk::ImageView> = present_images.iter()
        .map(|&image| {
            let create_view_info = vk::ImageViewCreateInfo {
                s_type: vk::StructureType::ImageViewCreateInfo,
                p_next: ptr::null(),
                flags: 0,
                view_type: vk::ImageViewType::Type2d,
                format: surface_format.format,
                components: vk::ComponentMapping {
                    r: vk::ComponentSwizzle::R,
                    g: vk::ComponentSwizzle::G,
                    b: vk::ComponentSwizzle::B,
                    a: vk::ComponentSwizzle::A,
                },
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::IMAGE_ASPECT_COLOR_BIT,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                image: image,
            };
            device.create_image_view(&create_view_info).unwrap()
        })
        .collect();
    let device_memory_properties = instance.get_physical_device_memory_properties(pdevice);
    let depth_image_create_info = vk::ImageCreateInfo {
        s_type: vk::StructureType::ImageCreateInfo,
        p_next: ptr::null(),
        flags: vk::IMAGE_CREATE_SPARSE_BINDING_BIT,
        image_type: vk::ImageType::Type2d,
        format: vk::Format::D16Unorm,
        extent: vk::Extent3D {
            width: surface_resoultion.width,
            height: surface_resoultion.height,
            depth: 1,
        },
        mip_levels: 1,
        array_layers: 1,
        samples: vk::SAMPLE_COUNT_1_BIT,
        tiling: vk::ImageTiling::Optimal,
        usage: vk::IMAGE_USAGE_DEPTH_STENCIL_ATTACHMENT_BIT,
        sharing_mode: vk::SharingMode::Exclusive,
        queue_family_index_count: 0,
        p_queue_family_indices: ptr::null(),
        initial_layout: vk::ImageLayout::Undefined,
    };
    let depth_image = device.create_image(&depth_image_create_info).unwrap();
    let depth_image_memory_req = device.get_image_memory_requirements(depth_image);
    let depth_image_memory_index = find_memorytype_index(&depth_image_memory_req,
                                                         &device_memory_properties,
                                                         vk::MEMORY_PROPERTY_DEVICE_LOCAL_BIT)
        .expect("Unable to find suitable memory index for depth image.");

    let depth_image_allocate_info = vk::MemoryAllocateInfo {
        s_type: vk::StructureType::MemoryAllocateInfo,
        p_next: ptr::null(),
        allocation_size: depth_image_memory_req.size,
        memory_type_index: depth_image_memory_index,
    };
    let depth_image_memory = device.allocate_memory(&depth_image_allocate_info).unwrap();
    device.bind_image_memory(depth_image, depth_image_memory, 0)
        .expect("Unable to bind depth image memory");

    let command_buffer_begin_info = vk::CommandBufferBeginInfo {
        s_type: vk::StructureType::CommandBufferBeginInfo,
        p_next: ptr::null(),
        p_inheritance_info: ptr::null(),
        flags: vk::COMMAND_BUFFER_USAGE_ONE_TIME_SUBMIT_BIT,
    };
    device.begin_command_buffer(setup_command_buffer, &command_buffer_begin_info).unwrap();
    let layout_transition_barrier = vk::ImageMemoryBarrier {
        s_type: vk::StructureType::ImageMemoryBarrier,
        p_next: ptr::null(),
        // TODO Is this correct?
        src_access_mask: vk::AccessFlags::empty(),
        dst_access_mask: vk::ACCESS_DEPTH_STENCIL_ATTACHMENT_READ_BIT |
                         vk::ACCESS_DEPTH_STENCIL_ATTACHMENT_WRITE_BIT,
        old_layout: vk::ImageLayout::Undefined,
        new_layout: vk::ImageLayout::DepthStencilAttachmentOptimal,
        src_queue_family_index: vk::VK_QUEUE_FAMILY_IGNORED,
        dst_queue_family_index: vk::VK_QUEUE_FAMILY_IGNORED,
        image: depth_image,
        subresource_range: vk::ImageSubresourceRange {
            aspect_mask: vk::IMAGE_ASPECT_DEPTH_BIT,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        },
    };
    device.cmd_pipeline_barrier(setup_command_buffer,
                                vk::PIPELINE_STAGE_TOP_OF_PIPE_BIT,
                                vk::PIPELINE_STAGE_TOP_OF_PIPE_BIT,
                                vk::DEPENDENCY_BY_REGION_BIT,
                                &[],
                                &[],
                                &[layout_transition_barrier]);
    let wait_stage_mask = [vk::PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT];
    let fence_create_info = vk::FenceCreateInfo {
        s_type: vk::StructureType::FenceCreateInfo,
        p_next: ptr::null(),
        flags: vk::FenceCreateFlags::empty(),
    };
    let submit_fence = device.create_fence(&fence_create_info).unwrap();
    let submit_info = vk::SubmitInfo {
        s_type: vk::StructureType::SubmitInfo,
        p_next: ptr::null(),
        wait_semaphore_count: 0,
        p_wait_semaphores: ptr::null(),
        signal_semaphore_count: 0,
        p_signal_semaphores: ptr::null(),
        p_wait_dst_stage_mask: wait_stage_mask.as_ptr(),
        command_buffer_count: 1,
        p_command_buffers: &setup_command_buffer,
    };
    device.end_command_buffer(setup_command_buffer).unwrap();
    device.queue_submit(present_queue, 1, &submit_info, submit_fence).unwrap();
    device.wait_for_fences(&[submit_fence], true, std::u64::MAX).unwrap();
    let depth_image_view_info = vk::ImageViewCreateInfo {
        s_type: vk::StructureType::ImageViewCreateInfo,
        p_next: ptr::null(),
        flags: 0,
        view_type: vk::ImageViewType::Type2d,
        format: depth_image_create_info.format,
        components: vk::ComponentMapping {
            r: vk::ComponentSwizzle::Identity,
            g: vk::ComponentSwizzle::Identity,
            b: vk::ComponentSwizzle::Identity,
            a: vk::ComponentSwizzle::Identity,
        },
        subresource_range: vk::ImageSubresourceRange {
            aspect_mask: vk::IMAGE_ASPECT_DEPTH_BIT,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        },
        image: depth_image,
    };
    let depth_image_view = device.create_image_view(&depth_image_view_info).unwrap();
    let renderpass_attachments = [vk::AttachmentDescription {
                                      format: surface_format.format,
                                      flags: vk::AttachmentDescriptionFlags::empty(),
                                      samples: vk::SAMPLE_COUNT_1_BIT,
                                      load_op: vk::AttachmentLoadOp::Clear,
                                      store_op: vk::AttachmentStoreOp::Store,
                                      stencil_load_op: vk::AttachmentLoadOp::DontCare,
                                      stencil_store_op: vk::AttachmentStoreOp::DontCare,
                                      initial_layout: vk::ImageLayout::ColorAttachmentOptimal,
                                      final_layout: vk::ImageLayout::ColorAttachmentOptimal,
                                  },
                                  vk::AttachmentDescription {
                                      format: depth_image_create_info.format,
                                      flags: vk::AttachmentDescriptionFlags::empty(),
                                      samples: vk::SAMPLE_COUNT_1_BIT,
                                      load_op: vk::AttachmentLoadOp::Clear,
                                      store_op: vk::AttachmentStoreOp::DontCare,
                                      stencil_load_op: vk::AttachmentLoadOp::DontCare,
                                      stencil_store_op: vk::AttachmentStoreOp::DontCare,
                                      initial_layout:
                                          vk::ImageLayout::DepthStencilAttachmentOptimal,
                                      final_layout: vk::ImageLayout::DepthStencilAttachmentOptimal,
                                  }];
    let color_attachment_ref = vk::AttachmentReference {
        attachment: 0,
        layout: vk::ImageLayout::ColorAttachmentOptimal,
    };
    let depth_attachment_ref = vk::AttachmentReference {
        attachment: 1,
        layout: vk::ImageLayout::DepthStencilAttachmentOptimal,
    };
    let subpass = vk::SubpassDescription {
        color_attachment_count: 1,
        p_color_attachments: &color_attachment_ref,
        p_depth_stencil_attachment: &depth_attachment_ref,
        // TODO: Why is there no wrapper?
        flags: 0,
        pipeline_bind_point: vk::PipelineBindPoint::Graphics,
        input_attachment_count: 0,
        p_input_attachments: ptr::null(),
        p_resolve_attachments: ptr::null(),
        preserve_attachment_count: 0,
        p_preserve_attachments: ptr::null(),
    };
    let renderpass_create_info = vk::RenderPassCreateInfo {
        s_type: vk::StructureType::RenderPassCreateInfo,
        flags: 0,
        p_next: ptr::null(),
        attachment_count: renderpass_attachments.len() as u32,
        p_attachments: renderpass_attachments.as_ptr(),
        subpass_count: 1,
        p_subpasses: &subpass,
        dependency_count: 0,
        p_dependencies: ptr::null(),
    };
    let renderpass = device.create_render_pass(&renderpass_create_info).unwrap();
    let framebuffers: Vec<vk::Framebuffer> = present_image_views.iter()
        .map(|&present_image_view| {
            let framebuffer_attachments = [present_image_view, depth_image_view];
            let frame_buffer_create_info = vk::FramebufferCreateInfo {
                s_type: vk::StructureType::FramebufferCreateInfo,
                p_next: ptr::null(),
                flags: 0,
                render_pass: renderpass,
                attachment_count: framebuffer_attachments.len() as u32,
                p_attachments: framebuffer_attachments.as_ptr(),
                width: surface_resoultion.width,
                height: surface_resoultion.height,
                layers: 1,
            };
            device.create_framebuffer(&frame_buffer_create_info).unwrap()
        })
        .collect();
    let vertex_input_buffer_info = vk::BufferCreateInfo {
        s_type: vk::StructureType::BufferCreateInfo,
        p_next: ptr::null(),
        flags: vk::BufferCreateFlags::empty(),
        size: 3 * std::mem::size_of::<Vertex>() as u64,
        usage: vk::BUFFER_USAGE_VERTEX_BUFFER_BIT,
        sharing_mode: vk::SharingMode::Exclusive,
        queue_family_index_count: 0,
        p_queue_family_indices: ptr::null(),
    };
    let vertex_input_buffer = device.create_buffer(&vertex_input_buffer_info).unwrap();
    let vertex_input_buffer_memory_req = device.get_buffer_memory_requirements(vertex_input_buffer);
    let vertex_input_buffer_memory_index =
        find_memorytype_index(&vertex_input_buffer_memory_req,
                              &device_memory_properties,
                              vk::MEMORY_PROPERTY_HOST_VISIBLE_BIT)
            .expect("Unable to find suitable memorytype for the vertex buffer.");

    let vertex_buffer_allocate_info = vk::MemoryAllocateInfo {
        s_type: vk::StructureType::MemoryAllocateInfo,
        p_next: ptr::null(),
        allocation_size: vertex_input_buffer_memory_req.size,
        memory_type_index: vertex_input_buffer_memory_index,
    };
    let vertex_input_buffer_memory = device.allocate_memory(&vertex_buffer_allocate_info).unwrap();
    let slice = device.map_memory::<Vertex>(vertex_input_buffer_memory,
                              0,
                              vertex_input_buffer_info.size,
                              0)
        .unwrap();
    let vertices = [Vertex {
                        x: -1.0,
                        y: 1.0,
                        z: 0.0,
                        w: 1.0,
                    },
                    Vertex {
                        x: 1.0,
                        y: 1.0,
                        z: 0.0,
                        w: 1.0,
                    },
                    Vertex {
                        x: 0.0,
                        y: -1.0,
                        z: 0.0,
                        w: 1.0,
                    }];

    slice.copy_from_slice(&vertices);
    printlndb!((slice));
    device.unmap_memory(vertex_input_buffer_memory);
    device.bind_buffer_memory(vertex_input_buffer, vertex_input_buffer_memory, 0).unwrap();
    let vertex_spv_file = File::open(Path::new("vert.spv")).expect("Could not find vert.spv.");
    let frag_spv_file = File::open(Path::new("frag.spv")).expect("Could not find frag.spv.");

    let vertex_bytes: Vec<u8> = vertex_spv_file.bytes().filter_map(|byte| byte.ok()).collect();
    let vertex_shader_info = vk::ShaderModuleCreateInfo {
        s_type: vk::StructureType::ShaderModuleCreateInfo,
        p_next: ptr::null(),
        flags: 0,
        code_size: vertex_bytes.len(),
        p_code: vertex_bytes.as_ptr() as *const u32,
    };
    let frag_bytes: Vec<u8> = frag_spv_file.bytes().filter_map(|byte| byte.ok()).collect();
    let frag_shader_info = vk::ShaderModuleCreateInfo {
        s_type: vk::StructureType::ShaderModuleCreateInfo,
        p_next: ptr::null(),
        flags: 0,
        code_size: frag_bytes.len(),
        p_code: frag_bytes.as_ptr() as *const u32,
    };
    let vertex_shader_module = device.create_shader_module(&vertex_shader_info)
        .expect("Vertex shader module error");
    let frag_shader_module = device.create_shader_module(&frag_shader_info)
        .expect("Fragment shader module error");



    device.destroy_shader_module(vertex_shader_module);
    device.destroy_shader_module(frag_shader_module);
    device.free_memory(vertex_input_buffer_memory);
    device.destroy_buffer(vertex_input_buffer);
    for framebuffer in framebuffers {
        device.destroy_framebuffer(framebuffer);
    }
    device.destroy_render_pass(renderpass);
    device.destroy_image_view(depth_image_view);
    device.destroy_fence(submit_fence);
    device.free_memory(depth_image_memory);
    device.destroy_image(depth_image);
    for image_view in present_image_views {
        device.destroy_image_view(image_view);
    }
    device.destroy_command_pool(pool);
    device.destroy_swapchain_khr(swapchain);
    device.destroy_device();
    instance.destroy_surface_khr(surface);
    instance.destroy_instance();
}