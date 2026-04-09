//! Fine-tuning namespace implementations.

use std::collections::BTreeMap;

use http::Method;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::generated::endpoints;
use crate::json_payload::JsonPayload;

use super::{
    DeleteResponse, FineTuningAlphaGradersResource, FineTuningAlphaResource, FineTuningCheckpoint,
    FineTuningCheckpointPermission, FineTuningCheckpointPermissionsResource, FineTuningJob,
    FineTuningJobCheckpointsResource, FineTuningJobCreateRequestBuilder, FineTuningJobEvent,
    FineTuningJobsResource, FineTuningResource, GradersResource, JsonRequestBuilder,
    ListRequestBuilder, encode_path_segment,
};

/// 表示 grader 执行结果。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GraderRunResponse {
    /// grader 元数据。
    pub metadata: Option<GraderRunMetadata>,
    /// 按模型拆分的 token 使用情况。
    #[serde(default)]
    pub model_grader_token_usage_per_model: BTreeMap<String, Value>,
    /// 总 reward。
    pub reward: Option<f64>,
    /// 子 reward。
    #[serde(default)]
    pub sub_rewards: BTreeMap<String, Value>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 grader 运行元数据。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GraderRunMetadata {
    /// 错误位图。
    pub errors: Option<GraderRunErrors>,
    /// 执行时间。
    pub execution_time: Option<f64>,
    /// grader 名称。
    pub name: Option<String>,
    /// 被评估模型名称。
    pub sampled_model_name: Option<String>,
    /// 分数字段。
    #[serde(default)]
    pub scores: BTreeMap<String, Value>,
    /// token 使用量。
    pub token_usage: Option<u64>,
    /// grader 类型。
    #[serde(rename = "type")]
    pub grader_type: Option<String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 grader 执行错误标记。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GraderRunErrors {
    /// 公式解析错误。
    #[serde(default)]
    pub formula_parse_error: bool,
    /// 非法变量错误。
    #[serde(default)]
    pub invalid_variable_error: bool,
    /// 模型 grader 解析错误。
    #[serde(default)]
    pub model_grader_parse_error: bool,
    /// 模型 grader 拒绝错误。
    #[serde(default)]
    pub model_grader_refusal_error: bool,
    /// 模型 grader 服务端错误。
    #[serde(default)]
    pub model_grader_server_error: bool,
    /// 模型 grader 服务端错误细节。
    pub model_grader_server_error_details: Option<String>,
    /// 其他错误。
    #[serde(default)]
    pub other_error: bool,
    /// Python grader 运行时错误。
    #[serde(default)]
    pub python_grader_runtime_error: bool,
    /// Python grader 运行时错误细节。
    pub python_grader_runtime_error_details: Option<String>,
    /// Python grader 服务端错误。
    #[serde(default)]
    pub python_grader_server_error: bool,
    /// Python grader 服务端错误类型。
    pub python_grader_server_error_type: Option<String>,
    /// 样本解析错误。
    #[serde(default)]
    pub sample_parse_error: bool,
    /// 截断观测错误。
    #[serde(default)]
    pub truncated_observation_error: bool,
    /// 无响应 reward 错误。
    #[serde(default)]
    pub unresponsive_reward_error: bool,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 grader 校验结果。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GraderValidateResponse {
    /// 返回的 grader 定义。
    pub grader: Option<JsonPayload>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 grader model 列表。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GraderModelCatalog {
    /// 列表对象类型。
    pub object: Option<String>,
    /// grader models。
    #[serde(default)]
    pub data: Vec<GraderModel>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示单个 grader model。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GraderModel {
    /// grader model ID。
    pub id: Option<String>,
    /// grader 名称。
    pub name: Option<String>,
    /// grader 类型。
    #[serde(rename = "type")]
    pub grader_type: Option<String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl FineTuningResource {
    /// 返回 jobs 子资源。
    pub fn jobs(&self) -> FineTuningJobsResource {
        FineTuningJobsResource::new(self.client.clone())
    }

    /// 返回 checkpoints permissions 子资源。
    pub fn checkpoints(&self) -> FineTuningCheckpointPermissionsResource {
        FineTuningCheckpointPermissionsResource::new(self.client.clone())
    }

    /// 返回 alpha 子资源。
    pub fn alpha(&self) -> FineTuningAlphaResource {
        FineTuningAlphaResource::new(self.client.clone())
    }
}

impl FineTuningJobsResource {
    /// 创建 fine-tuning job。
    pub fn create(&self) -> FineTuningJobCreateRequestBuilder {
        FineTuningJobCreateRequestBuilder::new(self.client.clone())
    }

    /// 获取 fine-tuning job。
    pub fn retrieve(&self, job_id: impl Into<String>) -> JsonRequestBuilder<FineTuningJob> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "fine_tuning.jobs.retrieve",
            Method::GET,
            format!("/fine_tuning/jobs/{}", encode_path_segment(job_id.into())),
        )
    }

    /// 列出 fine-tuning jobs。
    pub fn list(&self) -> ListRequestBuilder<FineTuningJob> {
        ListRequestBuilder::new(
            self.client.clone(),
            "fine_tuning.jobs.list",
            "/fine_tuning/jobs",
        )
    }

    /// 取消 fine-tuning job。
    pub fn cancel(&self, job_id: impl Into<String>) -> JsonRequestBuilder<FineTuningJob> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "fine_tuning.jobs.cancel",
            Method::POST,
            format!(
                "/fine_tuning/jobs/{}/cancel",
                encode_path_segment(job_id.into())
            ),
        )
    }

    /// 暂停 fine-tuning job。
    pub fn pause(&self, job_id: impl Into<String>) -> JsonRequestBuilder<FineTuningJob> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "fine_tuning.jobs.pause",
            Method::POST,
            format!(
                "/fine_tuning/jobs/{}/pause",
                encode_path_segment(job_id.into())
            ),
        )
    }

    /// 恢复 fine-tuning job。
    pub fn resume(&self, job_id: impl Into<String>) -> JsonRequestBuilder<FineTuningJob> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "fine_tuning.jobs.resume",
            Method::POST,
            format!(
                "/fine_tuning/jobs/{}/resume",
                encode_path_segment(job_id.into())
            ),
        )
    }

    /// 列出事件。
    pub fn list_events(&self, job_id: impl Into<String>) -> ListRequestBuilder<FineTuningJobEvent> {
        ListRequestBuilder::new(
            self.client.clone(),
            "fine_tuning.jobs.list_events",
            format!(
                "/fine_tuning/jobs/{}/events",
                encode_path_segment(job_id.into())
            ),
        )
    }

    /// 返回 checkpoints 子资源。
    pub fn checkpoints(&self) -> FineTuningJobCheckpointsResource {
        FineTuningJobCheckpointsResource::new(self.client.clone())
    }
}

