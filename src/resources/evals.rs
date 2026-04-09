//! Eval namespace implementations.

use http::Method;

use crate::generated::endpoints;

use super::{
    DeleteResponse, Eval, EvalOutputItem, EvalRun, EvalRunOutputItemsResource, EvalRunsResource,
    EvalsResource, JsonRequestBuilder, ListRequestBuilder, encode_path_segment,
};

impl EvalsResource {
    /// 创建 eval。
    pub fn create(&self) -> JsonRequestBuilder<Eval> {
        let endpoint = endpoints::evals::EVALS_CREATE;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::POST,
            endpoint.template,
        )
    }

    /// 获取 eval。
    pub fn retrieve(&self, eval_id: impl Into<String>) -> JsonRequestBuilder<Eval> {
        let eval_id = encode_path_segment(eval_id.into());
        let endpoint = endpoints::evals::EVALS_RETRIEVE;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::GET,
            endpoint.render(&[("eval_id", &eval_id)]),
        )
    }

    /// 更新 eval。
    pub fn update(&self, eval_id: impl Into<String>) -> JsonRequestBuilder<Eval> {
        let eval_id = encode_path_segment(eval_id.into());
        let endpoint = endpoints::evals::EVALS_UPDATE;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::POST,
            endpoint.render(&[("eval_id", &eval_id)]),
        )
    }

    /// 列出 evals。
    pub fn list(&self) -> ListRequestBuilder<Eval> {
        let endpoint = endpoints::evals::EVALS_LIST;
        ListRequestBuilder::new(self.client.clone(), endpoint.id, endpoint.template)
    }

    /// 删除 eval。
    pub fn delete(&self, eval_id: impl Into<String>) -> JsonRequestBuilder<DeleteResponse> {
        let eval_id = encode_path_segment(eval_id.into());
        let endpoint = endpoints::evals::EVALS_DELETE;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::DELETE,
            endpoint.render(&[("eval_id", &eval_id)]),
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
        let eval_id = encode_path_segment(eval_id.into());
        let endpoint = endpoints::evals::EVALS_RUNS_CREATE;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::POST,
            endpoint.render(&[("eval_id", &eval_id)]),
        )
    }

    /// 获取 eval run。
    pub fn retrieve(
        &self,
        eval_id: impl Into<String>,
        run_id: impl Into<String>,
    ) -> JsonRequestBuilder<EvalRun> {
        let eval_id = encode_path_segment(eval_id.into());
        let run_id = encode_path_segment(run_id.into());
        let endpoint = endpoints::evals::EVALS_RUNS_RETRIEVE;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::GET,
            endpoint.render(&[("eval_id", &eval_id), ("run_id", &run_id)]),
        )
    }

    /// 列出 eval runs。
    pub fn list(&self, eval_id: impl Into<String>) -> ListRequestBuilder<EvalRun> {
        let eval_id = encode_path_segment(eval_id.into());
        let endpoint = endpoints::evals::EVALS_RUNS_LIST;
        ListRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            endpoint.render(&[("eval_id", &eval_id)]),
        )
    }

    /// 删除 eval run。
    pub fn delete(
        &self,
        eval_id: impl Into<String>,
        run_id: impl Into<String>,
    ) -> JsonRequestBuilder<DeleteResponse> {
        let eval_id = encode_path_segment(eval_id.into());
        let run_id = encode_path_segment(run_id.into());
        let endpoint = endpoints::evals::EVALS_RUNS_DELETE;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::DELETE,
            endpoint.render(&[("eval_id", &eval_id), ("run_id", &run_id)]),
        )
    }

    /// 取消 eval run。
    pub fn cancel(
        &self,
        eval_id: impl Into<String>,
        run_id: impl Into<String>,
    ) -> JsonRequestBuilder<EvalRun> {
        let eval_id = encode_path_segment(eval_id.into());
        let run_id = encode_path_segment(run_id.into());
        let endpoint = endpoints::evals::EVALS_RUNS_CANCEL;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::POST,
            endpoint.render(&[("eval_id", &eval_id), ("run_id", &run_id)]),
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
        let eval_id = encode_path_segment(eval_id.into());
        let run_id = encode_path_segment(run_id.into());
        let item_id = encode_path_segment(item_id.into());
        let endpoint = endpoints::evals::EVALS_RUNS_OUTPUT_ITEMS_RETRIEVE;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::GET,
            endpoint.render(&[
                ("eval_id", &eval_id),
                ("run_id", &run_id),
                ("item_id", &item_id),
            ]),
        )
    }

    /// 列出 output items。
    pub fn list(
        &self,
        eval_id: impl Into<String>,
        run_id: impl Into<String>,
    ) -> ListRequestBuilder<EvalOutputItem> {
        let eval_id = encode_path_segment(eval_id.into());
        let run_id = encode_path_segment(run_id.into());
        let endpoint = endpoints::evals::EVALS_RUNS_OUTPUT_ITEMS_LIST;
        ListRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            endpoint.render(&[("eval_id", &eval_id), ("run_id", &run_id)]),
        )
    }
}
