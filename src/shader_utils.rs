use gfx_hal::Backend;
use gfx_hal::Device;
use shaderc::Compiler;
use std::marker::PhantomData;

pub struct ShaderUtils<B: Backend<Device = D>, D: Device<B>> {
    _backend: PhantomData<B>,
    _device: PhantomData<D>,
}

pub const DEFAULT_VERTEX_SOURCE: &str = "
#version 450
void main() {
}";

impl<B, D> ShaderUtils<B, D>
where
    B: Backend<Device = D>,
    D: Device<B>,
{

    pub fn source_to_artifact(
        compiler: &mut Compiler,
        kind: shaderc::ShaderKind,
        source: &str,
        entry: &str,
    ) -> Result<shaderc::CompilationArtifact, &'static str> {
        Ok(compiler
            .compile_into_spirv(
                source,
                kind,
                "vertex.vert",
                entry,
                None,
            )
            .map_err(|_| "Couldn't compile vertex shader!")?
        )
    }

    pub fn artifact_to_module(
        device: &D,
        artifact: shaderc::CompilationArtifact
    ) -> Result<B::ShaderModule, &'static str> {
        Ok(unsafe {
            device
                .create_shader_module(artifact.as_binary_u8())
                .map_err(|_| "Couldn't make the vertex module")?
        })
    }

    pub fn source_to_module(
        device: &D,
        compiler: &mut Compiler,
        kind: shaderc::ShaderKind,
        source: &str,
        entry: &str,
    ) -> Result<B::ShaderModule, &'static str> {
        let artifact = Self::source_to_artifact(compiler, kind, source, entry)?;
        Self::artifact_to_module(device, artifact)
    }

}
