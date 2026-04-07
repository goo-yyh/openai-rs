//! Fine-tuning namespace implementations.

use http::Method;
use serde_json::Value;

use super::{
    DeleteResponse, FineTuningAlphaGradersResource, FineTuningAlphaResource, FineTuningCheckpoint,
    FineTuningCheckpointPermission, FineTuningCheckpointPermissionsResource, FineTuningJob,
    FineTuningJobCheckpointsResource, FineTuningJobCreateRequestBuilder, FineTuningJobEvent,
    FineTuningJobsResource, FineTuningResource, GradersResource, JsonRequestBuilder,
    ListRequestBuilder, encode_path_segment,
};

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
    pub fn run(&self) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "fine_tuning.alpha.graders.run",
            Method::POST,
            "/fine_tuning/alpha/graders/run",
        )
    }

    /// 校验 grader。
    pub fn validate(&self) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "fine_tuning.alpha.graders.validate",
            Method::POST,
            "/fine_tuning/alpha/graders/validate",
        )
    }
}

impl GradersResource {
    /// 当前资源主要导出类型，暂不提供额外 HTTP 方法。
    pub fn grader_models(&self) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "graders.grader_models",
            Method::GET,
            "/graders/grader_models",
        )
    }
}
