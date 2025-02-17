use axum::extract::{Path, State};
use compositor_pipeline::pipeline::Port;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    error::ApiError,
    routes::Json,
    state::{Pipeline, Response},
    types::{
        ImageSpec, InputId, Mp4, OutputId, RendererId, RtpInputStream, RtpOutputStream, ShaderSpec,
        WebRendererSpec,
    },
};

use super::ApiState;

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RegisterInput {
    RtpStream(RtpInputStream),
    Mp4(Mp4),
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RegisterOutput {
    RtpStream(RtpOutputStream),
}

pub(super) async fn handle_input(
    State(api): State<ApiState>,
    Path(input_id): Path<InputId>,
    Json(request): Json<RegisterInput>,
) -> Result<Response, ApiError> {
    let api = api.clone();
    tokio::task::spawn_blocking(move || {
        let response = match request {
            RegisterInput::RtpStream(rtp) => {
                Pipeline::register_input(&api.pipeline, input_id.into(), rtp.try_into()?)?
            }
            RegisterInput::Mp4(mp4) => {
                Pipeline::register_input(&api.pipeline, input_id.into(), mp4.try_into()?)?
            }
        };
        match response {
            Some(Port(port)) => Ok(Response::RegisteredPort { port }),
            None => Ok(Response::Ok {}),
        }
    })
    .await
    // `unwrap()` panics only when the task panicked or `response.abort()` was called
    .unwrap()
}

pub(super) async fn handle_output(
    State(api): State<ApiState>,
    Path(output_id): Path<OutputId>,
    Json(request): Json<RegisterOutput>,
) -> Result<Response, ApiError> {
    let api = api.clone();
    tokio::task::spawn_blocking(move || {
        let response = match request {
            RegisterOutput::RtpStream(rtp) => {
                Pipeline::register_output(&mut api.pipeline(), output_id.into(), rtp.try_into()?)?
            }
        };
        match response {
            Some(Port(port)) => Ok(Response::RegisteredPort { port }),
            None => Ok(Response::Ok {}),
        }
    })
    .await
    .unwrap()
}

pub(super) async fn handle_shader(
    State(api): State<ApiState>,
    Path(shader_id): Path<RendererId>,
    Json(request): Json<ShaderSpec>,
) -> Result<Response, ApiError> {
    let api = api.clone();
    tokio::task::spawn_blocking(move || {
        Pipeline::register_renderer(&api.pipeline, shader_id.into(), request.try_into()?)?;
        Ok(Response::Ok {})
    })
    .await
    .unwrap()
}

pub(super) async fn handle_web_renderer(
    State(api): State<ApiState>,
    Path(instance_id): Path<RendererId>,
    Json(request): Json<WebRendererSpec>,
) -> Result<Response, ApiError> {
    let api = api.clone();
    tokio::task::spawn_blocking(move || {
        Pipeline::register_renderer(&api.pipeline, instance_id.into(), request.try_into()?)?;
        Ok(Response::Ok {})
    })
    .await
    .unwrap()
}

pub(super) async fn handle_image(
    State(api): State<ApiState>,
    Path(image_id): Path<RendererId>,
    Json(request): Json<ImageSpec>,
) -> Result<Response, ApiError> {
    let api = api.clone();
    tokio::task::spawn_blocking(move || {
        Pipeline::register_renderer(&api.pipeline, image_id.into(), request.try_into()?)?;
        Ok(Response::Ok {})
    })
    .await
    .unwrap()
}
