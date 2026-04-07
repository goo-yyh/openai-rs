//! Eval namespace implementations.

use http::Method;

use super::{
    DeleteResponse, Eval, EvalOutputItem, EvalRun, EvalRunOutputItemsResource, EvalRunsResource,
    EvalsResource, JsonRequestBuilder, ListRequestBuilder, encode_path_segment,
};

impl EvalsResource {
    /// 创建 eval。
    pub fn create(&self) -> JsonRequestBuilder<Eval> {
        JsonRequestBuilder::new(self.client.clone(), "evals.create", Method::POST, "/evals")
    }

    /// 获取 eval。
    pub fn retrieve(&self, eval_id: impl Into<String>) -> JsonRequestBuilder<Eval> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "evals.retrieve",
            Method::GET,
            format!("/evals/{}", encode_path_segment(eval_id.into())),
        )
    }

    /// 更新 eval。
    pub fn update(&self, eval_id: impl Into<String>) -> JsonRequestBuilder<Eval> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "evals.update",
            Method::POST,
            format!("/evals/{}", encode_path_segment(eval_id.into())),
        )
    }

    /// 列出 evals。
    pub fn list(&self) -> ListRequestBuilder<Eval> {
        ListRequestBuilder::new(self.client.clone(), "evals.list", "/evals")
    }

    /// 删除 eval。
    pub fn delete(&self, eval_id: impl Into<String>) -> JsonRequestBuilder<DeleteResponse> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "evals.delete",
            Method::DELETE,
            format!("/evals/{}", encode_path_segment(eval_id.into())),
        )
    }

    /// 返回 runs 子资源。
    pub fn runs(&self) -> EvalRunsResource {
        EvalRunsResource::new(self.client.clone())
    }
}

impl EvalRunsResource {
    /// 创建 eval run。
    pub fn create(&self, eval_id: impl Into<String>) -> JsonRequestBuilder<EvalRun> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "evals.runs.create",
            Method::POST,
            format!("/evals/{}/runs", encode_path_segment(eval_id.into())),
        )
    }

    /// 获取 eval run。
    pub fn retrieve(
        &self,
        eval_id: impl Into<String>,
        run_id: impl Into<String>,
    ) -> JsonRequestBuilder<EvalRun> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "evals.runs.retrieve",
            Method::GET,
            format!(
                "/evals/{}/runs/{}",
                encode_path_segment(eval_id.into()),
                encode_path_segment(run_id.into())
            ),
        )
    }

    /// 列出 eval runs。
    pub fn list(&self, eval_id: impl Into<String>) -> ListRequestBuilder<EvalRun> {
        ListRequestBuilder::new(
            self.client.clone(),
            "evals.runs.list",
            format!("/evals/{}/runs", encode_path_segment(eval_id.into())),
        )
    }

    /// 删除 eval run。
    pub fn delete(
        &self,
        eval_id: impl Into<String>,
        run_id: impl Into<String>,
    ) -> JsonRequestBuilder<DeleteResponse> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "evals.runs.delete",
            Method::DELETE,
            format!(
                "/evals/{}/runs/{}",
                encode_path_segment(eval_id.into()),
                encode_path_segment(run_id.into())
            ),
        )
    }

    /// 取消 eval run。
    pub fn cancel(
        &self,
        eval_id: impl Into<String>,
        run_id: impl Into<String>,
    ) -> JsonRequestBuilder<EvalRun> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "evals.runs.cancel",
            Method::POST,
            format!(
                "/evals/{}/runs/{}/cancel",
                encode_path_segment(eval_id.into()),
                encode_path_segment(run_id.into())
            ),
        )
    }

    /// 返回 output_items 子资源。
    pub fn output_items(&self) -> EvalRunOutputItemsResource {
        EvalRunOutputItemsResource::new(self.client.clone())
    }
}

impl EvalRunOutputItemsResource {
    /// 获取 output item。
    pub fn retrieve(
        &self,
        eval_id: impl Into<String>,
        run_id: impl Into<String>,
        item_id: impl Into<String>,
    ) -> JsonRequestBuilder<EvalOutputItem> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "evals.runs.output_items.retrieve",
            Method::GET,
            format!(
                "/evals/{}/runs/{}/output_items/{}",
                encode_path_segment(eval_id.into()),
                encode_path_segment(run_id.into()),
                encode_path_segment(item_id.into())
            ),
        )
    }

    /// 列出 output items。
    pub fn list(
        &self,
        eval_id: impl Into<String>,
        run_id: impl Into<String>,
    ) -> ListRequestBuilder<EvalOutputItem> {
        ListRequestBuilder::new(
            self.client.clone(),
            "evals.runs.output_items.list",
            format!(
                "/evals/{}/runs/{}/output_items",
                encode_path_segment(eval_id.into()),
                encode_path_segment(run_id.into())
            ),
        )
    }
}
