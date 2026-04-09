use openai_rs::Client;

fn main() {
    let client = Client::builder()
        .api_key("sk-canary")
        .base_url("http://127.0.0.1:4010/v1")
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let _ = client.responses().create().model("gpt-5.4").input_text("hello");
    let _ = client.responses().compact();
    let _ = client.chat().completions().create().model("gpt-5.4").message_user("hello");
    let _ = client.files().list();
    let _ = client.uploads().create();
    let _ = client.images().generate();
    let _ = client.videos().edit();
    let _ = client.videos().extend();
}
