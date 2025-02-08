use std::fs;
use std::sync::Mutex;
use actix_web::{get, middleware::Logger, web, App, HttpResponse, HttpServer, Responder};
use actix_cors::Cors;
use serde::{Serialize, Deserialize};
use env_logger::Env;
use log::error;

#[derive(Serialize, Deserialize, Clone)]
struct Book {
    id: u32,
    title: String,
    content: String,
}

struct AppState {
    data_file: String,
}

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[get("/books")]
async fn get_books(data: web::Data<Mutex<AppState>>) -> impl Responder {
    let file_path = &data.lock().unwrap().data_file;

    match fs::read_to_string(file_path) {
        Ok(contents) => {
            match serde_json::from_str::<Vec<Book>>(&contents) {
                Ok(books) => HttpResponse::Ok().json(books),
                Err(_) => HttpResponse::InternalServerError().body("Failed to parse JSON"),
            }
        },
        Err(_) => HttpResponse::InternalServerError().body("Failed to read JSON"),
    }
}

#[get("/books/id/{id}")]
async fn get_book_by_id(data: web::Data::<Mutex<AppState>>, id: web::Path<u32>) -> impl Responder {
    let file_path = &data.lock().unwrap().data_file;
    let id = id.into_inner();

    match fs::read_to_string(file_path) {
        Ok(contents) => {
            match serde_json::from_str::<Vec<Book>>(&contents) {
                Ok(books) => {
                    if let Some(book) = books.into_iter().find(|b| b.id == id) {
                        HttpResponse::Ok().json(book)
                    } else {
                        HttpResponse::NotFound().body("Book not found")
                    }
                },
                Err(_) => HttpResponse::InternalServerError().body("Failed to parse JSON"),
            }
        },
        Err(_) => HttpResponse::InternalServerError().body("Failed to read JSON"),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // ロガーの初期化
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let books = web::Data::new(Mutex::new(AppState {
        data_file: "src/data/book.json".to_string(),
    }));

    HttpServer::new(move || {
        App::new()
            .app_data(books.clone())
            .wrap(
                Cors::default()
                    .allowed_origin_fn(|origin, _req_head| {
                        let allowed_origins = vec![
                            "http://localhost:3000",
                            "http://localhost:5173",
                        ];

                        let allowed = allowed_origins
                            .into_iter()
                            .any(|allowed_origin| allowed_origin == origin.to_str().unwrap());

                        if !allowed {
                            error!("CORS violation: Origin {:?} is not allowed", origin);
                        }

                        allowed
                    })
                    .allow_any_method()
                    .allow_any_header()
            )
            .wrap(Logger::default())
            .service(hello)
            .service(get_books)
            .service(get_book_by_id)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
