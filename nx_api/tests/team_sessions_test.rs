//! 团队会话 API 集成测试
//!
//! 启动真实 HTTP 服务器，端到端测试 list、detail 和 delete 端点。

use nexus_orchestrator::session::ChainResult;
use nexus_orchestrator::session_store::SessionStore;
use nexus_orchestrator::team::{AgentId, AgentRole};
use std::net::SocketAddr;
use uuid::Uuid;

fn make_result(task: &str, agent_count: usize) -> ChainResult {
    let mut agents = Vec::new();
    for i in 0..agent_count {
        agents.push(nexus_orchestrator::session::AgentResult {
            agent_id: AgentId::new(),
            role: match i % 4 {
                0 => AgentRole::Architect,
                1 => AgentRole::Developer,
                2 => AgentRole::Reviewer,
                _ => AgentRole::Tester,
            },
            agent_name: format!("agent-{}", i),
            text: format!("result from agent {}", i),
            duration_ms: (i as u64 + 1) * 1000,
            attempts: 1,
        });
    }
    let total = agents.iter().map(|a| a.duration_ms).sum();
    ChainResult {
        execution_id: Uuid::new_v4(),
        task: task.into(),
        agent_results: agents,
        total_duration_ms: total,
    }
}

/// 创建一个绕过系统代理的 HTTP 客户端（测试用）
fn no_proxy_client() -> reqwest::Client {
    reqwest::Client::builder().no_proxy().build().unwrap()
}

/// 启动服务器并等待就绪
async fn spawn_server(router: axum::Router) -> SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let client = no_proxy_client();
    for _ in 0..20 {
        if client.get(format!("http://{}/", addr)).send().await.is_ok() {
            return addr;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    addr
}

// ── List ────────────────────────────────────────────────────────────

#[tokio::test]
async fn list_returns_seeded_sessions() {
    let store = SessionStore::in_memory().unwrap();
    let r1 = make_result("实现用户登录", 2);
    let r2 = make_result("添加数据库迁移", 3);
    store.save(&r1).unwrap();
    store.save(&r2).unwrap();

    let router = nx_api::routes::team_sessions::create_router_with_store(store);
    let addr = spawn_server(router).await;

    let resp = no_proxy_client()
        .get(format!("http://{}/", addr))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "list 端点应返回 200");

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["ok"], true);
    let data = body["data"].as_array().unwrap();
    assert_eq!(data.len(), 2, "应返回 2 条会话");
    let tasks: Vec<&str> = data.iter().map(|s| s["task"].as_str().unwrap()).collect();
    assert!(tasks.contains(&"实现用户登录"), "应包含第一条任务");
    assert!(tasks.contains(&"添加数据库迁移"), "应包含第二条任务");
    for item in data {
        assert_eq!(item["status"], "completed");
    }
}

#[tokio::test]
async fn empty_store_returns_empty_list() {
    let store = SessionStore::in_memory().unwrap();
    let router = nx_api::routes::team_sessions::create_router_with_store(store);
    let addr = spawn_server(router).await;

    let resp = no_proxy_client()
        .get(format!("http://{}/", addr))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["ok"], true);
    assert!(body["data"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn list_returns_ok_even_when_db_has_corrupted_data() {
    let store = SessionStore::in_memory().unwrap();
    // 插入有效数据
    store.save(&make_result("valid task", 1)).unwrap();
    // 直接写入损坏的 JSON
    {
        let conn = store.conn.lock();
        conn.execute(
            "INSERT INTO team_sessions (execution_id, task, chain_result, status) VALUES (?1, ?2, ?3, 'completed')",
            rusqlite::params!["corrupted-id", "corrupted task", "{{{not valid json}}"],
        ).unwrap();
    }

    let router = nx_api::routes::team_sessions::create_router_with_store(store);
    let addr = spawn_server(router).await;

    // 损坏的行应被静默跳过
    let resp = no_proxy_client()
        .get(format!("http://{}/", addr))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["ok"], true);
    let data = body["data"].as_array().unwrap();
    assert_eq!(data.len(), 1, "损坏数据行应被跳过，只返回有效行");
    assert_eq!(data[0]["task"], "valid task");
}

// ── Get ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_existing_session_returns_detail() {
    let store = SessionStore::in_memory().unwrap();
    let r = make_result("重构 auth 模块", 3);
    let id = r.execution_id.to_string();
    store.save(&r).unwrap();

    let router = nx_api::routes::team_sessions::create_router_with_store(store);
    let addr = spawn_server(router).await;

    let detail_url = format!("http://{}/{}", addr, id);
    let resp = no_proxy_client().get(&detail_url).send().await.unwrap();
    let status = resp.status();
    let body_text = resp.text().await.unwrap();
    let body: serde_json::Value =
        serde_json::from_str(&body_text).expect(&format!("body: {}", body_text));

    assert_eq!(status, 200, "详情端点应返回 200, body: {}", body_text);
    assert_eq!(body["ok"], true);
    let data = &body["data"];
    assert_eq!(data["task"], "重构 auth 模块");
    assert_eq!(data["execution_id"], id);
    assert_eq!(data["agent_results"].as_array().unwrap().len(), 3);
    assert_eq!(data["total_duration_ms"], 6000);
}

#[tokio::test]
async fn get_nonexistent_returns_404() {
    let store = SessionStore::in_memory().unwrap();
    let router = nx_api::routes::team_sessions::create_router_with_store(store);
    let addr = spawn_server(router).await;

    let url = format!("http://{}/{}", addr, Uuid::new_v4());
    let resp = no_proxy_client().get(&url).send().await.unwrap();
    let status = resp.status();
    let body_text = resp.text().await.unwrap();
    let body: serde_json::Value =
        serde_json::from_str(&body_text).expect(&format!("404 body should be json: {}", body_text));

    assert_eq!(status, 404, "不存在会话应返回 404");
    assert_eq!(body["ok"], false);
    assert!(
        body["error"].as_str().unwrap().contains("不存在"),
        "错误信息应包含'不存在'"
    );
}

