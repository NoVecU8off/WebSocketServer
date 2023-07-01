use crate::{ws, Client, Clients, Result};
use futures::Future;
use uuid::Uuid;
use warp::{hyper::StatusCode, reply::json, ws::Message, Reply};

#[derive(serde::Deserialize, serde::Serialize)]
pub struct RegisterRequest {
    user_id: usize,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct RegisterResponce {
    url: String,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Event {
    topic: String,
    user_id: Option<usize>,
    message: String,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct TopicsRequest {
    pub topics: Vec<String>,
}

pub async fn ws_handler(ws: warp::ws::Ws, id: String, clients: Clients) -> Result<impl Reply> {
    let clients_clone = clients.clone();
    let client = clients.lock().await.get(&id).cloned();
    match client {
        Some(c) => Ok(ws.on_upgrade(move |socket| {
            ws::client_connection(socket, id, clients_clone.clone(), c.clone())
        })),
        None => Err(warp::reject::not_found()),
    }
}

pub async fn register_handler(body: RegisterRequest, clients: Clients) -> Result<impl Reply> {
    let user_id = body.user_id;
    let uuid = Uuid::new_v4().simple().to_string();

    register_client(uuid.clone(), user_id, clients).await;
    Ok(json(&RegisterResponce {
        url: format!("ws://127.0.0.1:8080/ws/{}", uuid),
    }))
}

pub async fn unregister_handler(id: String, clients: Clients) -> Result<impl Reply> {
    clients.lock().await.remove(&id);
    Ok(StatusCode::OK)
}

pub async fn publish_handler(body: Event, clients: Clients) -> Result<impl Reply> {
    clients
        .lock()
        .await
        .iter_mut()
        .filter(|(_, client)| match body.user_id {
            Some(v) => client.user_id == v,
            None => true,
        })
        .filter(|(_, client)| client.topics.contains(&body.topic))
        .for_each(|(_, client)| {
            if let Some(sender) = &client.sender {
                let message = body.message.clone(); // Clone the message field
                let _ = sender.send(Ok(Message::text(message)));
            }
        });

    Ok(StatusCode::OK)
}

async fn register_client(id: String, user_id: usize, clients: Clients) {
    clients.lock().await.insert(
        id,
        Client {
            user_id,
            topics: vec![String::from("cats")],
            sender: None,
        },
    );
}

pub fn health_handler() -> impl Future<Output = Result<impl Reply>> {
    futures::future::ready(Ok(StatusCode::OK))
}