impl FineTuningJobCheckpointsResource {
    /// 列出某个 job 的 checkpoints。
    pub fn list(&self, job_id: impl Into<String>) -> ListRequestBuilder<FineTuningCheckpoint> {
        ListRequestBuilder::new(
            self.client.clone(),
            "fine_tuning.jobs.checkpoints.list",
            format!(
                "/fine_tuning/jobs/{}/checkpoints",
                encode_path_segment(job_id.into())
            ),
        )
    }
}

impl FineTuningCheckpointPermissionsResource {
    /// 创建 checkpoint permission。
    pub fn create(
        &self,
        checkpoint_id: impl Into<String>,
    ) -> JsonRequestBuilder<FineTuningCheckpointPermission> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "fine_tuning.checkpoints.permissions.create",
            Method::POST,
            format!(
                "/fine_tuning/checkpoints/{}/permissions",
                encode_path_segment(checkpoint_id.into())
            ),
        )
    }

    /// 获取 checkpoint permission。
    pub fn retrieve(
        &self,
        checkpoint_id: impl Into<String>,
        permission_id: impl Into<String>,
    ) -> JsonRequestBuilder<FineTuningCheckpointPermission> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "fine_tuning.checkpoints.permissions.retrieve",
            Method::GET,
            format!(
                "/fine_tuning/checkpoints/{}/permissions/{}",
                encode_path_segment(checkpoint_id.into()),
                encode_path_segment(permission_id.into())
            ),
        )
    }

    /// 列出 checkpoint permission。
    pub fn list(
        &self,
        checkpoint_id: impl Into<String>,
    ) -> ListRequestBuilder<FineTuningCheckpointPermission> {
        ListRequestBuilder::new(
            self.client.clone(),
            "fine_tuning.checkpoints.permissions.list",
            format!(
                "/fine_tuning/checkpoints/{}/permissions",
                encode_path_segment(checkpoint_id.into())
            ),
        )
    }

    /// 删除 checkpoint permission。
    pub fn delete(
        &self,
        checkpoint_id: impl Into<String>,
        permission_id: impl Into<String>,
    ) -> JsonRequestBuilder<DeleteResponse> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "fine_tuning.checkpoints.permissions.delete",
            Method::DELETE,
            format!(
                "/fine_tuning/checkpoints/{}/permissions/{}",
                encode_path_segment(checkpoint_id.into()),
                encode_path_segment(permission_id.into())
            ),
        )
    }
}

impl FineTuningAlphaResource {
    /// 返回 graders 子资源。
    pub fn graders(&self) -> FineTuningAlphaGradersResource {
        FineTuningAlphaGradersResource::new(self.client.clone())
    }
}

impl FineTuningAlphaGradersResource {
    /// 执行 grader。
    pub fn run(&self) -> JsonRequestBuilder<GraderRunResponse> {
        let endpoint = endpoints::fine_tuning::FINE_TUNING_ALPHA_GRADERS_RUN;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::POST,
            endpoint.template,
        )
    }

    /// 校验 grader。
    pub fn validate(&self) -> JsonRequestBuilder<GraderValidateResponse> {
        let endpoint = endpoints::fine_tuning::FINE_TUNING_ALPHA_GRADERS_VALIDATE;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::POST,
            endpoint.template,
        )
    }
}

impl GradersResource {
    /// 当前资源主要导出类型，暂不提供额外 HTTP 方法。
    pub fn grader_models(&self) -> JsonRequestBuilder<GraderModelCatalog> {
        let endpoint = endpoints::fine_tuning::GRADERS_GRADER_MODELS;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::GET,
            endpoint.template,
        )
    }
}
