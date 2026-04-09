use std::time::Duration;

use bytes::Bytes;
use http::Method;
use tokio_util::sync::CancellationToken;

use super::*;
use crate::files::UploadSource;
use crate::generated::endpoints;
use crate::json_payload::JsonPayload;
use crate::response_meta::ApiResponse;
use crate::stream::{RawSseStream, SseStream};
use crate::transport::RequestSpec;

macro_rules! json_payload_wrapper {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Value);

        impl Default for $name {
            fn default() -> Self {
                Self(Value::Null)
            }
        }

        impl From<Value> for $name {
            fn from(value: Value) -> Self {
                Self(value)
            }
        }

        impl From<$name> for Value {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl $name {
            /// 返回未经解释的原始 JSON 值。
            pub fn as_raw(&self) -> &Value {
                &self.0
            }

            /// 消费该包装器并返回原始 JSON 值。
            pub fn into_raw(self) -> Value {
                self.0
            }

            /// 返回载荷中的 `type` 字段，若存在且为字符串。
            pub fn kind(&self) -> Option<&str> {
                self.0.get("type").and_then(Value::as_str)
            }
        }
    };
}

json_payload_wrapper!(
    /// 表示 conversation item 的内容片段。
    ConversationContentPart
);
json_payload_wrapper!(
    /// 表示 conversation 创建时的初始条目。
    ConversationInputItem
);
json_payload_wrapper!(
    /// 表示 eval 数据源配置。
    EvalDataSourceConfig
);
json_payload_wrapper!(
    /// 表示 eval 测试标准项。
    EvalTestingCriterion
);
json_payload_wrapper!(
    /// 表示 eval run 的输入载荷。
    EvalRunInput
);
json_payload_wrapper!(
    /// 表示 eval 输出项载荷。
    EvalOutput
);
json_payload_wrapper!(
    /// 表示 skill version 内容载荷。
    SkillVersionContent
);

/// 表示音频转写 segment ID。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AudioTranscriptionSegmentId {
    /// 数值 ID。
    Number(u64),
    /// 字符串 ID。
    String(String),
}

/// 表示音频转写分段。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AudioTranscriptionSegment {
    /// 分段 ID。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<AudioTranscriptionSegmentId>,
    /// 平均 logprob。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_logprob: Option<f64>,
    /// 压缩比。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compression_ratio: Option<f64>,
    /// 结束时间。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<f64>,
    /// 静音概率。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_speech_prob: Option<f64>,
    /// seek 偏移。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seek: Option<u64>,
    /// 说话人。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speaker: Option<String>,
    /// 开始时间。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<f64>,
    /// 采样温度。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// 文本内容。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// token ID 列表。
    #[serde(default)]
    pub tokens: Vec<u64>,
    /// 分段类型。
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segment_type: Option<String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示音频转写词级时间戳。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AudioTranscriptionWord {
    /// 词文本。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub word: Option<String>,
    /// 开始时间。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<f64>,
    /// 结束时间。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<f64>,
    /// 概率。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub probability: Option<f64>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 fine-tuning 超参数值。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FineTuningHyperparameterValue {
    /// 字符串配置，通常为 `auto`。
    Text(String),
    /// 整型配置。
    Integer(u64),
    /// 浮点配置。
    Float(f64),
}

