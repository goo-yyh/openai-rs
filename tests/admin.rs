use serde_json::json;
use wiremock::matchers::{body_json, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use openai_core::{Client, Error};

fn test_client(server: &MockServer) -> Client {
    Client::builder()
        .api_key("sk-user")
        .admin_api_key("sk-admin")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap()
}

#[tokio::test]
async fn test_should_use_admin_organization_resources() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/organization/admin_api_keys"))
        .and(header("authorization", "Bearer sk-admin"))
        .and(body_json(json!({"name": "ops-admin"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "key_1",
            "object": "organization.admin_api_key",
            "name": "ops-admin",
            "value": "sk-admin-redacted"
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/organization/audit_logs"))
        .and(query_param("actor_id", "user_1"))
        .and(query_param("limit", "20"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "data": [{"id": "audit_1", "type": "login"}],
            "has_more": false
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/organization/certificates/activate"))
        .and(body_json(json!({"certificate_ids": ["cert_1"]})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "data": [{"id": "cert_1", "active": true}]
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/organization/usage/file_search_calls"))
        .and(query_param("start_time", "1700000000"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "organization.usage.file_search_calls",
            "data": [{"num_model_requests": 1}]
        })))
        .mount(&server)
        .await;

    let client = test_client(&server);

    let key = client
        .admin()
        .organization()
        .admin_api_keys()
        .create()
        .body_value(json!({"name": "ops-admin"}))
        .send()
        .await
        .unwrap();
    assert_eq!(key["id"], "key_1");
    assert_eq!(key["value"], "sk-admin-redacted");

    let audit_logs = client
        .admin()
        .organization()
        .audit_logs()
        .list()
        .limit(20)
        .extra_query("actor_id", "user_1")
        .send()
        .await
        .unwrap();
    assert_eq!(audit_logs.data[0]["id"], "audit_1");

    let activated = client
        .admin()
        .organization()
        .certificates()
        .activate()
        .body_value(json!({"certificate_ids": ["cert_1"]}))
        .send()
        .await
        .unwrap();
    assert_eq!(activated.data[0]["active"], true);

    let usage = client
        .admin()
        .organization()
        .usage()
        .file_search_calls()
        .extra_query("start_time", "1700000000")
        .send()
        .await
        .unwrap();
    assert_eq!(usage["data"][0]["num_model_requests"], 1);
}

#[tokio::test]
async fn test_should_require_admin_api_key_for_admin_resources() {
    let server = MockServer::start().await;
    let client = Client::builder()
        .api_key("sk-user")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let error = client
        .admin()
        .organization()
        .audit_logs()
        .list()
        .send()
        .await
        .unwrap_err();

    assert!(matches!(error, Error::MissingCredentials));
}

#[tokio::test]
async fn test_should_use_admin_project_resource_paths() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/organization/projects"))
        .and(query_param("include_archived", "false"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "data": [{"id": "proj_1", "name": "Production"}],
            "has_more": false
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/organization/projects/proj_1/archive"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "proj_1",
            "archived": true
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/organization/projects/proj_1/service_accounts"))
        .and(body_json(json!({"name": "worker"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "svc_1",
            "object": "organization.project.service_account",
            "api_key": {"value": "sk-service-redacted"}
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/projects/proj_1/users/user_1/roles"))
        .and(body_json(json!({"role_id": "role_1"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "user.role",
            "role": {"id": "role_1"},
            "user": {"id": "user_1"}
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/projects/proj_1/groups/group_1/roles"))
        .and(body_json(json!({"role_id": "role_1"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "group.role",
            "role": {"id": "role_1"},
            "group": {"id": "group_1"}
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/organization/projects/proj_1/model_permissions"))
        .and(body_json(json!({"allow": [{"model": "gpt-5.4"}]})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "organization.project.model_permissions",
            "allow": [{"model": "gpt-5.4"}]
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(
            "/organization/projects/proj_1/hosted_tool_permissions",
        ))
        .and(body_json(json!({"web_search": {"allowed": true}})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "organization.project.hosted_tool_permissions",
            "web_search": {"allowed": true}
        })))
        .mount(&server)
        .await;

    let client = test_client(&server);
    let projects = client
        .admin()
        .organization()
        .projects()
        .list()
        .extra_query("include_archived", "false")
        .send()
        .await
        .unwrap();
    assert_eq!(projects.data[0]["id"], "proj_1");

    let archived = client
        .admin()
        .organization()
        .projects()
        .archive("proj_1")
        .send()
        .await
        .unwrap();
    assert_eq!(archived["archived"], true);

    let service_account = client
        .admin()
        .organization()
        .projects()
        .service_accounts()
        .create("proj_1")
        .body_value(json!({"name": "worker"}))
        .send()
        .await
        .unwrap();
    assert_eq!(service_account["api_key"]["value"], "sk-service-redacted");

    let user_role = client
        .admin()
        .organization()
        .projects()
        .users()
        .roles()
        .create("proj_1", "user_1")
        .body_value(json!({"role_id": "role_1"}))
        .send()
        .await
        .unwrap();
    assert_eq!(user_role["object"], "user.role");

    let group_role = client
        .admin()
        .organization()
        .projects()
        .groups()
        .roles()
        .create("proj_1", "group_1")
        .body_value(json!({"role_id": "role_1"}))
        .send()
        .await
        .unwrap();
    assert_eq!(group_role["object"], "group.role");

    let model_permissions = client
        .admin()
        .organization()
        .projects()
        .model_permissions()
        .update("proj_1")
        .body_value(json!({"allow": [{"model": "gpt-5.4"}]}))
        .send()
        .await
        .unwrap();
    assert_eq!(model_permissions["allow"][0]["model"], "gpt-5.4");

    let hosted_tool_permissions = client
        .admin()
        .organization()
        .projects()
        .hosted_tool_permissions()
        .update("proj_1")
        .body_value(json!({"web_search": {"allowed": true}}))
        .send()
        .await
        .unwrap();
    assert_eq!(hosted_tool_permissions["web_search"]["allowed"], true);
}
