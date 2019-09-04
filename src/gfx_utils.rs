use gfx_hal::{
    adapter::{Adapter, PhysicalDevice},
    device::Device,
    format::{ChannelType, Format},
    image::{Layout, Usage},
    pass::{Attachment, AttachmentLoadOp, AttachmentOps, AttachmentStoreOp, SubpassDesc},
    queue::{family::QueueGroup, QueueFamily},
    window::{Backbuffer, CompositeAlpha, Extent2D, PresentMode, Surface, SwapchainConfig},
    {Backend, Gpu, Graphics, Instance},
};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::marker::PhantomData;
use winit::Window;

pub struct GfxUtils<B: Backend<Device = D>, D: Device<B>, I: Instance<Backend = B>> {
    _backend: PhantomData<B>,
    _device: PhantomData<D>,
    _instance: PhantomData<I>,
}

impl<B, D, I> GfxUtils<B, D, I>
where
    B: Backend<Device = D>,
    D: Device<B>,
    I: Instance<Backend = B>,
{
    ///
    /// Choose an adapter from the ones that are availables
    ///
    pub fn pick_adapter(instance: &I, surface: &B::Surface) -> Result<Adapter<B>, &'static str> {
        Ok(instance
            .enumerate_adapters()
            .into_iter()
            .find(|a| {
                a.queue_families
                    .iter()
                    .any(|qf| qf.supports_graphics() && surface.supports_queue_family(qf))
            })
            .ok_or("Couldn't find a graphical Adapter!")?)
    }

    ///
    /// Get a queue family that support graphics and that is supported by the surface
    ///
    pub fn get_queue_family<'a>(
        adapter: &'a Adapter<B>,
        surface: &B::Surface,
    ) -> Result<&'a B::QueueFamily, &'static str> {
        Ok(adapter
            .queue_families
            .iter()
            .find(|qf| qf.supports_graphics() && surface.supports_queue_family(qf))
            .ok_or("Couldn't find a QueueFamily with graphics!")?)
    }

    ///
    /// Get the render pass
    /// TODO: Modify hyperparameters
    ///
    pub fn get_render_pass(format: Format, device: &D) -> Result<B::RenderPass, &'static str> {
        let color_attachment = Attachment {
            format: Some(format),
            samples: 1,
            ops: AttachmentOps {
                load: AttachmentLoadOp::Clear,
                store: AttachmentStoreOp::Store,
            },
            stencil_ops: AttachmentOps::DONT_CARE,
            layouts: Layout::Undefined..Layout::Present,
        };
        let subpass = SubpassDesc {
            colors: &[(0, Layout::ColorAttachmentOptimal)],
            depth_stencil: None,
            inputs: &[],
            resolves: &[],
            preserves: &[],
        };
        unsafe {
            Ok(device
                .create_render_pass(&[color_attachment], &[subpass], &[])
                .map_err(|_| "Couldn't create a render pass!")?)
        }
    }

    ///
    /// Get Device and QueueGroup.
    /// Once a correct QueueFamily (see above) has been found, it query the PhysicalDevice from the provided adapter.
    /// This will fail if the device is not an actual GPU.
    /// It then tries to take ownership of the QueueGroup using the QueueFamily id.
    /// Ultimately, it returns both structures.
    ///
    pub fn get_device(
        adapter: &Adapter<B>,
        surface: &B::Surface,
    ) -> Result<(D, QueueGroup<B, Graphics>), &'static str> {
        let queue_family = Self::get_queue_family(&adapter, &surface)?;
        let Gpu { device, mut queues } = unsafe {
            adapter
                .physical_device
                .open(&[(&queue_family, &[1.0; 1])])
                .map_err(|_| "Couldn't open the PhysicalDevice!")?
        };
        let queue_group = queues
            .take::<Graphics>(queue_family.id())
            .ok_or("Couldn't take ownership of the QueueGroup!")?;
        let _ = if queue_group.queues.len() > 0 {
            Ok(())
        } else {
            Err("The QueueGroup did not have any CommandQueues available!")
        }?;
        Ok((device, queue_group))
    }

    pub fn get_present_mode(
        adapter: &Adapter<B>,
        surface: &B::Surface,
        preferred_modes: &Vec<PresentMode>,
    ) -> Result<PresentMode, &'static str> {
        let (_, _, present_modes) = surface.compatibility(&adapter.physical_device);
        Ok(preferred_modes
            .iter()
            .cloned()
            .find(|pm| present_modes.contains(pm))
            .ok_or("No PresentMode values specified!")?)
    }

    pub fn get_composite_alpha(
        adapter: &Adapter<B>,
        surface: &B::Surface,
        preferred_modes: &Vec<CompositeAlpha>,
    ) -> Result<CompositeAlpha, &'static str> {
        let (_, _, _, composite_alphas) = surface.compatibility(&adapter.physical_device);
        Ok(preferred_modes
            .iter()
            .cloned()
            .find(|ca| composite_alphas.contains(ca))
            .ok_or("No CompositeAlpha values specified!")?)
    }

    pub fn get_format(adapter: &Adapter<B>, surface: &B::Surface) -> Result<Format, &'static str> {
        let (_, available_formats, _) = surface.compatibility(&adapter.physical_device);
        Ok(match available_formats {
            None => Format::Rgba8Srgb,
            Some(formats) => match formats
                .iter()
                .find(|format| format.base_format().1 == ChannelType::Srgb)
                .cloned()
            {
                Some(srgb_format) => srgb_format,
                None => formats
                    .get(0)
                    .cloned()
                    .ok_or("Available format list was empty!")?,
            },
        })
    }

    pub fn get_extent(
        adapter: &Adapter<B>,
        surface: &B::Surface,
        window: &Window,
    ) -> Result<Extent2D, &'static str> {
        let (caps, _, _) = surface.compatibility(&adapter.physical_device);
        let window_client_area = window
            .get_inner_size()
            .ok_or("Window doesn't exist!")?
            .to_physical(window.get_hidpi_factor());
        Ok(Extent2D {
            width: caps.extents.end.width.min(window_client_area.width as u32),
            height: caps
                .extents
                .end
                .height
                .min(window_client_area.height as u32),
        })
    }

    pub fn get_image_count(
        adapter: &Adapter<B>,
        surface: &B::Surface,
        present_mode: PresentMode,
    ) -> u32 {
        let (caps, _, _) = surface.compatibility(&adapter.physical_device);
        if present_mode == PresentMode::Mailbox {
            (caps.image_count.end - 1).min(caps.image_count.start.max(3))
        } else {
            (caps.image_count.end - 1).min(caps.image_count.start.max(2))
        }
    }

    pub fn get_image_usage(
        adapter: &Adapter<B>,
        surface: &B::Surface,
    ) -> Result<Usage, &'static str> {
        let (caps, _, _) = surface.compatibility(&adapter.physical_device);
        if caps.usage.contains(Usage::COLOR_ATTACHMENT) {
            Ok(Usage::COLOR_ATTACHMENT)
        } else {
            Err("The Surface isn't capable of supporting color!")
        }
    }

    pub fn get_swapchain(
        device: &D,
        surface: &mut B::Surface,
        config: SwapchainConfig,
    ) -> Result<(B::Swapchain, B::Image), &'static str> {
        let (swapchain, image) = unsafe {
            device
                .create_swapchain(surface, config, None)
                .map_err(|_| "Failed to create the swapchain!")?
        };
        Ok((swapchain, image))
    }
}
