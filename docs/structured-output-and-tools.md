# Structured Output 与 Tool Runner

## Structured Output

需要开启 `structured-output` feature。

```rust,ignore
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
struct Answer {
    answer: String,
}

let parsed = client
    .responses()
    .parse::<Answer>()
    .model("gpt-5.4")
    .input_text("返回 JSON")
    .send()
    .await?;
```

SDK 会尝试：

- 读取 assistant / response 的文本输出
- 自动去掉 Markdown 代码块外壳
- 反序列化为目标结构体

## Tool Runner

需要开启 `tool-runner` feature。

```rust,ignore
use openai_rs::ToolDefinition;
use serde_json::json;

let tool = ToolDefinition::new(
    "get_weather",
    Some("查询天气"),
    json!({
        "type": "object",
        "properties": {
            "city": { "type": "string" }
        },
        "required": ["city"]
    }),
    |arguments| async move {
        Ok(json!({
            "city": arguments["city"].as_str().unwrap_or("unknown"),
            "weather": "sunny"
        }))
    },
);
```

Tool runner 当前策略：

- 自动把注册工具转换成 chat tools
- 自动执行工具调用
- 自动把工具输出写回消息历史
- 在达到 `max_rounds` 后停止，避免无限循环
