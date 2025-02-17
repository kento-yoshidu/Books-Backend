use std::env;
use std::fs;
use std::sync::Mutex;
use actix_web::{get, post, middleware::Logger, web, App, HttpResponse, HttpServer, Responder};
use actix_cors::Cors;
use serde::{Serialize, Deserialize};
use env_logger::Env;
use log::error;
use thiserror::Error;
use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{rand_core::OsRng, SaltString, PasswordHash};
use std::io::Read;

fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    argon2.hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string()
}

#[derive(Serialize, Deserialize, Debug)]
struct User {
    username: String,
    password: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct Book {
    id: u32,
    title: String,
    content: String,
    tags: Vec<String>,
}

#[derive(Deserialize)]
struct BookQuery {
    id: Option<u32>,
    tag: Option<String>,
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

fn write_books_to_file(file_path: &str, books: &Vec<Book>) -> Result<(), BookError> {
    let contents = serde_json::to_string_pretty(books)?;

    fs::write(file_path, contents)?;

    Ok(())
}

#[post("/books")]
async fn add_or_update_book(data: web::Data<Mutex<AppState>>, new_book: web::Json<Book>) -> Result<impl Responder, BookError> {
    let file_path = {
        let state = data.lock().unwrap();
        state.data_file.clone()
    };

    let mut books = read_books_from_file(&file_path)?;

    let existing_book_pos = books.iter_mut().position(|b| b.id == new_book.id);

    match existing_book_pos {
        Some(pos) => {
            books[pos] = new_book.into_inner();
        }
        None => {
            books.push(new_book.into_inner());
        }
    }

    // ファイルに保存
    write_books_to_file(&file_path, &books)?;

    Ok(HttpResponse::Ok().json(books))
}

#[get("/books/search")]
async fn get_book_with_query(
    data: web::Data<Mutex<AppState>>,
    query: web::Query<BookQuery>,
) -> Result<impl Responder, BookError> {
    let file_path = {
        let state = data.lock().unwrap();
        state.data_file.clone()
    };

    let books = read_books_from_file(&file_path)?;

    let filtered_books: Vec<Book> = books.into_iter()
        .filter(|b| {
            (query.id.map_or(true, |id| b.id == id as u32)) &&
            (query.tag.as_deref().map_or(true, |tag| b.tags.contains(&tag.to_string())))
        })
        .collect();

    Ok(HttpResponse::Ok().json(filtered_books))
}

#[get("/books/id/{id}")]
async fn get_book_by_id(data: web::Data::<Mutex<AppState>>, id: web::Path<u32>) -> Result<impl Responder, BookError> {
    let file_path = {
        let state = data.lock().unwrap();
        state.data_file.clone()
    };
    let id = id.into_inner();

    let books = read_books_from_file(&file_path)?;

    let filtered_book: Vec<Book> = books.into_iter()
        .filter(|b| b.id == id)
        .collect();

    Ok(HttpResponse::Ok().json(filtered_book))
}

fn load_users() -> Vec<User> {
    let mut file = match fs::File::open("users.json") {
        Ok(file) => file,
        Err(_) => return Vec::new(),
    };

    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();

    serde_json::from_str(&contents).unwrap_or_else(|_| Vec::new())
}

fn save_user(username: &str, password: &str) {
    let hashed_password = hash_password(password);
    let new_user = User {
        username: username.to_string(),
        password: hashed_password,
    };

    let mut users = load_users();
    users.push(new_user);

    let json = serde_json::to_string_pretty(&users).unwrap();
    fs::write("src/users/users.json", json).expect("Failed to write file");
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("debug"));

    let current_dir = env::current_dir().expect("Failed to get current dir");
    let file_path = current_dir.join("src/data/book.json").to_str().unwrap().to_string();

    let books = web::Data::new(Mutex::new(AppState {
        data_file: file_path,
    }));

    save_user("user1", "password");

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
            .service(get_book_with_query)
            .service(add_or_update_book)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    use actix_web::http::StatusCode;

    fn setup_books() -> web::Data<Mutex<AppState>> {
        let current_dir = env::current_dir().expect("Failed to get current dir");
        let file_path = current_dir.join("src/data/book.json").to_str().unwrap().to_string();

        web::Data::new(Mutex::new(AppState {
            data_file: file_path,
        }))
    }

    #[actix_rt::test]
    async fn test_get_books() {
        let books = setup_books();

        let app = test::init_service(App::new().app_data(books).service(get_books)).await;

        let req = test::TestRequest::get().uri("/books").to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::OK);

        let body = test::read_body(resp).await;
        let body = String::from_utf8_lossy(&body);

        assert!(body.contains("Rust Basics"));
        assert!(body.contains("Async in Rust"));
        assert!(body.contains("Parallelism"));
    }

    #[actix_rt::test]
    async fn test_get_book_by_id() {
        let books = setup_books();

        let app = test::init_service(App::new().app_data(books).service(get_book_by_id)).await;

        let req = test::TestRequest::get().uri("/books/id/1").to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::OK);

        let body = test::read_body(resp).await;
        let body = String::from_utf8_lossy(&body);

        assert!(body.contains("Rust Basics"));

        let req = test::TestRequest::get().uri("/books/id/50").to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::OK);

        let body = test::read_body(resp).await;
        let body = String::from_utf8_lossy(&body);

        assert!(body.contains("Parallelism"));
    }

    #[actix_rt::test]
    async fn test_get_book_not_found() {
        let books = setup_books();

        let app = test::init_service(App::new().app_data(books).service(get_book_by_id)).await;

        let req = test::TestRequest::get().uri("/books/id/999").to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::OK);

        let body: Vec<Book> = test::read_body_json(resp).await;

        assert!(body.is_empty());
    }

    #[actix_rt::test]
    async fn test_get_book_with_query() {
        let books = setup_books();

        let app = test::init_service(App::new().app_data(books).service(get_book_with_query)).await;

        let req = test::TestRequest::get().uri("/books/search?id=1").to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::OK);

        let body = test::read_body(resp).await;
        let body = String::from_utf8_lossy(&body);

        assert!(body.contains("Rust Basics"));
    }
}

// fn verify_password(stored_hash: &str, password: &str) -> bool {
//     let parsed_hash = PasswordHash::new(stored_hash).unwrap();
//     let argon2 = Argon2::default();

//     argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok()
// }