#[tokio::test]
async fn get_with_invalid_uuid_returns_404() {
    let store = SessionStore::in_memory().unwrap();
    let router = nx_api::routes::team_sessions::create_router_with_store(store);
    let addr = spawn_server(router).await;

    let resp = no_proxy_client()
        .get(format!("http://{}/{}", addr, "not-a-uuid"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["ok"], false);
    assert!(
        body["error"].as_str().unwrap().contains("不存在"),
        "错误信息应包含'不存在'"
    );
}

#[tokio::test]
async fn get_deleted_session_returns_404() {
    let store = SessionStore::in_memory().unwrap();
    let r = make_result("to-delete", 1);
    let id = r.execution_id.to_string();
    store.save(&r).unwrap();
    store.delete(&id).unwrap();

    let router = nx_api::routes::team_sessions::create_router_with_store(store);
    let addr = spawn_server(router).await;

    let resp = no_proxy_client()
        .get(format!("http://{}/{}", addr, &id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["ok"], false);
}

// ── Delete ──────────────────────────────────────────────────────────

#[tokio::test]
async fn delete_existing_session_returns_ok() {
    let store = SessionStore::in_memory().unwrap();
    let r = make_result("删除测试", 1);
    let id = r.execution_id.to_string();
    store.save(&r).unwrap();

    let router = nx_api::routes::team_sessions::create_router_with_store(store);
    let addr = spawn_server(router).await;

    let client = no_proxy_client();
    let resp = client
        .delete(format!("http://{}/{}", addr, id))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200, "删除成功应返回 200");
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["ok"], true);
    assert_eq!(body["data"]["deleted"], id);
}

#[tokio::test]
async fn delete_nonexistent_session_returns_404() {
    let store = SessionStore::in_memory().unwrap();
    let router = nx_api::routes::team_sessions::create_router_with_store(store);
    let addr = spawn_server(router).await;

    let client = no_proxy_client();
    let resp = client
        .delete(format!("http://{}/{}", addr, Uuid::new_v4()))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 404, "不存在会话删除应返回 404");
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["ok"], false);
    assert!(
        body["error"].as_str().unwrap().contains("不存在"),
        "错误信息应包含'不存在'"
    );
}

#[tokio::test]
async fn delete_twice_returns_404_second_time() {
    let store = SessionStore::in_memory().unwrap();
    let r = make_result("二次删除", 1);
    let id = r.execution_id.to_string();
    store.save(&r).unwrap();

    let router = nx_api::routes::team_sessions::create_router_with_store(store);
    let addr = spawn_server(router).await;
    let client = no_proxy_client();

    // 第一次删除成功
    let resp1 = client
        .delete(format!("http://{}/{}", addr, &id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp1.status(), 200);

    // 第二次删除返回 404
    let resp2 = client
        .delete(format!("http://{}/{}", addr, &id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp2.status(), 404);
}

#[tokio::test]
async fn delete_removes_from_list() {
    let store = SessionStore::in_memory().unwrap();
    let r1 = make_result("task1", 1);
    let r2 = make_result("task2", 1);
    let id1 = r1.execution_id.to_string();
    store.save(&r1).unwrap();
    store.save(&r2).unwrap();

    let router = nx_api::routes::team_sessions::create_router_with_store(store);
    let addr = spawn_server(router).await;
    let client = no_proxy_client();

    // 删除第一个
    client
        .delete(format!("http://{}/{}", addr, &id1))
        .send()
        .await
        .unwrap();

    // 列表应只有一个
    let resp = no_proxy_client()
        .get(format!("http://{}/", addr))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
    assert_eq!(body["data"][0]["task"], "task2");
}

// ── Full lifecycle ──────────────────────────────────────────────────

#[tokio::test]
async fn full_lifecycle_create_list_get_delete() {
    let store = SessionStore::in_memory().unwrap();
    let r = make_result("lifecycle test", 2);
    let id = r.execution_id.to_string();
    store.save(&r).unwrap();

    let router = nx_api::routes::team_sessions::create_router_with_store(store);
    let addr = spawn_server(router).await;
    let client = no_proxy_client();

    // 1. List — 包含新建的 session
    let list_resp = no_proxy_client()
        .get(format!("http://{}/", addr))
        .send()
        .await
        .unwrap();
    let list_body: serde_json::Value = list_resp.json().await.unwrap();
    assert_eq!(list_body["data"].as_array().unwrap().len(), 1);

    // 2. Get — 获取详情
    let detail_resp = no_proxy_client()
        .get(format!("http://{}/{}", addr, &id))
        .send()
        .await
        .unwrap();
    assert_eq!(detail_resp.status(), 200);
    let detail_body: serde_json::Value = detail_resp.json().await.unwrap();
    assert_eq!(detail_body["data"]["execution_id"], id);

    // 3. Delete — 删除
    let del_resp = client
        .delete(format!("http://{}/{}", addr, &id))
        .send()
        .await
        .unwrap();
    assert_eq!(del_resp.status(), 200);

    // 4. Verify — 确认已删除
    let verify_list = no_proxy_client()
        .get(format!("http://{}/", addr))
        .send()
        .await
        .unwrap();
    let verify_body: serde_json::Value = verify_list.json().await.unwrap();
    assert!(verify_body["data"].as_array().unwrap().is_empty());

    let verify_get = no_proxy_client()
        .get(format!("http://{}/{}", addr, &id))
        .send()
        .await
        .unwrap();
    assert_eq!(verify_get.status(), 404);
}
