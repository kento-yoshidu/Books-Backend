use std::fs;
use std::sync::Mutex;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use serde::{Serialize, Deserialize};

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
    println!("books");
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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let books = web::Data::new(Mutex::new(AppState {
        data_file: "src/data/book.json".to_string(),
    }));

    HttpServer::new(move || {
        App::new()
            .app_data(books.clone())
            .service(hello)
            .service(get_books)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
