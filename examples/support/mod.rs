#![allow(dead_code)]

use std::error::Error;
use std::path::PathBuf;

use openai_rs::{ChatCompletionMessage, Client, UploadSource};
use serde_json::{Value, json};

pub type ExampleResult<T = ()> = Result<T, Box<dyn Error>>;

pub fn openai_client() -> ExampleResult<Client> {
    Ok(Client::builder()
        .api_key(std::env::var("OPENAI_API_KEY")?)
        .build()?)
}

pub fn azure_client() -> ExampleResult<Client> {
    Ok(Client::builder()
        .azure_endpoint(std::env::var("AZURE_OPENAI_ENDPOINT")?)
        .azure_api_version(
            std::env::var("OPENAI_API_VERSION").unwrap_or_else(|_| "2024-02-15-preview".into()),
        )
        .azure_deployment(std::env::var("AZURE_OPENAI_DEPLOYMENT")?)
        .api_key(std::env::var("AZURE_OPENAI_API_KEY")?)
        .build()?)
}

pub fn demo_messages() -> Vec<ChatCompletionMessage> {
    vec![
        ChatCompletionMessage::system("你是一个擅长推荐图书的助手。"),
        ChatCompletionMessage::user(
            "我很喜欢《To Kill a Mockingbird》，请推荐一本相似的书，并说明理由。",
        ),
    ]
}

pub fn book_tools_json() -> Value {
    json!([
        {
            "type": "function",
            "function": {
                "name": "list_books",
                "description": "按题材列出书籍",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "genre": {
                            "type": "string",
                            "enum": ["historical", "memoir", "nonfiction", "romance"]
                        }
                    },
                    "required": ["genre"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "search_books",
                "description": "按书名搜索书籍",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" }
                    },
                    "required": ["name"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "get_book",
                "description": "根据 id 获取书籍详情",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" }
                    },
                    "required": ["id"]
                }
            }
        }
    ])
}

pub fn query_tool_json() -> Value {
    json!([
        {
            "type": "function",
            "function": {
                "name": "query_orders",
                "description": "查询订单系统",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "table_name": {
                            "type": "string",
                            "enum": ["orders", "customers", "products"]
                        },
                        "columns": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "conditions": {
                            "type": "array",
                            "items": { "type": "object" }
                        },
                        "order_by": {
                            "type": "string",
                            "enum": ["asc", "desc"]
                        }
                    },
                    "required": ["table_name", "columns", "conditions", "order_by"]
                }
            }
        }
    ])
}

pub fn dispatch_book_tool(name: &str, arguments: &str) -> ExampleResult<Value> {
    let arguments = if arguments.trim().is_empty() {
        json!({})
    } else {
        serde_json::from_str::<Value>(arguments)?
    };

    let books = json!([
        {
            "id": "a1",
            "name": "To Kill a Mockingbird",
            "genre": "historical",
            "description": "关于同理心、偏见与成长的经典小说。"
        },
        {
            "id": "a2",
            "name": "All the Light We Cannot See",
            "genre": "historical",
            "description": "以战争为背景，交织多个角色命运。"
        },
        {
            "id": "a3",
            "name": "Where the Crawdads Sing",
            "genre": "historical",
            "description": "兼具成长、孤独与悬疑元素。"
        }
    ]);

    let result = match name {
        "list_books" => {
            let genre = arguments["genre"].as_str().unwrap_or_default();
            let items = books
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .filter(|item| item["genre"] == genre)
                .map(|item| json!({"id": item["id"], "name": item["name"]}))
                .collect::<Vec<_>>();
            Value::Array(items)
        }
        "search_books" => {
            let name = arguments["name"]
                .as_str()
                .unwrap_or_default()
                .to_ascii_lowercase();
            let items = books
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .filter(|item| {
                    item["name"]
                        .as_str()
                        .unwrap_or_default()
                        .to_ascii_lowercase()
                        .contains(&name)
                })
                .map(|item| json!({"id": item["id"], "name": item["name"]}))
                .collect::<Vec<_>>();
            Value::Array(items)
        }
        "get_book" => {
            let id = arguments["id"].as_str().unwrap_or_default();
            books
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .find(|item| item["id"] == id)
                .cloned()
                .unwrap_or_else(|| json!({"error": "book not found"}))
        }
        other => return Err(format!("未知工具: {other}").into()),
    };

    Ok(result)
}

#[cfg(feature = "tool-runner")]
pub fn weather_tool() -> openai_rs::ToolDefinition {
    use openai_rs::ToolDefinition;

    ToolDefinition::new(
        "get_weather",
        Some("根据城市查询天气"),
        json!({
            "type": "object",
            "properties": {
                "city": { "type": "string" },
                "country": { "type": "string" },
                "units": {
                    "type": "string",
                    "enum": ["c", "f"]
                }
            },
            "required": ["city", "country"]
        }),
        |arguments: Value| async move {
            Ok(json!({
                "city": arguments["city"].as_str().unwrap_or("unknown"),
                "country": arguments["country"].as_str().unwrap_or("unknown"),
                "units": arguments["units"].as_str().unwrap_or("c"),
                "temperature": 23,
                "conditions": "sunny"
            }))
        },
    )
}

pub fn sample_training_file() -> UploadSource {
    UploadSource::from_bytes(
        include_str!("../fine_tuning_data.jsonl")
            .as_bytes()
            .to_vec(),
        "fine_tuning_data.jsonl",
    )
    .with_mime_type("application/jsonl")
}

pub fn sample_audio_file() -> UploadSource {
    UploadSource::from_bytes(b"RIFF....WAVEfmt ".to_vec(), "sample.wav").with_mime_type("audio/wav")
}

pub fn output_path(filename: &str) -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(filename)
}
