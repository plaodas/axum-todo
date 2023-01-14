mod handlers;
mod repositories;

use crate::repositories::{
    TodoRepository,
    TodoRepositoryForMemory
};
use axum::{
    extract::Extension,
    routing::{get,post}, 
    Router,
};
use handlers::create_todo;
use std::net::SocketAddr;
use std::{
    env,
    sync::Arc,
};


#[tokio::main]
async fn main() {
    // initialize logging
    let log_level = env::var("RUST_LOG").unwrap_or("info".to_string());
    env::set_var("RUST_LOG", log_level);
    tracing_subscriber::fmt::init();

    // let app = create_app();
    let repository = TodoRepositoryForMemory::new();
    let app = create_app(repository);

    // let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 3000);
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::debug!("listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

// fn create_app()-> Router{
fn create_app<T: TodoRepository>(repository:T) -> Router {
    Router::new()
    .route("/", get(root))
    // .route("/users", post(create_user))
    .route("/todos", post(create_todo::<T>))
    .layer(Extension(Arc::new(repository)))
}


async fn root() -> &'static str {
    "Hello, world!"
}

// async fn create_user( Json(payload): Json<CreateUser>) -> impl IntoResponse {
//     let user = User {
//         id: 1337,
//         username: payload.username,
//     };

//     (StatusCode::CREATED, Json(user))
// }

// #[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
// struct CreateUser{
//     username: String,
// }

// #[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
// struct User{
//     id: u64,
//     username: String,
// }

#[cfg(test)]
mod test {
    use super::*;
    // use axum::{
    //     body::Body,
    //     http::{header, Method, Request},
    // };
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    #[tokio::test]
    async fn should_return_hello_world(){
        let repository = TodoRepositoryForMemory::new();
        let req = Request::builder().uri("/").body(Body::empty()).unwrap();
        // let res = create_app().oneshot(req).await.unwrap();
        let res = create_app(repository).oneshot(req).await.unwrap();
        
        let bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
        let body = String::from_utf8(bytes.to_vec()).unwrap();
        assert_eq!(body, "Hello, world!");
    }

    // #[tokio::test]
    // async fn should_return_user_data(){
    //     let repository = TodoRepositoryForMemory::new();
    //     let req = Request::builder()
    //         .uri("/users")
    //         .method(Method::POST)
    //         .header(header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
    //         .body(Body::from(r#"{ "username":"テストの部屋" }"#))
    //         .unwrap();
        
    //     // let res = create_app().oneshot(req).await.unwrap();
    //     let res = create_app(repository).oneshot(req).await.unwrap();

    //     let bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
    //     let body = String::from_utf8(bytes.to_vec()).unwrap();

    //     let user:User = serde_json::from_str(&body).expect("cannot convert User instance.");
    //     assert_eq!(user, User{id:1337, username:"テストの部屋".to_string()});
    // }











}