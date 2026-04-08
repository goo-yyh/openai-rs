use openai_rs::Client;

#[tokio::main]
async fn main() {
    let client = Client::builder()
        .api_key("sk-fixture")
        .base_url("http://127.0.0.1:4010/v1")
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let _socket = client
        .responses()
        .ws()
        .extra_header("x-fixture", "responses_ws_client");
}
