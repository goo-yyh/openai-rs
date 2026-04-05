# Azure OpenAI 接入说明

`openai-rs` 对 Azure OpenAI 提供一等支持，重点处理以下差异：

- `azure_endpoint`
- `deployment`
- `api-version`
- `api-key` Header 鉴权
- Azure AD / Entra Bearer Token 鉴权

## 最小示例

```rust,ignore
use openai_rs::Client;

let client = Client::builder()
    .azure_endpoint("https://example-resource.openai.azure.com")
    .azure_api_version("2024-02-15-preview")
    .azure_deployment("gpt-4o-prod")
    .api_key(std::env::var("AZURE_OPENAI_API_KEY")?)
    .build()?;
```

## Bearer Token

如果你使用 Azure AD / Entra Token，可以直接切换到 Bearer 模式：

```rust,ignore
use openai_rs::Client;
use secrecy::SecretString;

let client = Client::builder()
    .azure_endpoint("https://example-resource.openai.azure.com")
    .azure_api_version("2024-02-15-preview")
    .azure_ad_token_provider(|| async {
        Ok(SecretString::new("azure-ad-token".into()))
    })
    .build()?;
```

## 行为说明

- `azure_endpoint` 应传资源级地址，SDK 会自动补上 `/openai`
- 配置了默认 deployment 后，常见请求会自动注入 deployment 路径
- Realtime WebSocket 会自动把 deployment 放到 query 中
- 若 `base_url` 与 `azure_endpoint` 同时设置，构建器会直接报错

## 排障建议

- 先确认 `api-version` 是否与目标资源兼容
- 再确认 deployment 名称是否真实存在
- 如果使用 Bearer Token，优先检查 token 是否仍然有效
