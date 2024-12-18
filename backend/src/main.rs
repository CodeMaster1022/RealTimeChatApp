use std::collections::HashMap;
use rocket::futures::{SinkExt, StreamExt, stream::SplitSink};
use rocket_ws::{WebSocket, Channel, stream::DuplexStream, Message};
use std::sync::atomic::{AtomicUsize, Ordering};
use rocket::{State, tokio::sync::Mutex};

static USER_ID_COUNTER : AtomicUsize = AtomicUsize::new(0);

#[derive(Default)]
struct ChatRoom {
    connections: Mutex<HashMap<usize, SplitSink<DuplexStream, Message>>>
}
impl ChatRoom {
    pub async fn add(&self, id: usize, sink: SplitSink<DuplexStream, Message>) {
        let mut conns = self.connections.lock().await;
        conns.insert(id, sink);
    }
    pub async fn remove(&self, id: usize) {
        let mut conns = self.connections.lock().await;
        conns.remove(&id);
    }
    pub async fn broadcast_message(&self, message: Message){
        let mut conns = self.connections.lock().await;
        for(_id, sink) in conns.iter_mut() {
            let _ = sink.send(message.clone()).await;
        }         
    }
}
#[rocket::get("/")]
fn chat<'r>(ws: WebSocket, state: &'r State<ChatRoom>) -> Channel<'r> {
    ws.channel(move |stream| Box::pin(async move {
        let user_id = USER_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let (ws_sink, mut ws_stream) = stream.split();
        state.add(user_id, ws_sink).await;
        while let Some(message) =ws_stream.next().await {
            state.broadcast_message(message?).await;
        }
        state.remove(user_id).await;
        Ok(())
    }))
}
#[rocket::main] 
async fn main() {
    let _ = rocket::build()
        .mount("/", rocket::routes![
            chat
        ])
        .manage(ChatRoom::default())
        .launch()
        .await;
}
