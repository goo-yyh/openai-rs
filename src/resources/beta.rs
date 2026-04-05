//! Beta 命名空间实现。

use std::collections::BTreeMap;
use std::time::Duration;

use http::Method;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::time::sleep;

use crate::Client;
use crate::error::{Error, Result};

#[cfg(feature = "realtime")]
use super::RealtimeSocketRequestBuilder;
use super::{
    AssistantStreamRequestBuilder, BetaAssistantsResource, BetaChatkitResource,
    BetaChatkitSessionsResource, BetaChatkitThreadsResource, BetaRealtimeResource,
    BetaRealtimeSessionsResource, BetaRealtimeTranscriptionSessionsResource, BetaResource,
    BetaThreadMessagesResource, BetaThreadRunStepsResource, BetaThreadRunsResource,
    BetaThreadsResource, DeleteResponse, JsonRequestBuilder, ListRequestBuilder,
    encode_path_segment,
};

/// 表示 beta assistant 对象。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BetaAssistant {
    /// 对象 ID。
    pub id: String,
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// 模型 ID。
    pub model: Option<String>,
    /// 名称。
    pub name: Option<String>,
    /// 描述。
    pub description: Option<String>,
    /// 指令。
    pub instructions: Option<String>,
    /// 工具集合。
    #[serde(default)]
    pub tools: Vec<Value>,
    /// 元数据。
    pub metadata: Option<Value>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 beta thread 对象。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BetaThread {
    /// 对象 ID。
    pub id: String,
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// 元数据。
    pub metadata: Option<Value>,
    /// 工具资源。
    pub tool_resources: Option<Value>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 beta thread message 对象。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BetaThreadMessage {
    /// 对象 ID。
    pub id: String,
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// thread ID。
    pub thread_id: Option<String>,
    /// 角色。
    pub role: Option<String>,
    /// 状态。
    pub status: Option<String>,
    /// 内容。
    #[serde(default)]
    pub content: Vec<Value>,
    /// assistant ID。
    pub assistant_id: Option<String>,
    /// run ID。
    pub run_id: Option<String>,
    /// 元数据。
    pub metadata: Option<Value>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 beta thread run 对象。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BetaThreadRun {
    /// 对象 ID。
    pub id: String,
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// thread ID。
    pub thread_id: Option<String>,
    /// assistant ID。
    pub assistant_id: Option<String>,
    /// 状态。
    pub status: Option<String>,
    /// 模型 ID。
    pub model: Option<String>,
    /// 指令。
    pub instructions: Option<String>,
    /// 需要用户采取的动作。
    pub required_action: Option<Value>,
    /// 最近错误。
    pub last_error: Option<Value>,
    /// 不完整细节。
    pub incomplete_details: Option<Value>,
    /// 工具集合。
    #[serde(default)]
    pub tools: Vec<Value>,
    /// 元数据。
    pub metadata: Option<Value>,
    /// 用量。
    pub usage: Option<Value>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 beta thread run step 对象。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BetaThreadRunStep {
    /// 对象 ID。
    pub id: String,
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// run ID。
    pub run_id: Option<String>,
    /// assistant ID。
    pub assistant_id: Option<String>,
    /// 状态。
    pub status: Option<String>,
    /// step 详情。
    pub step_details: Option<Value>,
    /// 用量。
    pub usage: Option<Value>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl BetaResource {
    /// 返回 assistants 子资源。
    pub fn assistants(&self) -> BetaAssistantsResource {
        BetaAssistantsResource::new(self.client.clone())
    }

    /// 返回 threads 子资源。
    pub fn threads(&self) -> BetaThreadsResource {
        BetaThreadsResource::new(self.client.clone())
    }

    /// 返回 chatkit 子资源。
    pub fn chatkit(&self) -> BetaChatkitResource {
        BetaChatkitResource::new(self.client.clone())
    }

    /// 返回 realtime 子资源。
    pub fn realtime(&self) -> BetaRealtimeResource {
        BetaRealtimeResource::new(self.client.clone())
    }
}

impl BetaAssistantsResource {
    /// 创建 assistant。
    pub fn create(&self) -> JsonRequestBuilder<BetaAssistant> {
        beta_json(
            self.client.clone(),
            "beta.assistants.create",
            Method::POST,
            "/assistants",
        )
    }

    /// 获取 assistant。
    pub fn retrieve(&self, assistant_id: impl Into<String>) -> JsonRequestBuilder<BetaAssistant> {
        beta_json(
            self.client.clone(),
            "beta.assistants.retrieve",
            Method::GET,
            format!("/assistants/{}", encode_path_segment(assistant_id.into())),
        )
    }

    /// 更新 assistant。
    pub fn update(&self, assistant_id: impl Into<String>) -> JsonRequestBuilder<BetaAssistant> {
        beta_json(
            self.client.clone(),
            "beta.assistants.update",
            Method::POST,
            format!("/assistants/{}", encode_path_segment(assistant_id.into())),
        )
    }

    /// 列出 assistants。
    pub fn list(&self) -> ListRequestBuilder<BetaAssistant> {
        beta_list(self.client.clone(), "beta.assistants.list", "/assistants")
    }

    /// 删除 assistant。
    pub fn delete(&self, assistant_id: impl Into<String>) -> JsonRequestBuilder<DeleteResponse> {
        beta_json(
            self.client.clone(),
            "beta.assistants.delete",
            Method::DELETE,
            format!("/assistants/{}", encode_path_segment(assistant_id.into())),
        )
    }
}

impl BetaThreadsResource {
    /// 创建 thread。
    pub fn create(&self) -> JsonRequestBuilder<BetaThread> {
        beta_json(
            self.client.clone(),
            "beta.threads.create",
            Method::POST,
            "/threads",
        )
    }

    /// 获取 thread。
    pub fn retrieve(&self, thread_id: impl Into<String>) -> JsonRequestBuilder<BetaThread> {
        beta_json(
            self.client.clone(),
            "beta.threads.retrieve",
            Method::GET,
            format!("/threads/{}", encode_path_segment(thread_id.into())),
        )
    }

    /// 更新 thread。
    pub fn update(&self, thread_id: impl Into<String>) -> JsonRequestBuilder<BetaThread> {
        beta_json(
            self.client.clone(),
            "beta.threads.update",
            Method::POST,
            format!("/threads/{}", encode_path_segment(thread_id.into())),
        )
    }

    /// 删除 thread。
    pub fn delete(&self, thread_id: impl Into<String>) -> JsonRequestBuilder<DeleteResponse> {
        beta_json(
            self.client.clone(),
            "beta.threads.delete",
            Method::DELETE,
            format!("/threads/{}", encode_path_segment(thread_id.into())),
        )
    }

    /// 创建并运行 thread。
    pub fn create_and_run(&self) -> JsonRequestBuilder<BetaThreadRun> {
        beta_json(
            self.client.clone(),
            "beta.threads.create_and_run",
            Method::POST,
            "/threads/runs",
        )
    }

    /// 创建并运行流式 thread。
    pub fn create_and_run_stream(&self) -> AssistantStreamRequestBuilder {
        beta_assistant_stream(
            self.client.clone(),
            "beta.threads.create_and_run_stream",
            Method::POST,
            "/threads/runs",
        )
        .extra_body("stream", Value::Bool(true))
    }

    /// 创建并运行 thread，然后轮询直到 run 进入终态。
    ///
    /// # Errors
    ///
    /// 当请求失败、响应缺少 `thread_id` 或轮询失败时返回错误。
    pub async fn create_and_run_poll<T>(
        &self,
        body: &T,
        poll_interval: Option<Duration>,
    ) -> Result<BetaThreadRun>
    where
        T: Serialize,
    {
        let run = self.create_and_run().json_body(body)?.send().await?;
        let thread_id = run
            .thread_id
            .clone()
            .ok_or_else(|| Error::InvalidConfig("run 响应缺少 thread_id，无法继续轮询".into()))?;
        self.runs()
            .poll(thread_id, run.id.clone(), poll_interval)
            .await
    }

    /// 返回 messages 子资源。
    pub fn messages(&self) -> BetaThreadMessagesResource {
        BetaThreadMessagesResource::new(self.client.clone())
    }

    /// 返回 runs 子资源。
    pub fn runs(&self) -> BetaThreadRunsResource {
        BetaThreadRunsResource::new(self.client.clone())
    }
}

impl BetaThreadMessagesResource {
    /// 创建 thread message。
    pub fn create(&self, thread_id: impl Into<String>) -> JsonRequestBuilder<BetaThreadMessage> {
        beta_json(
            self.client.clone(),
            "beta.threads.messages.create",
            Method::POST,
            format!(
                "/threads/{}/messages",
                encode_path_segment(thread_id.into())
            ),
        )
    }

    /// 获取 thread message。
    pub fn retrieve(
        &self,
        thread_id: impl Into<String>,
        message_id: impl Into<String>,
    ) -> JsonRequestBuilder<BetaThreadMessage> {
        beta_json(
            self.client.clone(),
            "beta.threads.messages.retrieve",
            Method::GET,
            format!(
                "/threads/{}/messages/{}",
                encode_path_segment(thread_id.into()),
                encode_path_segment(message_id.into())
            ),
        )
    }

    /// 更新 thread message。
    pub fn update(
        &self,
        thread_id: impl Into<String>,
        message_id: impl Into<String>,
    ) -> JsonRequestBuilder<BetaThreadMessage> {
        beta_json(
            self.client.clone(),
            "beta.threads.messages.update",
            Method::POST,
            format!(
                "/threads/{}/messages/{}",
                encode_path_segment(thread_id.into()),
                encode_path_segment(message_id.into())
            ),
        )
    }

    /// 列出 thread messages。
    pub fn list(&self, thread_id: impl Into<String>) -> ListRequestBuilder<BetaThreadMessage> {
        beta_list(
            self.client.clone(),
            "beta.threads.messages.list",
            format!(
                "/threads/{}/messages",
                encode_path_segment(thread_id.into())
            ),
        )
    }

    /// 删除 thread message。
    pub fn delete(
        &self,
        thread_id: impl Into<String>,
        message_id: impl Into<String>,
    ) -> JsonRequestBuilder<DeleteResponse> {
        beta_json(
            self.client.clone(),
            "beta.threads.messages.delete",
            Method::DELETE,
            format!(
                "/threads/{}/messages/{}",
                encode_path_segment(thread_id.into()),
                encode_path_segment(message_id.into())
            ),
        )
    }
}

impl BetaThreadRunsResource {
    /// 创建 run。
    pub fn create(&self, thread_id: impl Into<String>) -> JsonRequestBuilder<BetaThreadRun> {
        beta_json(
            self.client.clone(),
            "beta.threads.runs.create",
            Method::POST,
            format!("/threads/{}/runs", encode_path_segment(thread_id.into())),
        )
    }

    /// 获取 run。
    pub fn retrieve(
        &self,
        thread_id: impl Into<String>,
        run_id: impl Into<String>,
    ) -> JsonRequestBuilder<BetaThreadRun> {
        beta_json(
            self.client.clone(),
            "beta.threads.runs.retrieve",
            Method::GET,
            format!(
                "/threads/{}/runs/{}",
                encode_path_segment(thread_id.into()),
                encode_path_segment(run_id.into())
            ),
        )
    }

    /// 更新 run。
    pub fn update(
        &self,
        thread_id: impl Into<String>,
        run_id: impl Into<String>,
    ) -> JsonRequestBuilder<BetaThreadRun> {
        beta_json(
            self.client.clone(),
            "beta.threads.runs.update",
            Method::POST,
            format!(
                "/threads/{}/runs/{}",
                encode_path_segment(thread_id.into()),
                encode_path_segment(run_id.into())
            ),
        )
    }

    /// 列出 runs。
    pub fn list(&self, thread_id: impl Into<String>) -> ListRequestBuilder<BetaThreadRun> {
        beta_list(
            self.client.clone(),
            "beta.threads.runs.list",
            format!("/threads/{}/runs", encode_path_segment(thread_id.into())),
        )
    }

    /// 取消 run。
    pub fn cancel(
        &self,
        thread_id: impl Into<String>,
        run_id: impl Into<String>,
    ) -> JsonRequestBuilder<BetaThreadRun> {
        beta_json(
            self.client.clone(),
            "beta.threads.runs.cancel",
            Method::POST,
            format!(
                "/threads/{}/runs/{}/cancel",
                encode_path_segment(thread_id.into()),
                encode_path_segment(run_id.into())
            ),
        )
    }

    /// 创建并流式获取 run。
    pub fn create_and_stream(&self, thread_id: impl Into<String>) -> AssistantStreamRequestBuilder {
        beta_assistant_stream(
            self.client.clone(),
            "beta.threads.runs.create_and_stream",
            Method::POST,
            format!("/threads/{}/runs", encode_path_segment(thread_id.into())),
        )
        .extra_body("stream", Value::Bool(true))
    }

    /// 提交工具输出。
    pub fn submit_tool_outputs(
        &self,
        thread_id: impl Into<String>,
        run_id: impl Into<String>,
    ) -> JsonRequestBuilder<BetaThreadRun> {
        beta_json(
            self.client.clone(),
            "beta.threads.runs.submit_tool_outputs",
            Method::POST,
            format!(
                "/threads/{}/runs/{}/submit_tool_outputs",
                encode_path_segment(thread_id.into()),
                encode_path_segment(run_id.into())
            ),
        )
    }

    /// 流式提交工具输出。
    pub fn submit_tool_outputs_stream(
        &self,
        thread_id: impl Into<String>,
        run_id: impl Into<String>,
    ) -> AssistantStreamRequestBuilder {
        beta_assistant_stream(
            self.client.clone(),
            "beta.threads.runs.submit_tool_outputs_stream",
            Method::POST,
            format!(
                "/threads/{}/runs/{}/submit_tool_outputs",
                encode_path_segment(thread_id.into()),
                encode_path_segment(run_id.into())
            ),
        )
        .extra_body("stream", Value::Bool(true))
    }

    /// 流式获取 run。
    pub fn stream(
        &self,
        thread_id: impl Into<String>,
        run_id: impl Into<String>,
    ) -> AssistantStreamRequestBuilder {
        beta_assistant_stream(
            self.client.clone(),
            "beta.threads.runs.stream",
            Method::GET,
            format!(
                "/threads/{}/runs/{}",
                encode_path_segment(thread_id.into()),
                encode_path_segment(run_id.into())
            ),
        )
        .extra_query("stream", "true")
    }

    /// 创建 run，然后轮询直到进入终态。
    ///
    /// # Errors
    ///
    /// 当创建或轮询失败时返回错误。
    pub async fn create_and_poll<T>(
        &self,
        thread_id: impl Into<String>,
        body: &T,
        poll_interval: Option<Duration>,
    ) -> Result<BetaThreadRun>
    where
        T: Serialize,
    {
        let thread_id = thread_id.into();
        let run = self
            .create(thread_id.clone())
            .json_body(body)?
            .send()
            .await?;
        self.poll(thread_id, run.id.clone(), poll_interval).await
    }

    /// 轮询 run，直到状态进入终态。
    ///
    /// # Errors
    ///
    /// 当请求失败时返回错误。
    pub async fn poll(
        &self,
        thread_id: impl Into<String>,
        run_id: impl Into<String>,
        poll_interval: Option<Duration>,
    ) -> Result<BetaThreadRun> {
        let thread_id = thread_id.into();
        let run_id = run_id.into();

        loop {
            let mut request = self
                .retrieve(thread_id.clone(), run_id.clone())
                .extra_header("x-stainless-poll-helper", "true");
            if let Some(interval) = poll_interval {
                request = request.extra_header(
                    "x-stainless-custom-poll-interval",
                    interval.as_millis().to_string(),
                );
            }

            let response = request.send_with_meta().await?;
            let run = response.data;
            match run.status.as_deref().unwrap_or_default() {
                "queued" | "in_progress" | "cancelling" => {
                    let header_delay = response
                        .meta
                        .headers
                        .get("openai-poll-after-ms")
                        .and_then(|value| value.to_str().ok())
                        .and_then(|value| value.parse::<u64>().ok())
                        .map(Duration::from_millis);
                    sleep(
                        poll_interval
                            .or(header_delay)
                            .unwrap_or(Duration::from_secs(5)),
                    )
                    .await;
                }
                _ => return Ok(run),
            }
        }
    }

    /// 提交工具输出，然后轮询直到 run 进入终态。
    ///
    /// # Errors
    ///
    /// 当提交工具输出或轮询失败时返回错误。
    pub async fn submit_tool_outputs_and_poll<T>(
        &self,
        thread_id: impl Into<String>,
        run_id: impl Into<String>,
        body: &T,
        poll_interval: Option<Duration>,
    ) -> Result<BetaThreadRun>
    where
        T: Serialize,
    {
        let thread_id = thread_id.into();
        let run_id = run_id.into();
        let run = self
            .submit_tool_outputs(thread_id.clone(), run_id)
            .json_body(body)?
            .send()
            .await?;
        self.poll(thread_id, run.id.clone(), poll_interval).await
    }

    /// 返回 steps 子资源。
    pub fn steps(&self) -> BetaThreadRunStepsResource {
        BetaThreadRunStepsResource::new(self.client.clone())
    }
}

impl BetaThreadRunStepsResource {
    /// 获取 run step。
    pub fn retrieve(
        &self,
        thread_id: impl Into<String>,
        run_id: impl Into<String>,
        step_id: impl Into<String>,
    ) -> JsonRequestBuilder<BetaThreadRunStep> {
        beta_json(
            self.client.clone(),
            "beta.threads.runs.steps.retrieve",
            Method::GET,
            format!(
                "/threads/{}/runs/{}/steps/{}",
                encode_path_segment(thread_id.into()),
                encode_path_segment(run_id.into()),
                encode_path_segment(step_id.into())
            ),
        )
    }

    /// 列出 run steps。
    pub fn list(
        &self,
        thread_id: impl Into<String>,
        run_id: impl Into<String>,
    ) -> ListRequestBuilder<BetaThreadRunStep> {
        beta_list(
            self.client.clone(),
            "beta.threads.runs.steps.list",
            format!(
                "/threads/{}/runs/{}/steps",
                encode_path_segment(thread_id.into()),
                encode_path_segment(run_id.into())
            ),
        )
    }
}

impl BetaChatkitResource {
    /// 返回 sessions 子资源。
    pub fn sessions(&self) -> BetaChatkitSessionsResource {
        BetaChatkitSessionsResource::new(self.client.clone())
    }

    /// 返回 threads 子资源。
    pub fn threads(&self) -> BetaChatkitThreadsResource {
        BetaChatkitThreadsResource::new(self.client.clone())
    }
}

impl BetaChatkitSessionsResource {
    /// 创建 chatkit session。
    pub fn create(&self) -> JsonRequestBuilder<Value> {
        beta_json(
            self.client.clone(),
            "beta.chatkit.sessions.create",
            Method::POST,
            "/chatkit/sessions",
        )
    }

    /// 取消 chatkit session。
    pub fn cancel(&self, session_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        beta_json(
            self.client.clone(),
            "beta.chatkit.sessions.cancel",
            Method::POST,
            format!(
                "/chatkit/sessions/{}/cancel",
                encode_path_segment(session_id.into())
            ),
        )
    }
}

impl BetaChatkitThreadsResource {
    /// 获取 chatkit thread。
    pub fn retrieve(&self, thread_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        beta_json(
            self.client.clone(),
            "beta.chatkit.threads.retrieve",
            Method::GET,
            format!("/chatkit/threads/{}", encode_path_segment(thread_id.into())),
        )
    }

    /// 列出 chatkit threads。
    pub fn list(&self) -> ListRequestBuilder<Value> {
        beta_list(
            self.client.clone(),
            "beta.chatkit.threads.list",
            "/chatkit/threads",
        )
    }

    /// 列出 chatkit thread items。
    pub fn list_items(&self, thread_id: impl Into<String>) -> ListRequestBuilder<Value> {
        beta_list(
            self.client.clone(),
            "beta.chatkit.threads.list_items",
            format!(
                "/chatkit/threads/{}/items",
                encode_path_segment(thread_id.into())
            ),
        )
    }

    /// 删除 chatkit thread。
    pub fn delete(&self, thread_id: impl Into<String>) -> JsonRequestBuilder<DeleteResponse> {
        beta_json(
            self.client.clone(),
            "beta.chatkit.threads.delete",
            Method::DELETE,
            format!("/chatkit/threads/{}", encode_path_segment(thread_id.into())),
        )
    }
}

impl BetaRealtimeResource {
    /// 创建 Realtime WebSocket 连接构建器。
    #[cfg(feature = "realtime")]
    #[cfg_attr(docsrs, doc(cfg(feature = "realtime")))]
    pub fn ws(&self) -> RealtimeSocketRequestBuilder {
        RealtimeSocketRequestBuilder::new(self.client.clone())
    }

    /// 返回 sessions 子资源。
    pub fn sessions(&self) -> BetaRealtimeSessionsResource {
        BetaRealtimeSessionsResource::new(self.client.clone())
    }

    /// 返回 transcription_sessions 子资源。
    pub fn transcription_sessions(&self) -> BetaRealtimeTranscriptionSessionsResource {
        BetaRealtimeTranscriptionSessionsResource::new(self.client.clone())
    }
}

impl BetaRealtimeSessionsResource {
    /// 创建 beta realtime session。
    pub fn create(&self) -> JsonRequestBuilder<Value> {
        beta_json(
            self.client.clone(),
            "beta.realtime.sessions.create",
            Method::POST,
            "/realtime/sessions",
        )
    }
}

impl BetaRealtimeTranscriptionSessionsResource {
    /// 创建 beta realtime transcription session。
    pub fn create(&self) -> JsonRequestBuilder<Value> {
        beta_json(
            self.client.clone(),
            "beta.realtime.transcription_sessions.create",
            Method::POST,
            "/realtime/transcription_sessions",
        )
    }
}

fn beta_json<T>(
    client: Client,
    endpoint_id: &'static str,
    method: Method,
    path: impl Into<String>,
) -> JsonRequestBuilder<T> {
    JsonRequestBuilder::new(client, endpoint_id, method, path)
        .extra_header("openai-beta", "assistants=v2")
}

fn beta_list<T>(
    client: Client,
    endpoint_id: &'static str,
    path: impl Into<String>,
) -> ListRequestBuilder<T> {
    ListRequestBuilder::new(client, endpoint_id, path).extra_header("openai-beta", "assistants=v2")
}

fn beta_assistant_stream(
    client: Client,
    endpoint_id: &'static str,
    method: Method,
    path: impl Into<String>,
) -> AssistantStreamRequestBuilder {
    AssistantStreamRequestBuilder::new(client, endpoint_id, method, path)
        .extra_header("openai-beta", "assistants=v2")
}
