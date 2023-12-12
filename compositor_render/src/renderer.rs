use std::sync::Arc;
use std::time::Duration;

use crate::{
    error::{InitRendererEngineError, RenderSceneError, UpdateSceneError},
    transformations::{
        text_renderer::TextRendererCtx, web_renderer::chromium_context::ChromiumContext,
    },
    types::Framerate,
    FrameSet, InputId, OutputId, WebRendererOptions,
};
use crate::{
    scene::{self, SceneState},
    wgpu::{WgpuCtx, WgpuErrorScope},
};

use self::{
    render_graph::RenderGraph,
    render_loop::{populate_inputs, read_outputs, run_transforms},
    renderers::Renderers,
};

pub mod node;
pub mod render_graph;
mod render_loop;
pub mod renderers;

pub(crate) use render_loop::NodeRenderPass;

pub struct RendererOptions {
    pub web_renderer: WebRendererOptions,
    pub framerate: Framerate,
    pub stream_fallback_timeout: Duration,
}

pub struct Renderer {
    pub(crate) wgpu_ctx: Arc<WgpuCtx>,
    pub(crate) text_renderer_ctx: TextRendererCtx,
    pub(crate) chromium_context: Arc<ChromiumContext>,

    pub(crate) render_graph: RenderGraph,
    pub(crate) scene: SceneState,

    pub(crate) renderers: Renderers,

    stream_fallback_timeout: Duration,
}

pub struct RenderCtx<'a> {
    pub(crate) wgpu_ctx: &'a Arc<WgpuCtx>,

    pub(crate) text_renderer_ctx: &'a TextRendererCtx,

    pub(crate) renderers: &'a Renderers,

    pub(crate) stream_fallback_timeout: Duration,
}

pub struct RegisterCtx {
    pub wgpu_ctx: Arc<WgpuCtx>,
    pub chromium: Arc<ChromiumContext>,
}

impl Renderer {
    pub fn new(opts: RendererOptions) -> Result<Self, InitRendererEngineError> {
        let wgpu_ctx = Arc::new(WgpuCtx::new()?);

        Ok(Self {
            wgpu_ctx: wgpu_ctx.clone(),
            text_renderer_ctx: TextRendererCtx::new(),
            chromium_context: Arc::new(ChromiumContext::new(opts.web_renderer, opts.framerate)?),
            render_graph: RenderGraph::empty(),
            renderers: Renderers::new(wgpu_ctx)?,
            stream_fallback_timeout: opts.stream_fallback_timeout,
            scene: SceneState::new(),
        })
    }

    pub(super) fn register_ctx(&self) -> RegisterCtx {
        RegisterCtx {
            wgpu_ctx: self.wgpu_ctx.clone(),
            chromium: self.chromium_context.clone(),
        }
    }

    pub fn render(
        &mut self,
        mut inputs: FrameSet<InputId>,
    ) -> Result<FrameSet<OutputId>, RenderSceneError> {
        let ctx = &mut RenderCtx {
            wgpu_ctx: &self.wgpu_ctx,
            text_renderer_ctx: &self.text_renderer_ctx,
            renderers: &self.renderers,
            stream_fallback_timeout: self.stream_fallback_timeout,
        };

        let scope = WgpuErrorScope::push(&ctx.wgpu_ctx.device);

        let input_resolutions = inputs
            .frames
            .iter()
            .map(|(input_id, frame)| (input_id.clone(), frame.resolution))
            .collect();
        self.scene
            .register_render_event(inputs.pts, input_resolutions);

        populate_inputs(ctx, &mut self.render_graph, &mut inputs).unwrap();
        run_transforms(ctx, &mut self.render_graph, inputs.pts).unwrap();
        let frames = read_outputs(ctx, &mut self.render_graph, inputs.pts).unwrap();

        scope.pop(&ctx.wgpu_ctx.device)?;

        Ok(FrameSet {
            frames,
            pts: inputs.pts,
        })
    }

    pub fn update_scene(
        &mut self,
        scenes: Vec<scene::OutputScene>,
    ) -> Result<(), UpdateSceneError> {
        let output_nodes =
            self.scene
                .update_scene(scenes, &self.renderers, &self.text_renderer_ctx)?;
        self.render_graph.update(
            &RenderCtx {
                wgpu_ctx: &self.wgpu_ctx,
                text_renderer_ctx: &self.text_renderer_ctx,
                renderers: &self.renderers,
                stream_fallback_timeout: self.stream_fallback_timeout,
            },
            output_nodes,
        )?;
        Ok(())
    }
}
