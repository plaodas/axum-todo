mod handlers;
mod repositories;

use hyper::header::CONTENT_TYPE;
use crate::repositories::{
    label::LabelRepositoryForDb,
    todo::{
        TodoRepository,
        TodoRepositoryForDb,
    }
};
use axum::{
    extract::Extension,
    routing::{get,post,delete}, 
    Router,
};
use handlers::{
    label::{all_label, create_label, delete_label},
    todo::{all_todo, create_todo, delete_todo, find_todo, update_todo }
};
use std::net::SocketAddr;
use std::{
    env,
    sync::Arc,
};
use repositories::label::LabelRepository;
use sqlx::PgPool;
use dotenv::dotenv;
use tower_http::cors::{Any, CorsLayer, AllowOrigin};

#[tokio::main]
async fn main() {
    // initialize logging
    let log_level = env::var("RUST_LOG").unwrap_or("info".to_string());
    env::set_var("RUST_LOG", log_level);
    tracing_subscriber::fmt::init();
    dotenv().ok();

    let database_url = &env::var("DATABASE_URL").expect("undefined [DATABASE_URL]");
    tracing::debug!("start connect database..");

    let pool = PgPool::connect(database_url)
        .await
        .expect(&format!("fail connect database, url is [{}]", database_url));

    // let repository = TodoRepositoryForDb::new(pool.clone());
    // let app = create_app(repository);
    let app = create_app(
        TodoRepositoryForDb::new(pool.clone()),
        LabelRepositoryForDb::new(pool.clone()),
    );

    // let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 3000);
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::debug!("listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

// fn create_app()-> Router{
// fn create_app<T: TodoRepository>(repository:T) -> Router {
fn create_app<Todo: TodoRepository, Label: LabelRepository>(
    todo_repository: Todo,
    label_repository: Label,
) -> Router {
    Router::new()
    .route("/", get(root))
    // .route("/users", post(create_user))
    // .route("/todos", post(create_todo::<T>))
    .route(
        "/todos", 
        post(create_todo::<Todo>)
                .get(all_todo::<Todo>))
    .route(
        "/todos/:id", 
        // get(find_todo::<T>)
        //     .delete(delete_todo::<T>)
        //     .patch(update_todo::<T>),
        get(find_todo::<Todo>)
            .delete(delete_todo::<Todo>)
            .patch(update_todo::<Todo>),
    )

    .route(
        "/labels", 
        post(create_label::<Label>)
            .get(all_label::<Label>),
    )
    .route("/labels/:id", delete(delete_label::<Label>))
    .layer(Extension(Arc::new(todo_repository)))
    .layer(Extension(Arc::new(label_repository)))
    .layer(
        CorsLayer::new()
            .allow_origin(AllowOrigin::exact("http://localhost:3001".parse().unwrap()))
            .allow_methods(Any)
            .allow_headers(vec![CONTENT_TYPE])
    )
}


async fn root() -> &'static str {
    "Hello, world!"
}


#[cfg(test)]
mod test {
    use std::vec;

    use super::*;
    use crate::repositories::{
        label::{
            test_utils::LabelRepositoryForMemory, 
            Label,
        },
        todo::{
            test_utils::TodoRepositoryForMemory, 
            CreateTodo, 
            TodoWithLabelFromRow, TodoEntity, fold_entity, fold_entities
        }
    };

    use axum::response::Response;
    use axum::{
        body::Body, 
        http::{header, Method, Request, StatusCode},
    };
    use tower::ServiceExt;

    fn build_req_with_json(path: &str, method: Method, json_body: String) -> Request<Body>{
        Request::builder()
            .uri(path)
            .method(method)
            .header(header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
            .body(Body::from(json_body))
            .unwrap()
    }

    fn build_req_with_empty(method: Method, path: &str) -> Request<Body>{
        Request::builder()
            .uri(path)
            .method(method)
            .body(Body::empty())
            .unwrap()
    }

    async fn res_to_todo(res: Response) ->TodoWithLabelFromRow {
        let bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
        let body = String::from_utf8(bytes.to_vec()).unwrap();
        let todo = serde_json::from_str(&body)
                .expect(&format!("cannot convert Todo instance, body:{}",body));
        todo
    }

    async fn res_to_label(res: Response) ->Label {
        let bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
        let body = String::from_utf8(bytes.to_vec()).unwrap();
        let label = serde_json::from_str(&body)
                .expect(&format!("cannot convert Label instance, body:{}",body));
        label
    }

    #[tokio::test]
    async fn should_return_hello_world(){
        let todo_repository = TodoRepositoryForMemory::new();
        let label_repository = LabelRepositoryForMemory::new();
        let req = Request::builder().uri("/").body(Body::empty()).unwrap();
        // let res = create_app().oneshot(req).await.unwrap();
        let res = create_app(
            todo_repository,
            label_repository
        ).oneshot(req).await.unwrap();
        
        let bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
        let body = String::from_utf8(bytes.to_vec()).unwrap();
        assert_eq!(body, "Hello, world!");
    }

    #[tokio::test]
    async fn should_created_todo(){
        let expected = TodoEntity::new(1, "should_return_created_todo".to_string());

        let todo_repository = TodoRepositoryForMemory::new();
        let label_repository = LabelRepositoryForMemory::new();

        let req = build_req_with_json(
            "/todos", 
            Method::POST, 
            r#"{ "text": "should_return_created_todo"}"#.to_string(),
        );

        let res = create_app(
            todo_repository,
            label_repository
        ).oneshot(req).await.unwrap();
        
        let todo = res_to_todo(res).await;

        assert_eq!(expected, fold_entity( todo));

    }


    #[tokio::test]
    async fn should_find_todo(){

        let expected = TodoEntity::new(1,"should_find_todo".to_string());

        let labels = vec![];

        let todo_repository = TodoRepositoryForMemory::new();
        let label_repository = LabelRepositoryForMemory::new();

        todo_repository
            .create(CreateTodo::new("should_find_todo".to_string(),labels))
            .await
            .expect("failed create todo");
        let req = build_req_with_empty(Method::GET, "/todos/1");
        let res =create_app(
            todo_repository,
            label_repository
        ).oneshot(req).await.unwrap();
        let todo = res_to_todo(res).await;
        assert_eq!(expected, fold_entity( todo));
    }


    #[tokio::test]
    async fn should_get_all_todos(){

        let expected = TodoEntity::new(1,"should_get_all_todos".to_string());

        let labels = vec![];

        let todo_repository = TodoRepositoryForMemory::new();
        let label_repository = LabelRepositoryForMemory::new();

        todo_repository
            .create(CreateTodo::new("should_get_all_todos".to_string(), labels))
            .await
            .expect("failed create todo");
        let req = build_req_with_empty(Method::GET, "/todos");
        let res = create_app(
            todo_repository,
            label_repository            
        ).oneshot(req).await.unwrap();

        let bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
        let body = String::from_utf8(bytes.to_vec()).unwrap();
        let todo: Vec<TodoWithLabelFromRow> = serde_json::from_str(&body)
                .expect(&format!("cannot convert Todo instance. body: {}", body));
        
        assert_eq!(vec![expected], fold_entities(todo));
    }

    #[tokio::test]
    async fn should_update_todo(){
        let expected = TodoEntity::new(1, "should_update_todo".to_string());

        let labels = vec![];

        let todo_repository = TodoRepositoryForMemory::new();
        let label_repository = LabelRepositoryForMemory::new();

        todo_repository
            .create(CreateTodo::new("before_update_todo".to_string(),labels))
            .await
            .expect("failed create todo");
        let req = build_req_with_json(
            "/todos/1", 
            Method::PATCH, 
            r#"{"text": "should_update_todo","completed": false}"#.to_string(),
        );
        let res = create_app(
            todo_repository,
            label_repository            
        ).oneshot(req).await.unwrap();
        let todo = res_to_todo(res).await;

        assert_eq!(expected, fold_entity( todo));
    }

    #[tokio::test]
    async fn should_delete_todo(){

        let labels = vec![];

        let todo_repository = TodoRepositoryForMemory::new();
        let label_repository = LabelRepositoryForMemory::new();

        todo_repository
            .create(CreateTodo::new("should_delete_todo".to_string(),labels))
            .await
            .expect("failed create todo");
        let req = build_req_with_empty(Method::DELETE, "/todos/1");
        let res = create_app(
            todo_repository,
            label_repository            
        ).oneshot(req).await.unwrap();
        assert_eq!(StatusCode::NO_CONTENT, res.status());

    }


    #[tokio::test]
    async fn should_created_label(){
        let expected = Label::new(1, "should_return_created_label".to_string());

        let todo_repository = TodoRepositoryForMemory::new();
        let label_repository = LabelRepositoryForMemory::new();

        let req = build_req_with_json(
            "/labels", 
            Method::POST, 
            r#"{ "name": "should_return_created_label"}"#.to_string(),
        );
        let res = create_app(
            todo_repository,
            label_repository
        ).oneshot(req).await.unwrap();
        let label = res_to_label(res).await;

        assert_eq!(expected, label);

    }


    #[tokio::test]
    async fn should_get_all_labels(){

        let expected = Label::new(1,"should_get_all_labels".to_string());

        let todo_repository = TodoRepositoryForMemory::new();
        let label_repository = LabelRepositoryForMemory::new();

        label_repository
            .create("should_get_all_labels".to_string())
            .await
            .expect("failed create label");
        let req = build_req_with_empty(Method::GET, "/labels");
        let res = create_app(
            todo_repository,
            label_repository            
        ).oneshot(req).await.unwrap();

        let bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
        let body = String::from_utf8(bytes.to_vec()).unwrap();
        let label: Vec<Label> = serde_json::from_str(&body)
                .expect(&format!("cannot convert Label instance. body: {}", body));
        
        assert_eq!(vec![expected], label);
    }

    #[tokio::test]
    async fn should_delete_label(){
        let todo_repository = TodoRepositoryForMemory::new();
        let label_repository = LabelRepositoryForMemory::new();

        label_repository
            .create("should_delete_label".to_string())
            .await
            .expect("failed create label");
        let req = build_req_with_empty(Method::DELETE, "/labels/1");
        let res = create_app(
            todo_repository,
            label_repository            
        ).oneshot(req).await.unwrap();
        assert_eq!(StatusCode::NO_CONTENT, res.status());

    }


}