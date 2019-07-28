use gfx_hal::Backend;
use gfx_hal::Device;
use shaderc::Compiler;
use std::marker::PhantomData;

pub struct ShaderUtils<B: Backend<Device = D>, D: Device<B>> {
    _backend: PhantomData<B>,
    _device: PhantomData<D>,
}

impl<B, D> ShaderUtils<B, D>
where
    B: Backend<Device = D>,
    D: Device<B>,
{
    pub fn vertex_to_module(
        device: &D,
        compiler: &mut Compiler,
        source: &str,
        entry: &str,
    ) -> Result<B::ShaderModule, &'static str> {
        let artifact = compiler
            .compile_into_spirv(
                source,
                shaderc::ShaderKind::Vertex,
                "vertex.vert",
                entry,
                None,
            )
            .map_err(|_| "Couldn't compile vertex shader!")?;
        Ok(unsafe {
            device
                .create_shader_module(artifact.as_binary_u8())
                .map_err(|_| "Couldn't make the vertex module")?
        })
    }

    pub fn fragment_to_module(
        device: &D,
        compiler: &mut Compiler,
        source: &str,
        entry: &str,
    ) -> Result<B::ShaderModule, &'static str> {
        let artifact = compiler
            .compile_into_spirv(
                source,
                shaderc::ShaderKind::Fragment,
                "fragment.frag",
                entry,
                None,
            )
            .map_err(|_| "Couldn't compile fragment shader!")?;
        Ok(unsafe {
            device
                .create_shader_module(artifact.as_binary_u8())
                .map_err(|_| "Couldn't make the fragment module")?
        })
    }
}
