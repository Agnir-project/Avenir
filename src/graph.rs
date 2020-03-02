use rendy::{
    command::{
        CommandBuffer, CommandPool, Compute, DrawCommand, DrawIndexedCommand, ExecutableState,
        Families, Family, MultiShot, PendingState, QueueId, RenderPassEncoder, SimultaneousUse,
        Submit,
    },
    core::EnabledBackend,
    factory::{BufferState, Config, Factory},
    frame::Frames,
    graph::{
        gfx_acquire_barriers, gfx_release_barriers,
        present::PresentNode,
        render::{
            Layout, PrepareResult, RenderGroupBuilder, SetLayout, SimpleGraphicsPipeline,
            SimpleGraphicsPipelineDesc,
        },
        BufferAccess, Graph, GraphBuildError, GraphBuilder, GraphContext, Node, NodeBuffer,
        NodeBuildError, NodeDesc, NodeImage, NodeSubmittable,
    },
    hal::{self, adapter::PhysicalDevice, device::Device, pso::CreationError},
    init::{
        winit::{
            dpi::{LogicalSize, PhysicalSize},
            event::{Event, VirtualKeyCode, WindowEvent},
            event_loop::{ControlFlow, EventLoop},
            window::{Window, WindowBuilder},
        },
        AnyWindowedRendy,
    },
    memory::Dynamic,
    mesh::{Indices, Mesh, Model, PosColorNorm},
    resource::{Buffer, BufferInfo, DescriptorSet, DescriptorSetLayout, Escape, Handle},
    wsi::Surface,
};

pub fn build<B>(
    mut families: &mut Families<B>,
    window: &Window,
    mut factory: &mut Factory<B>,
    surface: Surface<B>,
) -> Result<Graph<B, ()>, GraphBuildError>
where
    B: hal::Backend,
{
    let mut graph_builder = GraphBuilder::<B, ()>::new();

    let size = window.inner_size();

    let window_kind = hal::image::Kind::D2(size.width as u32, size.height as u32, 1, 1);

    // Create the depth stencil image.
    let depth = graph_builder.create_image(
        window_kind,
        1,
        hal::format::Format::D32Sfloat,
        Some(hal::command::ClearValue {
            depth_stencil: hal::command::ClearDepthStencil {
                depth: 1.,
                stencil: 0,
            },
        }),
    );

    let _meshpass = graph_builder.add_node(
        crate::mesh::Pipeline::builder()
            .into_subpass()
            .with_depth_stencil(depth)
            .with_color_surface()
            .into_pass()
            .with_surface(
                surface,
                hal::window::Extent2D {
                    width: size.width as _,
                    height: size.height as _,
                },
                Some(hal::command::ClearValue {
                    color: hal::command::ClearColor {
                        float32: [0.5, 0.0, 0.5, 1.0],
                    },
                }),
            ),
    );

    graph_builder.build(&mut factory, &mut families, &())
}
