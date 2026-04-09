use openai_rs::Client;

fn main() {
    let client = Client::builder()
        .api_key("sk-canary")
        .base_url("http://127.0.0.1:4010/v1")
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let _ = client.realtime().ws();
    let _ = client.responses().ws();
    let _ = client.realtime().client_secrets().create();
}
