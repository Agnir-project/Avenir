//!
//! HalState module
//! `HalState` is the generic instance of gfx-hal.
//!

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use gfx_hal::{
    adapter::Adapter,
    command::{ClearColor, ClearValue, CommandBuffer, MultiShot, Primary},
    device::Device,
    format::{Aspects, Format, Swizzle},
    image::{Extent, SubresourceRange, ViewKind},
    pool::{CommandPool, CommandPoolCreateFlags},
    pso::*,
    queue::{QueueGroup, Submission},
    window::{CompositeAlpha, PresentMode, Surface, Swapchain},
    Backend, Graphics, Instance,
};

use std::mem::ManuallyDrop;

use crate::Triangle;
use arrayvec::ArrayVec;
use gfx_hal::buffer;

use crate::back;
use crate::buffer_bundle::BufferBundle;
use crate::gfx_utils::GfxUtils;
use crate::pipeline::{Pipeline, PipelineBuilder};
use crate::utils::{Build, With, WithError};
use gfx_hal::Primitive;

use gfx_hal::window::Suboptimal;
use winit::Window;

/// HalStateOptions is needed by the `HalState::new` function.
/// It initialize the Generic HalState with all the needed informations.
/// (For now it's simple. It may become really heavy in a near future.)
pub struct HalStateOptions<'a> {
    /// Order of the presentation mode.
    pub pm_order: Vec<PresentMode>,

    /// A slice of shader.
    pub shaders: &'a [(shaderc::ShaderKind, String)],

    /// A drawing primitive.
    pub primitive: Primitive,
}

/// HalState is an alias of GenericHalState<B, D, I>.
pub type HalState = GenericHalState<back::Backend, back::Device, back::Instance>;

/// It is the main data-structure.
/// It contain every pieces of information needed to handle a simple draw.
/// The `GenericHalState` contain three Templated type. `Backend<D>`, `Device<B>`, `Instance<B>`.
/// The `Backend` specifies ether metal-api, vulkan-api, blank-api, dx12-api, opengl-api.
/// Thanks to gfx-hal We are fully cross platform.
/// The `Device` specifies the software representation of an hardware entity.
/// The `Instance` specifies the cross platform analog of the `VkInstance`.
pub struct GenericHalState<B: Backend<Device = D>, D: Device<B>, I: Instance<Backend = B>> {
    current_frame: usize,
    frames_in_flight: u32,
    in_flight_fences: Vec<B::Fence>,
    render_finished_semaphores: Vec<B::Semaphore>,
    image_available_semaphores: Vec<B::Semaphore>,
    command_buffers: Vec<CommandBuffer<B, Graphics, MultiShot, Primary>>,
    command_pool: ManuallyDrop<CommandPool<B, Graphics>>,
    framebuffers: Vec<B::Framebuffer>,
    image_views: Vec<B::ImageView>,
    render_pass: ManuallyDrop<B::RenderPass>,
    render_area: Rect,
    queue_group: QueueGroup<B, Graphics>,
    swapchain: ManuallyDrop<B::Swapchain>,
    device: ManuallyDrop<D>,
    vertices: Option<BufferBundle<B, D>>,
    pipeline: ManuallyDrop<Pipeline<B, D>>,
    _adapter: Adapter<B>,
    _surface: B::Surface,
    _instance: ManuallyDrop<I>,
}

impl HalState {
    /// Create a new HalState and initialize it.
    pub fn new(window: &Window, opt: &HalStateOptions) -> Result<Self, &'static str> {
        let instance = back::Instance::create("HalState", 1);
        let surface = instance.create_surface(window);
        HalState::init(&window, instance, surface, opt)
    }
}

