use std::fs;
use std::sync::Mutex;
use actix_web::{get, middleware::Logger, web, App, HttpResponse, HttpServer, Responder};
use actix_cors::Cors;
use serde::{Serialize, Deserialize};
use env_logger::Env;
use log::error;
use thiserror::Error;

#[derive(Serialize, Deserialize, Clone)]
struct Book {
    id: u32,
    title: String,
    content: String,
}

struct AppState {
    data_file: String,
}

#[derive(Debug, Error)]
enum BookError {
    #[error("Failed to read JSON file")]
    FileReadError(#[from] std::io::Error),

    #[error("Failed to parse JSON")]
    JsonParseError(#[from] serde_json::Error),
}

impl actix_web::ResponseError for BookError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        match self {
            BookError::FileReadError(_) => HttpResponse::InternalServerError().body("Failed to read JSON"),
            BookError::JsonParseError(_) => HttpResponse::InternalServerError().body("Failed to parse JSON"),
        }
    }
}

fn read_books_from_file(file_path: &str) -> Result<Vec<Book>, BookError> {
    let contents = fs::read_to_string(file_path)?;

    let books: Vec<Book> = serde_json::from_str(&contents)?;

    Ok(books)
}

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[get("/books")]
async fn get_books(data: web::Data<Mutex<AppState>>) -> Result<impl Responder, BookError> {
    let file_path = {
        let state = data.lock().unwrap();
        state.data_file.clone()
    };

    let books = read_books_from_file(&file_path)?;
    Ok(HttpResponse::Ok().json(books))
}

#[get("/books/id/{id}")]
async fn get_book_by_id(data: web::Data::<Mutex<AppState>>, id: web::Path<u32>) -> Result<impl Responder, BookError> {
    let file_path = {
        let state = data.lock().unwrap();
        state.data_file.clone()
    };
    let id = id.into_inner();

    let books = read_books_from_file(&file_path)?;

    if let Some(book) = books.into_iter().find(|b| b.id == id) {
        Ok(HttpResponse::Ok().json(book))
    } else {
        Ok(HttpResponse::NotFound().body("Book not found"))
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
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