/// 表示 fine-tuning 超参数。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FineTuningJobHyperparameters {
    /// batch size。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_size: Option<FineTuningHyperparameterValue>,
    /// learning rate multiplier。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub learning_rate_multiplier: Option<FineTuningHyperparameterValue>,
    /// epoch 数。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n_epochs: Option<FineTuningHyperparameterValue>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 fine-tuning 错误。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FineTuningJobError {
    /// 错误码。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// 错误消息。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// 关联参数。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub param: Option<String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 fine-tuning 指标。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FineTuningMetrics {
    /// 完整验证集 loss。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_valid_loss: Option<f64>,
    /// 完整验证集平均 token 准确率。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_valid_mean_token_accuracy: Option<f64>,
    /// 当前 step。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<u64>,
    /// 训练 loss。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub train_loss: Option<f64>,
    /// 训练平均 token 准确率。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub train_mean_token_accuracy: Option<f64>,
    /// 验证 loss。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_loss: Option<f64>,
    /// 验证平均 token 准确率。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_mean_token_accuracy: Option<f64>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 Weights & Biases 集成配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FineTuningWandbIntegration {
    /// 项目名。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    /// 实体名。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity: Option<String>,
    /// 展示名称。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 标签集合。
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 fine-tuning 集成项。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FineTuningJobIntegration {
    /// 集成类型。
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integration_type: Option<String>,
    /// Weights & Biases 配置。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wandb: Option<FineTuningWandbIntegration>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 container 过期策略。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContainerExpiresAfter {
    /// 锚点。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<String>,
    /// 过期分钟数。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minutes: Option<u64>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示单个图像输出。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImageData {
    /// 远程图片 URL。
    pub url: Option<String>,
    /// Base64 编码的图片内容。
    pub b64_json: Option<String>,
    /// 模型重写后的 prompt。
    pub revised_prompt: Option<String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示图像生成响应。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImageGenerationResponse {
    /// 创建时间。
    pub created: Option<u64>,
    /// 图像结果列表。
    #[serde(default)]
    pub data: Vec<ImageData>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示图像生成请求参数。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImageGenerateParams {
    /// 模型 ID。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// 生成提示词。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// 结果数量。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u32>,
    /// 图像尺寸。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    /// 图像质量。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<String>,
    /// 响应格式。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<String>,
    /// 背景模式。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    /// 输出格式。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,
    /// 审核策略。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moderation: Option<String>,
    /// 是否启用流式图片事件。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// 局部图片数量。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partial_images: Option<u32>,
    /// 用户标识。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    /// 自定义 metadata。
    #[serde(default, skip_serializing_if = "metadata_is_empty")]
    pub metadata: BTreeMap<String, String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示语音合成请求参数。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AudioSpeechCreateParams {
    /// 模型 ID。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// 声音名称。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<String>,
    /// 输入文本。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,
    /// 可选指令。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    /// 输出音频格式。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// 语速。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f32>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示音频转写响应。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AudioTranscription {
    /// 转写文本。
    #[serde(default)]
    pub text: String,
    /// 语言代码。
    pub language: Option<String>,
    /// 音频时长。
    pub duration: Option<f64>,
    /// 分段结果。
    #[serde(default)]
    pub segments: Vec<AudioTranscriptionSegment>,
    /// 词级结果。
    #[serde(default)]
    pub words: Vec<AudioTranscriptionWord>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示音频翻译响应。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AudioTranslation {
    /// 翻译文本。
    #[serde(default)]
    pub text: String,
    /// 语言代码。
    pub language: Option<String>,
    /// 音频时长。
    pub duration: Option<f64>,
    /// 分段结果。
    #[serde(default)]
    pub segments: Vec<AudioTranscriptionSegment>,
    /// 词级结果。
    #[serde(default)]
    pub words: Vec<AudioTranscriptionWord>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 fine-tuning job。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FineTuningJob {
    /// Job ID。
    pub id: String,
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// 基础模型。
    pub model: Option<String>,
    /// 产出的微调模型。
    pub fine_tuned_model: Option<String>,
    /// 当前状态。
    pub status: Option<String>,
    /// 训练文件 ID。
    pub training_file: Option<String>,
    /// 验证文件 ID。
    pub validation_file: Option<String>,
    /// 创建时间。
    pub created_at: Option<u64>,
    /// 完成时间。
    pub finished_at: Option<u64>,
    /// 已训练 token 数。
    pub trained_tokens: Option<u64>,
    /// 自定义 metadata。
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
    /// 超参数配置。
    pub hyperparameters: Option<FineTuningJobHyperparameters>,
    /// 结果文件。
    #[serde(default)]
    pub result_files: Vec<String>,
    /// 错误信息。
    pub error: Option<FineTuningJobError>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 fine-tuning job 事件。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FineTuningJobEvent {
    /// 事件 ID。
    pub id: Option<String>,
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// 事件类型。
    #[serde(rename = "type")]
    pub event_type: Option<String>,
    /// 日志级别。
    pub level: Option<String>,
    /// 事件消息。
    pub message: Option<String>,
    /// 创建时间。
    pub created_at: Option<u64>,
    /// 额外数据。
    pub data: Option<FineTuningMetrics>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 fine-tuning checkpoint。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FineTuningCheckpoint {
    /// Checkpoint ID。
    pub id: String,
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// 所属 job ID。
    pub fine_tuning_job_id: Option<String>,
    /// Checkpoint 模型 ID。
    pub fine_tuned_model_checkpoint: Option<String>,
    /// 步数。
    pub step_number: Option<u64>,
    /// 创建时间。
    pub created_at: Option<u64>,
    /// 指标。
    pub metrics: Option<FineTuningMetrics>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 fine-tuning checkpoint permission。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FineTuningCheckpointPermission {
    /// Permission ID。
    pub id: String,
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// 项目 ID。
    pub project_id: Option<String>,
    /// Checkpoint ID。
    pub fine_tuning_checkpoint_id: Option<String>,
    /// 创建时间。
    pub created_at: Option<u64>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 fine-tuning job 创建参数。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FineTuningJobCreateParams {
    /// 模型 ID。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// 训练文件 ID。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub training_file: Option<String>,
    /// 验证文件 ID。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_file: Option<String>,
    /// 后缀。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
    /// 随机种子。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    /// 超参数。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hyperparameters: Option<FineTuningJobHyperparameters>,
    /// 集成配置。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub integrations: Vec<FineTuningJobIntegration>,
    /// 自定义 metadata。
    #[serde(default, skip_serializing_if = "metadata_is_empty")]
    pub metadata: BTreeMap<String, String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 batch 对象。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Batch {
    /// Batch ID。
    pub id: String,
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// 接口路径。
    pub endpoint: Option<String>,
    /// 当前状态。
    pub status: Option<String>,
    /// 输入文件 ID。
    pub input_file_id: Option<String>,
    /// 输出文件 ID。
    pub output_file_id: Option<String>,
    /// 错误文件 ID。
    pub error_file_id: Option<String>,
    /// 完成窗口。
    pub completion_window: Option<String>,
    /// 创建时间。
    pub created_at: Option<u64>,
    /// 取消时间。
    pub cancelled_at: Option<u64>,
    /// 开始取消时间。
    pub cancelling_at: Option<u64>,
    /// 完成时间。
    pub completed_at: Option<u64>,
    /// 过期时间。
    pub expired_at: Option<u64>,
    /// 预计过期时间。
    pub expires_at: Option<u64>,
    /// 失败时间。
    pub failed_at: Option<u64>,
    /// 开始最终整理时间。
    pub finalizing_at: Option<u64>,
    /// 开始执行时间。
    pub in_progress_at: Option<u64>,
    /// 自定义 metadata。
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
    /// 处理该 batch 的模型。
    pub model: Option<String>,
    /// 请求统计。
    pub request_counts: Option<BatchRequestCounts>,
    /// 错误摘要。
    pub errors: Option<BatchErrors>,
    /// token 用量统计。
    pub usage: Option<BatchUsage>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 batch 的单条错误。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct BatchError {
    /// 错误码。
    pub code: Option<String>,
    /// 输入文件中的行号。
    pub line: Option<u64>,
    /// 错误消息。
    pub message: Option<String>,
    /// 相关参数名。
    pub param: Option<String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 batch 的错误摘要列表。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct BatchErrors {
    /// 错误列表。
    #[serde(default)]
    pub data: Vec<BatchError>,
    /// 对象类型。
    pub object: Option<String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 batch 内各状态请求数。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct BatchRequestCounts {
    /// 已完成请求数。
    #[serde(default)]
    pub completed: u64,
    /// 失败请求数。
    #[serde(default)]
    pub failed: u64,
    /// 总请求数。
    #[serde(default)]
    pub total: u64,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 batch 输入 token 明细。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct BatchUsageInputTokensDetails {
    /// 缓存命中的 token 数。
    pub cached_tokens: Option<u64>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 batch 输出 token 明细。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct BatchUsageOutputTokensDetails {
    /// reasoning token 数。
    pub reasoning_tokens: Option<u64>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 batch token 用量。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct BatchUsage {
    /// 输入 token 数。
    #[serde(default)]
    pub input_tokens: u64,
    /// 输入 token 明细。
    pub input_tokens_details: Option<BatchUsageInputTokensDetails>,
    /// 输出 token 数。
    #[serde(default)]
    pub output_tokens: u64,
    /// 输出 token 明细。
    pub output_tokens_details: Option<BatchUsageOutputTokensDetails>,
    /// 总 token 数。
    #[serde(default)]
    pub total_tokens: u64,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 batch 创建参数。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BatchCreateParams {
    /// 输入文件 ID。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_file_id: Option<String>,
    /// 目标接口路径。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    /// 完成窗口。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_window: Option<String>,
    /// 自定义 metadata。
    #[serde(default, skip_serializing_if = "metadata_is_empty")]
    pub metadata: BTreeMap<String, String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 conversation 对象。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Conversation {
    /// Conversation ID。
    pub id: String,
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// 名称。
    pub name: Option<String>,
    /// 创建时间。
    pub created_at: Option<u64>,
    /// 自定义 metadata。
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 conversation item 对象。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConversationItem {
    /// Item ID。
    pub id: String,
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// Item 类型。
    #[serde(rename = "type")]
    pub item_type: Option<String>,
    /// 当前状态。
    pub status: Option<String>,
    /// 角色。
    pub role: Option<String>,
    /// 内容列表。
    #[serde(default)]
    pub content: Vec<ConversationContentPart>,
    /// 自定义 metadata。
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 conversation 创建参数。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConversationCreateParams {
    /// 名称。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 初始条目。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<ConversationInputItem>,
    /// 自定义 metadata。
    #[serde(default, skip_serializing_if = "metadata_is_empty")]
    pub metadata: BTreeMap<String, String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 conversation 更新参数。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConversationUpdateParams {
    /// 名称。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 自定义 metadata。
    #[serde(default, skip_serializing_if = "metadata_is_empty")]
    pub metadata: BTreeMap<String, String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 conversation item 创建参数。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConversationItemCreateParams {
    /// Item 类型。
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub item_type: Option<String>,
    /// 角色。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// 内容列表。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content: Vec<ConversationContentPart>,
    /// 自定义 metadata。
    #[serde(default, skip_serializing_if = "metadata_is_empty")]
    pub metadata: BTreeMap<String, String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 eval 对象。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Eval {
    /// Eval ID。
    pub id: String,
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// 名称。
    pub name: Option<String>,
    /// 当前状态。
    pub status: Option<String>,
    /// 创建时间。
    pub created_at: Option<u64>,
    /// 自定义 metadata。
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 eval run 对象。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EvalRun {
    /// Run ID。
    pub id: String,
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// Eval ID。
    pub eval_id: Option<String>,
    /// 当前状态。
    pub status: Option<String>,
    /// 创建时间。
    pub created_at: Option<u64>,
    /// 自定义 metadata。
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 eval run output item。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EvalOutputItem {
    /// Item ID。
    pub id: String,
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// 当前状态。
    pub status: Option<String>,
    /// 输出内容。
    pub output: Option<EvalOutput>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 eval 创建参数。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EvalCreateParams {
    /// 名称。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 数据源。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_source: Option<EvalDataSourceConfig>,
    /// 测试标准。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub testing_criteria: Vec<EvalTestingCriterion>,
    /// 自定义 metadata。
    #[serde(default, skip_serializing_if = "metadata_is_empty")]
    pub metadata: BTreeMap<String, String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 eval 更新参数。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EvalUpdateParams {
    /// 名称。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 数据源。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_source: Option<EvalDataSourceConfig>,
    /// 测试标准。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub testing_criteria: Vec<EvalTestingCriterion>,
    /// 自定义 metadata。
    #[serde(default, skip_serializing_if = "metadata_is_empty")]
    pub metadata: BTreeMap<String, String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 eval run 创建参数。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EvalRunCreateParams {
    /// 输入数据。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<EvalRunInput>,
    /// 数据源。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_source: Option<EvalDataSourceConfig>,
    /// 自定义 metadata。
    #[serde(default, skip_serializing_if = "metadata_is_empty")]
    pub metadata: BTreeMap<String, String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 container 对象。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Container {
    /// Container ID。
    pub id: String,
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// 名称。
    pub name: Option<String>,
    /// 当前状态。
    pub status: Option<String>,
    /// 创建时间。
    pub created_at: Option<u64>,
    /// 自定义 metadata。
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 container file 对象。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContainerFile {
    /// Container file ID。
    pub id: String,
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// Container ID。
    pub container_id: Option<String>,
    /// 底层文件 ID。
    pub file_id: Option<String>,
    /// 文件名。
    pub filename: Option<String>,
    /// 当前状态。
    pub status: Option<String>,
    /// 自定义 metadata。
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 container 创建参数。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContainerCreateParams {
    /// 名称。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 过期策略。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_after: Option<ContainerExpiresAfter>,
    /// 自定义 metadata。
    #[serde(default, skip_serializing_if = "metadata_is_empty")]
    pub metadata: BTreeMap<String, String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 container file 创建参数。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContainerFileCreateParams {
    /// 关联文件 ID。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    /// 目标路径。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// 自定义 metadata。
    #[serde(default, skip_serializing_if = "metadata_is_empty")]
    pub metadata: BTreeMap<String, String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 skill 对象。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Skill {
    /// Skill ID。
    pub id: String,
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// 名称。
    pub name: Option<String>,
    /// 描述。
    pub description: Option<String>,
    /// 绑定模型。
    pub model: Option<String>,
    /// 当前状态。
    pub status: Option<String>,
    /// 自定义 metadata。
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 skill version 对象。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillVersion {
    /// Version ID。
    pub id: String,
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// Skill ID。
    pub skill_id: Option<String>,
    /// 当前状态。
    pub status: Option<String>,
    /// 自定义 metadata。
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 skill 创建参数。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillCreateParams {
    /// 名称。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 描述。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 绑定模型。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// 指令。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    /// 自定义 metadata。
    #[serde(default, skip_serializing_if = "metadata_is_empty")]
    pub metadata: BTreeMap<String, String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 skill 更新参数。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillUpdateParams {
    /// 名称。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 描述。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 绑定模型。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// 指令。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    /// 自定义 metadata。
    #[serde(default, skip_serializing_if = "metadata_is_empty")]
    pub metadata: BTreeMap<String, String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 skill version 创建参数。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillVersionCreateParams {
    /// 描述。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 版本内容。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<SkillVersionContent>,
    /// 自定义 metadata。
    #[serde(default, skip_serializing_if = "metadata_is_empty")]
    pub metadata: BTreeMap<String, String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 video 对象。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Video {
    /// Video ID。
    pub id: String,
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// 模型 ID。
    pub model: Option<String>,
    /// Prompt。
    pub prompt: Option<String>,
    /// 当前状态。
    pub status: Option<String>,
    /// 创建时间。
    pub created_at: Option<u64>,
    /// 自定义 metadata。
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 video character 对象。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VideoCharacter {
    /// Character ID。
    pub id: String,
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// 名称。
    pub name: Option<String>,
    /// 当前状态。
    pub status: Option<String>,
    /// 自定义 metadata。
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 video 创建参数。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VideoCreateParams {
    /// 模型 ID。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Prompt。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// 参考图片。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    /// 目标尺寸。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    /// 时长。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,
    /// 自定义 metadata。
    #[serde(default, skip_serializing_if = "metadata_is_empty")]
    pub metadata: BTreeMap<String, String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 video character 创建参数。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VideoCharacterCreateParams {
    /// 名称。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 角色图片。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    /// 自定义 metadata。
    #[serde(default, skip_serializing_if = "metadata_is_empty")]
    pub metadata: BTreeMap<String, String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示图像生成请求构建器。
#[derive(Debug, Clone)]
pub struct ImageGenerateRequestBuilder {
    state: TypedJsonRequestState<ImageGenerateParams>,
}

impl ImageGenerateRequestBuilder {
    pub(crate) fn new(client: Client) -> Self {
        Self {
            state: TypedJsonRequestState::new(client, ImageGenerateParams::default()),
        }
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.state.params.model = Some(model.into());
        self
    }

    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.state.params.prompt = Some(prompt.into());
        self
    }

    pub fn n(mut self, n: u32) -> Self {
        self.state.params.n = Some(n);
        self
    }

    pub fn size(mut self, size: impl Into<String>) -> Self {
        self.state.params.size = Some(size.into());
        self
    }

    pub fn quality(mut self, quality: impl Into<String>) -> Self {
        self.state.params.quality = Some(quality.into());
        self
    }

    pub fn response_format(mut self, response_format: impl Into<String>) -> Self {
        self.state.params.response_format = Some(response_format.into());
        self
    }

    pub fn background(mut self, background: impl Into<String>) -> Self {
        self.state.params.background = Some(background.into());
        self
    }

    pub fn output_format(mut self, output_format: impl Into<String>) -> Self {
        self.state.params.output_format = Some(output_format.into());
        self
    }

    pub fn moderation(mut self, moderation: impl Into<String>) -> Self {
        self.state.params.moderation = Some(moderation.into());
        self
    }

    pub fn partial_images(mut self, partial_images: u32) -> Self {
        self.state.params.partial_images = Some(partial_images);
        self
    }

    pub fn stream(mut self, stream: bool) -> Self {
        self.state.params.stream = Some(stream);
        self
    }

    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.state.params.user = Some(user.into());
        self
    }

    pub fn metadata(mut self, metadata: BTreeMap<String, String>) -> Self {
        self.state.params.metadata = metadata;
        self
    }

    pub fn params(mut self, params: ImageGenerateParams) -> Self {
        self.state.params = params;
        self
    }

    pub fn body_value(mut self, body: impl Into<JsonPayload>) -> Self {
        self.state = self.state.body_value(body);
        self
    }

    pub fn json_body<U>(mut self, body: &U) -> Result<Self>
    where
        U: Serialize,
    {
        self.state = self.state.body_value(value_from(body)?);
        Ok(self)
    }

    pub fn extra_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.state = self.state.extra_header(key, value);
        self
    }

    pub fn extra_query(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.state = self.state.extra_query(key, value);
        self
    }

    pub fn extra_body(mut self, key: impl Into<String>, value: impl Into<JsonPayload>) -> Self {
        self.state = self.state.extra_body(key, value);
        self
    }

    pub fn provider_option(
        mut self,
        key: impl Into<String>,
        value: impl Into<JsonPayload>,
    ) -> Self {
        self.state = self.state.provider_option(key, value);
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.state = self.state.timeout(timeout);
        self
    }

    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.state = self.state.max_retries(max_retries);
        self
    }

    pub fn cancellation_token(mut self, token: CancellationToken) -> Self {
        self.state = self.state.cancellation_token(token);
        self
    }

    fn build_spec(self) -> Result<(Client, RequestSpec)> {
        if self.state.body_override.is_none() {
            if self
                .state
                .params
                .model
                .as_deref()
                .unwrap_or_default()
                .is_empty()
            {
                return Err(Error::MissingRequiredField { field: "model" });
            }
            if self
                .state
                .params
                .prompt
                .as_deref()
                .unwrap_or_default()
                .is_empty()
            {
                return Err(Error::MissingRequiredField { field: "prompt" });
            }
        }
        self.state
            .build_spec("images.generate", "/images/generations")
    }

    pub async fn send(self) -> Result<ImageGenerationResponse> {
        Ok(self.send_with_meta().await?.data)
    }

    pub async fn send_with_meta(self) -> Result<ApiResponse<ImageGenerationResponse>> {
        let (client, spec) = self.build_spec()?;
        client.execute_json(spec).await
    }

    pub async fn send_raw(self) -> Result<http::Response<Bytes>> {
        let (client, spec) = self.build_spec()?;
        client.execute_raw_http(spec).await
    }

    pub async fn send_sse(mut self) -> Result<SseStream<Value>> {
        self.state.params.stream = Some(true);
        let (client, mut spec) = self.build_spec()?;
        spec.options.insert_header("accept", "text/event-stream");
        client.execute_sse(spec).await
    }

    pub async fn send_raw_sse(mut self) -> Result<RawSseStream> {
        self.state.params.stream = Some(true);
        let (client, mut spec) = self.build_spec()?;
        spec.options.insert_header("accept", "text/event-stream");
        client.execute_raw_sse(spec).await
    }
}

/// 表示语音合成请求构建器。
#[derive(Debug, Clone)]
pub struct AudioSpeechRequestBuilder {
    inner: BytesRequestBuilder,
}

impl AudioSpeechRequestBuilder {
    pub(crate) fn new(client: Client) -> Self {
        Self {
            inner: BytesRequestBuilder::new(
                client,
                "audio.speech.create",
                Method::POST,
                "/audio/speech",
            ),
        }
    }

    pub(crate) fn stream(client: Client) -> Self {
        Self::new(client).extra_body("stream_format", Value::String("sse".into()))
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.inner = self.inner.extra_body("model", Value::String(model.into()));
        self
    }

    pub fn voice(mut self, voice: impl Into<String>) -> Self {
        self.inner = self.inner.extra_body("voice", Value::String(voice.into()));
        self
    }

    pub fn input(mut self, input: impl Into<String>) -> Self {
        self.inner = self.inner.extra_body("input", Value::String(input.into()));
        self
    }

    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        self.inner = self
            .inner
            .extra_body("instructions", Value::String(instructions.into()));
        self
    }

    pub fn audio_format(mut self, format: impl Into<String>) -> Self {
        self.inner = self
            .inner
            .extra_body("format", Value::String(format.into()));
        self
    }

    pub fn speed(mut self, speed: f32) -> Self {
        self.inner = self.inner.extra_body("speed", Value::from(speed));
        self
    }

    pub fn body_value(mut self, body: impl Into<JsonPayload>) -> Self {
        self.inner = self.inner.body_value(body);
        self
    }

    pub fn json_body<U>(mut self, body: &U) -> Result<Self>
    where
        U: Serialize,
    {
        self.inner = self.inner.json_body(body)?;
        Ok(self)
    }

    pub fn extra_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.inner = self.inner.extra_header(key, value);
        self
    }

    pub fn extra_query(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.inner = self.inner.extra_query(key, value);
        self
    }

    pub fn extra_body(mut self, key: impl Into<String>, value: impl Into<JsonPayload>) -> Self {
        self.inner = self.inner.extra_body(key, value);
        self
    }

    pub fn provider_option(
        mut self,
        key: impl Into<String>,
        value: impl Into<JsonPayload>,
    ) -> Self {
        self.inner = self.inner.provider_option(key, value);
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.inner = self.inner.timeout(timeout);
        self
    }

    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.inner = self.inner.max_retries(max_retries);
        self
    }

    pub fn cancellation_token(mut self, token: CancellationToken) -> Self {
        self.inner = self.inner.cancellation_token(token);
        self
    }

    pub async fn send(self) -> Result<Bytes> {
        self.inner.send().await
    }

    pub async fn send_with_meta(self) -> Result<ApiResponse<Bytes>> {
        self.inner.send_with_meta().await
    }

    pub async fn send_raw(self) -> Result<http::Response<Bytes>> {
        self.inner.send_raw().await
    }

    pub async fn send_raw_sse(self) -> Result<RawSseStream> {
        self.inner.send_raw_sse().await
    }

    pub async fn send_sse(self) -> Result<SseStream<Value>> {
        self.inner.send_sse().await
    }
}

/// 表示音频转写请求构建器。
#[derive(Debug, Clone)]
pub struct AudioTranscriptionRequestBuilder {
    inner: JsonRequestBuilder<AudioTranscription>,
}

impl AudioTranscriptionRequestBuilder {
    pub(crate) fn new(client: Client, stream: bool) -> Self {
        let inner = JsonRequestBuilder::new(
            client,
            "audio.transcriptions.create",
            Method::POST,
            "/audio/transcriptions",
        );
        Self {
            inner: if stream {
                inner.extra_body("stream", Value::Bool(true))
            } else {
                inner
            },
        }
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.inner = self.inner.multipart_text("model", model);
        self
    }

    pub fn file(mut self, file: UploadSource) -> Self {
        self.inner = self.inner.multipart_file("file", file);
        self
    }

    pub fn language(mut self, language: impl Into<String>) -> Self {
        self.inner = self.inner.multipart_text("language", language);
        self
    }

    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.inner = self.inner.multipart_text("prompt", prompt);
        self
    }

    pub fn response_format(mut self, response_format: impl Into<String>) -> Self {
        self.inner = self
            .inner
            .multipart_text("response_format", response_format);
        self
    }

    pub fn temperature(mut self, temperature: f32) -> Self {
        self.inner = self
            .inner
            .multipart_text("temperature", temperature.to_string());
        self
    }

    pub fn timestamp_granularity(mut self, granularity: impl Into<String>) -> Self {
        self.inner = self
            .inner
            .multipart_text("timestamp_granularities[]", granularity);
        self
    }

    pub fn body_value(mut self, body: impl Into<JsonPayload>) -> Self {
        self.inner = self.inner.body_value(body);
        self
    }

    pub fn json_body<U>(mut self, body: &U) -> Result<Self>
    where
        U: Serialize,
    {
        self.inner = self.inner.json_body(body)?;
        Ok(self)
    }

    pub fn multipart_text(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.inner = self.inner.multipart_text(name, value);
        self
    }

    pub fn multipart_file(mut self, name: impl Into<String>, file: UploadSource) -> Self {
        self.inner = self.inner.multipart_file(name, file);
        self
    }

    pub fn extra_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.inner = self.inner.extra_header(key, value);
        self
    }

    pub fn extra_query(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.inner = self.inner.extra_query(key, value);
        self
    }

    pub fn extra_body(mut self, key: impl Into<String>, value: impl Into<JsonPayload>) -> Self {
        self.inner = self.inner.extra_body(key, value);
        self
    }

    pub fn provider_option(
        mut self,
        key: impl Into<String>,
        value: impl Into<JsonPayload>,
    ) -> Self {
        self.inner = self.inner.provider_option(key, value);
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.inner = self.inner.timeout(timeout);
        self
    }

    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.inner = self.inner.max_retries(max_retries);
        self
    }

    pub fn cancellation_token(mut self, token: CancellationToken) -> Self {
        self.inner = self.inner.cancellation_token(token);
        self
    }

    pub async fn send(self) -> Result<AudioTranscription> {
        self.inner.send().await
    }

    pub async fn send_with_meta(self) -> Result<ApiResponse<AudioTranscription>> {
        self.inner.send_with_meta().await
    }

    pub async fn send_raw(self) -> Result<http::Response<Bytes>> {
        self.inner.send_raw().await
    }

    pub async fn send_raw_sse(self) -> Result<RawSseStream> {
        let client = self.inner.client.clone();
        let mut spec = self.inner.into_spec();
        spec.options.insert_header("accept", "text/event-stream");
        client.execute_raw_sse(spec).await
    }

    pub async fn send_sse(self) -> Result<SseStream<Value>> {
        let client = self.inner.client.clone();
        let mut spec = self.inner.into_spec();
        spec.options.insert_header("accept", "text/event-stream");
        client.execute_sse(spec).await
    }
}

/// 表示音频翻译请求构建器。
#[derive(Debug, Clone)]
pub struct AudioTranslationRequestBuilder {
    inner: JsonRequestBuilder<AudioTranslation>,
}

impl AudioTranslationRequestBuilder {
    pub(crate) fn new(client: Client) -> Self {
        Self {
            inner: JsonRequestBuilder::new(
                client,
                "audio.translations.create",
                Method::POST,
                "/audio/translations",
            ),
        }
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.inner = self.inner.multipart_text("model", model);
        self
    }

    pub fn file(mut self, file: UploadSource) -> Self {
        self.inner = self.inner.multipart_file("file", file);
        self
    }

    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.inner = self.inner.multipart_text("prompt", prompt);
        self
    }

    pub fn response_format(mut self, response_format: impl Into<String>) -> Self {
        self.inner = self
            .inner
            .multipart_text("response_format", response_format);
        self
    }

    pub fn temperature(mut self, temperature: f32) -> Self {
        self.inner = self
            .inner
            .multipart_text("temperature", temperature.to_string());
        self
    }

    pub fn body_value(mut self, body: impl Into<JsonPayload>) -> Self {
        self.inner = self.inner.body_value(body);
        self
    }

    pub fn json_body<U>(mut self, body: &U) -> Result<Self>
    where
        U: Serialize,
    {
        self.inner = self.inner.json_body(body)?;
        Ok(self)
    }

    pub fn multipart_text(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.inner = self.inner.multipart_text(name, value);
        self
    }

    pub fn multipart_file(mut self, name: impl Into<String>, file: UploadSource) -> Self {
        self.inner = self.inner.multipart_file(name, file);
        self
    }

    pub fn extra_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.inner = self.inner.extra_header(key, value);
        self
    }

    pub fn extra_query(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.inner = self.inner.extra_query(key, value);
        self
    }

    pub fn extra_body(mut self, key: impl Into<String>, value: impl Into<JsonPayload>) -> Self {
        self.inner = self.inner.extra_body(key, value);
        self
    }

    pub fn provider_option(
        mut self,
        key: impl Into<String>,
        value: impl Into<JsonPayload>,
    ) -> Self {
        self.inner = self.inner.provider_option(key, value);
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.inner = self.inner.timeout(timeout);
        self
    }

    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.inner = self.inner.max_retries(max_retries);
        self
    }

    pub fn cancellation_token(mut self, token: CancellationToken) -> Self {
        self.inner = self.inner.cancellation_token(token);
        self
    }

    pub async fn send(self) -> Result<AudioTranslation> {
        self.inner.send().await
    }

    pub async fn send_with_meta(self) -> Result<ApiResponse<AudioTranslation>> {
        self.inner.send_with_meta().await
    }

    pub async fn send_raw(self) -> Result<http::Response<Bytes>> {
        self.inner.send_raw().await
    }
}

/// 表示 fine-tuning job 创建构建器。
#[derive(Debug, Clone)]
pub struct FineTuningJobCreateRequestBuilder {
    state: TypedJsonRequestState<FineTuningJobCreateParams>,
}

impl FineTuningJobCreateRequestBuilder {
    pub(crate) fn new(client: Client) -> Self {
        Self {
            state: TypedJsonRequestState::new(client, FineTuningJobCreateParams::default()),
        }
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.state.params.model = Some(model.into());
        self
    }

    pub fn training_file(mut self, training_file: impl Into<String>) -> Self {
        self.state.params.training_file = Some(training_file.into());
        self
    }

    pub fn validation_file(mut self, validation_file: impl Into<String>) -> Self {
        self.state.params.validation_file = Some(validation_file.into());
        self
    }

    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.state.params.suffix = Some(suffix.into());
        self
    }

    pub fn seed(mut self, seed: u64) -> Self {
        self.state.params.seed = Some(seed);
        self
    }

    pub fn hyperparameters(
        mut self,
        hyperparameters: impl Into<FineTuningJobHyperparameters>,
    ) -> Self {
        self.state.params.hyperparameters = Some(hyperparameters.into());
        self
    }

    pub fn integration(mut self, integration: impl Into<FineTuningJobIntegration>) -> Self {
        self.state.params.integrations.push(integration.into());
        self
    }

    pub fn metadata(mut self, metadata: BTreeMap<String, String>) -> Self {
        self.state.params.metadata = metadata;
        self
    }

    pub fn params(mut self, params: FineTuningJobCreateParams) -> Self {
        self.state.params = params;
        self
    }

    pub fn body_value(mut self, body: impl Into<JsonPayload>) -> Self {
        self.state = self.state.body_value(body);
        self
    }

    pub fn json_body<U>(mut self, body: &U) -> Result<Self>
    where
        U: Serialize,
    {
        self.state = self.state.body_value(value_from(body)?);
        Ok(self)
    }

    pub fn extra_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.state = self.state.extra_header(key, value);
        self
    }

    pub fn extra_query(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.state = self.state.extra_query(key, value);
        self
    }

    pub fn extra_body(mut self, key: impl Into<String>, value: impl Into<JsonPayload>) -> Self {
        self.state = self.state.extra_body(key, value);
        self
    }

    pub fn provider_option(
        mut self,
        key: impl Into<String>,
        value: impl Into<JsonPayload>,
    ) -> Self {
        self.state = self.state.provider_option(key, value);
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.state = self.state.timeout(timeout);
        self
    }

    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.state = self.state.max_retries(max_retries);
        self
    }

    pub fn cancellation_token(mut self, token: CancellationToken) -> Self {
        self.state = self.state.cancellation_token(token);
        self
    }

    fn build_spec(self) -> Result<(Client, RequestSpec)> {
        if self.state.body_override.is_none() {
            if self
                .state
                .params
                .model
                .as_deref()
                .unwrap_or_default()
                .is_empty()
            {
                return Err(Error::MissingRequiredField { field: "model" });
            }
            if self
                .state
                .params
                .training_file
                .as_deref()
                .unwrap_or_default()
                .is_empty()
            {
                return Err(Error::MissingRequiredField {
                    field: "training_file",
                });
            }
        }
        self.state
            .build_spec("fine_tuning.jobs.create", "/fine_tuning/jobs")
    }

    pub async fn send(self) -> Result<FineTuningJob> {
        Ok(self.send_with_meta().await?.data)
    }

    pub async fn send_with_meta(self) -> Result<ApiResponse<FineTuningJob>> {
        let (client, spec) = self.build_spec()?;
        client.execute_json(spec).await
    }

    pub async fn send_raw(self) -> Result<http::Response<Bytes>> {
        let (client, spec) = self.build_spec()?;
        client.execute_raw_http(spec).await
    }
}

/// 表示 batch 创建构建器。
#[derive(Debug, Clone)]
pub struct BatchCreateRequestBuilder {
    state: TypedJsonRequestState<BatchCreateParams>,
}

impl BatchCreateRequestBuilder {
    pub(crate) fn new(client: Client) -> Self {
        Self {
            state: TypedJsonRequestState::new(client, BatchCreateParams::default()),
        }
    }

    pub fn input_file_id(mut self, input_file_id: impl Into<String>) -> Self {
        self.state.params.input_file_id = Some(input_file_id.into());
        self
    }

    pub fn endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.state.params.endpoint = Some(endpoint.into());
        self
    }

    pub fn completion_window(mut self, completion_window: impl Into<String>) -> Self {
        self.state.params.completion_window = Some(completion_window.into());
        self
    }

    pub fn metadata(mut self, metadata: BTreeMap<String, String>) -> Self {
        self.state.params.metadata = metadata;
        self
    }

    pub fn params(mut self, params: BatchCreateParams) -> Self {
        self.state.params = params;
        self
    }

    pub fn body_value(mut self, body: impl Into<JsonPayload>) -> Self {
        self.state = self.state.body_value(body);
        self
    }

    pub fn json_body<U>(mut self, body: &U) -> Result<Self>
    where
        U: Serialize,
    {
        self.state = self.state.body_value(value_from(body)?);
        Ok(self)
    }

    pub fn extra_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.state = self.state.extra_header(key, value);
        self
    }

    pub fn extra_query(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.state = self.state.extra_query(key, value);
        self
    }

    pub fn extra_body(mut self, key: impl Into<String>, value: impl Into<JsonPayload>) -> Self {
        self.state = self.state.extra_body(key, value);
        self
    }

    pub fn provider_option(
        mut self,
        key: impl Into<String>,
        value: impl Into<JsonPayload>,
    ) -> Self {
        self.state = self.state.provider_option(key, value);
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.state = self.state.timeout(timeout);
        self
    }

    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.state = self.state.max_retries(max_retries);
        self
    }

    pub fn cancellation_token(mut self, token: CancellationToken) -> Self {
        self.state = self.state.cancellation_token(token);
        self
    }

    fn build_spec(self) -> Result<(Client, RequestSpec)> {
        if self.state.body_override.is_none() {
            if self
                .state
                .params
                .input_file_id
                .as_deref()
                .unwrap_or_default()
                .is_empty()
            {
                return Err(Error::MissingRequiredField {
                    field: "input_file_id",
                });
            }
            if self
                .state
                .params
                .endpoint
                .as_deref()
                .unwrap_or_default()
                .is_empty()
            {
                return Err(Error::MissingRequiredField { field: "endpoint" });
            }
            if self
                .state
                .params
                .completion_window
                .as_deref()
                .unwrap_or_default()
                .is_empty()
            {
                return Err(Error::MissingRequiredField {
                    field: "completion_window",
                });
            }
        }
        let endpoint = endpoints::batches::BATCHES_CREATE;
        self.state.build_spec(endpoint.id, endpoint.template)
    }

    pub async fn send(self) -> Result<Batch> {
        Ok(self.send_with_meta().await?.data)
    }

    pub async fn send_with_meta(self) -> Result<ApiResponse<Batch>> {
        let (client, spec) = self.build_spec()?;
        client.execute_json(spec).await
    }

    pub async fn send_raw(self) -> Result<http::Response<Bytes>> {
        let (client, spec) = self.build_spec()?;
        client.execute_raw_http(spec).await
    }
}