impl<B, D, I> GenericHalState<B, D, I>
where
    B: Backend<Device = D>,
    D: Device<B>,
    I: Instance<Backend = B>,
{
    /// initialize the `GenericHalState`
    /// This is a verty important function that may in a near future be splitted into smaller ones.
    /// * We pick an `adapter` from the `instance` given in parameters.
    /// * We get `device` and `queue_group` from the adapter and the surface.
    /// * We get all informations needed from the adapter.
    /// * We get the `swapchain` and the `backbuffer` from the `device`, the `surface` and a `SwapchainConfig` that let we fill with previously gathered informations.
    /// * We get the `render_pass` from `device`.
    /// * We create `semaphore` and `fences` necessary to pipeline synchronisation.
    /// * We create `image_view` from `backbuffer`.
    /// * We create `framebuffers` from `image_view`.
    /// * We create a basic `command_pool` for `queue_group`.
    /// * For each `framebuffer` in `framebuffers` we create a `command_buffer` that we put in `command_buffers`
    /// * We setup some important blending informations.
    /// * We build the `Pipeline` with a `PipelineBuilder`.
    /// * We append all shaders given in `HalStateOptions::shaders` to the `PipelineBuilder`.
    /// * We build the `Pipeline`
    /// * We create the `GenericHalState` from all the previously created datas.
    fn init(
        window: &Window,
        instance: I,
        mut surface: <B>::Surface,
        opt: &HalStateOptions,
    ) -> Result<Self, &'static str> {
        let adapter = GfxUtils::pick_adapter(&instance, &surface)?;

        let (mut device, queue_group) = GfxUtils::<B, D, I>::get_device(&adapter, &surface)?;
        {
            let (caps, available_formats, available_modes) =
                surface.compatibility(&adapter.physical_device);
            info!("{:?}", caps);
            info!("Available Formats: {:?}", available_formats);
            info!("Available Present Modes: {:?}", available_modes);
        };

        let format = GfxUtils::<B, D, I>::get_format(&adapter, &surface)?;
        let extent = GfxUtils::<B, D, I>::get_extent(&adapter, &surface, window)?;
        let present_mode =
            GfxUtils::<B, D, I>::get_present_mode(&adapter, &surface, &opt.pm_order)?;
        let frames_in_flight =
            GfxUtils::<B, D, I>::get_image_count(&adapter, &surface, present_mode);
        let (swapchain, backbuffer) =
            GfxUtils::<B, D, I>::get_swapchain(&adapter, &device, &mut surface, &window)?;
        let render_pass = GfxUtils::<B, D, I>::get_render_pass(format, &device)?;
        let (image_available_semaphores, render_finished_semaphores, in_flight_fences) = {
            let in_flight_fences = ((0..frames_in_flight)
                .map(|_| {
                    let fence = device
                        .create_fence(true)
                        .map_err(|_| "Could not create a fence!")?;
                    Ok(fence)
                })
                .collect(): Result<Vec<_>, _>)?;

            let render_finished_semaphores = ((0..frames_in_flight)
                .map(|_| {
                    let semaphore = device
                        .create_semaphore()
                        .map_err(|_| "Could not create a semaphore!")?;
                    Ok(semaphore)
                })
                .collect(): Result<Vec<_>, _>)?;

            let image_available_semaphores = ((0..frames_in_flight)
                .map(|_| {
                    let semaphore = device
                        .create_semaphore()
                        .map_err(|_| "Could not create a semaphore!")?;
                    Ok(semaphore)
                })
                .collect(): Result<Vec<_>, _>)?;

            (
                image_available_semaphores,
                render_finished_semaphores,
                in_flight_fences,
            )
        };
        let image_views = backbuffer
            .into_iter()
            .map(|image| unsafe {
                device
                    .create_image_view(
                        &image,
                        ViewKind::D2,
                        format,
                        Swizzle::NO,
                        SubresourceRange {
                            aspects: Aspects::COLOR,
                            levels: 0..1,
                            layers: 0..1,
                        },
                    )
                    .map_err(|_| "Couldn't create the image_view for the image!")
            })
            .collect::<Result<Vec<_>, &str>>()?;

        let framebuffers: Vec<B::Framebuffer> = {
            image_views
                .iter()
                .map(|image_view| unsafe {
                    device
                        .create_framebuffer(
                            &render_pass,
                            vec![image_view],
                            Extent {
                                width: extent.width as u32,
                                height: extent.height as u32,
                                depth: 1,
                            },
                        )
                        .map_err(|_| "Failed to create a framebuffer!")
                })
                .collect::<Result<Vec<_>, &str>>()?
        };
        let mut command_pool = unsafe {
            device
                .create_command_pool_typed(&queue_group, CommandPoolCreateFlags::RESET_INDIVIDUAL)
                .map_err(|_| "Could not create the raw command pool!")?
        };
        let command_buffers: Vec<_> = framebuffers
            .iter()
            .map(|_| command_pool.acquire_command_buffer())
            .collect();
        let blend_state = gfx_hal::pso::BlendState {
            color: BlendOp::Add {
                src: Factor::One,
                dst: Factor::Zero,
            },
            alpha: BlendOp::Add {
                src: Factor::One,
                dst: Factor::Zero,
            },
        };
        let mut pipeline_builder = PipelineBuilder::new(&mut device, extent, &render_pass)?
            .with(AttributeDesc {
                // XY
                location: 0,
                binding: 0,
                element: Element {
                    format: Format::Rg32Sfloat,
                    offset: 0,
                },
            })
            .with(AttributeDesc {
                // RGB
                location: 1,
                binding: 0,
                element: Element {
                    format: Format::Rgb32Sfloat,
                    offset: (std::mem::size_of::<f32>() * 2) as ElemOffset,
                },
            })
            .with(opt.primitive)
            .with(VertexBufferDesc {
                binding: 0,
                stride: (std::mem::size_of::<f32>() * 5) as u32,
                rate: VertexInputRate::Vertex,
            })
            .with(Rasterizer {
                depth_clamping: false,
                polygon_mode: PolygonMode::Fill,
                cull_face: Face::NONE,
                front_face: FrontFace::Clockwise,
                depth_bias: None,
                conservative: false,
            })
            .with(DepthStencilDesc {
                depth: None,
                depth_bounds: false,
                stencil: None,
            })
            .with(BlendDesc {
                logic_op: Some(LogicOp::Copy),
                targets: vec![ColorBlendDesc {
                    mask: ColorMask::ALL,
                    blend: Some(blend_state),
                }],
            })
            .with(BakedStates {
                viewport: Some(Viewport {
                    rect: extent.to_extent().rect(),
                    depth: (0.0..1.0),
                }),
                scissor: Some(extent.to_extent().rect()),
                blend_color: None,
                depth_bounds: None,
            });

        for item in opt.shaders {
            pipeline_builder = pipeline_builder.with_error(&item)?;
        }
        let pipeline = pipeline_builder.build()?;
        //let vertices = BufferBundle::new(
        //&adapter,
        //&device,
        //F32_XY_RGB_TRIANGLE,
        //buffer::Usage::VERTEX,
        //)?;
        Ok(Self {
            _instance: ManuallyDrop::new(instance),
            _surface: surface,
            _adapter: adapter,
            device: ManuallyDrop::new(device),
            queue_group,
            swapchain: ManuallyDrop::new(swapchain),
            render_area: extent.to_extent().rect(),
            render_pass: ManuallyDrop::new(render_pass),
            image_views,
            framebuffers,
            command_pool: ManuallyDrop::new(command_pool),
            command_buffers,
            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
            frames_in_flight,
            current_frame: 0,
            vertices: None,
            pipeline: ManuallyDrop::new(pipeline),
        })
    }

    /// Set a buffer bundle.
    pub fn set_buffer_bundle(&mut self, size: usize) -> Result<(), &'static str> {
        self.vertices = Some(BufferBundle::new(
            &self._adapter,
            &*self.device,
            size,
            buffer::Usage::VERTEX,
        )?);

        Ok(())
    }

    /// Draw a a given triangle.
    /// It's a big function again and it will certainly be splitted or reworked.
    pub fn draw_triangle_frame(
        &mut self,
        triangle: Triangle,
    ) -> Result<Option<Suboptimal>, &'static str> {
        // SETUP FOR THIS FRAME
        let image_available = &self.image_available_semaphores[self.current_frame];
        let render_finished = &self.render_finished_semaphores[self.current_frame];
        // Advance the frame _before_ we start using the `?` operator
        self.current_frame = (self.current_frame + 1) % (self.frames_in_flight as usize);

        let (i_u32, i_usize) = unsafe {
            let image_index = self
                .swapchain
                .acquire_image(core::u64::MAX, Some(image_available), None)
                .map_err(|_| "Couldn't acquire an image from the swapchain!")?;
            (image_index.0, image_index.0 as usize)
        };

        let flight_fence = &self.in_flight_fences[i_usize];
        unsafe {
            self.device
                .wait_for_fence(flight_fence, core::u64::MAX)
                .map_err(|_| "Failed to wait on the fence!")?;
            self.device
                .reset_fence(flight_fence)
                .map_err(|_| "Couldn't reset the fence!")?;
        }

        // WRITE THE TRIANGLE DATA
        unsafe {
            let vertices = self.vertices.as_ref().ok_or("Cannot find buffer bundle")?;

            let mut data_target = self
                .device
                .acquire_mapping_writer(&vertices.memory, 0..vertices.requirements.size)
                .map_err(|_| "Failed to acquire a memory writer!")?;
            let points = triangle.vertex_attributes();
            data_target[..points.len()].copy_from_slice(&points);
            self.device
                .release_mapping_writer(data_target)
                .map_err(|_| "Couldn't release the mapping writer!")?;
        }

        // RECORD COMMANDS
        unsafe {
            let vertices = self.vertices.as_ref().ok_or("Cannot find buffer bundle.")?;

            let buffer = &mut self.command_buffers[i_usize];
            const TRIANGLE_CLEAR: [ClearValue; 1] =
                [ClearValue::Color(ClearColor::Sfloat([0.1, 0.2, 0.3, 1.0]))];
            buffer.begin(false);
            {
                let mut encoder = buffer.begin_render_pass_inline(
                    &self.render_pass,
                    &self.framebuffers[i_usize],
                    self.render_area,
                    TRIANGLE_CLEAR.iter(),
                );
                encoder.bind_graphics_pipeline(&self.pipeline.graphics_pipeline);

                // Here we must force the Deref impl of ManuallyDrop to play nice.
                let buffer_ref: &B::Buffer = &vertices.buffer;
                let buffers: ArrayVec<[_; 1]> = [(buffer_ref, 0)].into();
                encoder.bind_vertex_buffers(0, buffers);
                encoder.draw(0..3, 0..1);
            }
            buffer.finish();
        }

        // SUBMISSION AND PRESENT
        let command_buffers = &self.command_buffers[i_usize..=i_usize];
        let wait_semaphores: ArrayVec<[_; 1]> =
            [(image_available, PipelineStage::COLOR_ATTACHMENT_OUTPUT)].into();
        let signal_semaphores: ArrayVec<[_; 1]> = [render_finished].into();
        // yes, you have to write it twice like this. yes, it's silly.
        let present_wait_semaphores: ArrayVec<[_; 1]> = [render_finished].into();
        let submission = Submission {
            command_buffers,
            wait_semaphores,
            signal_semaphores,
        };
        let the_command_queue = &mut self.queue_group.queues[0];
        unsafe {
            the_command_queue.submit(submission, Some(flight_fence));
            self.swapchain
                .present(the_command_queue, i_u32, present_wait_semaphores)
                .map_err(|_| "Failed to present into the swapchain!")
        }
    }
}
