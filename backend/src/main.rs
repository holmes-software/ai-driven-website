#[macro_use]
extern crate rocket;

use std::path::PathBuf;
use std::sync::Arc;

mod cache;
mod cors;
mod llm;
mod profile;
mod resume;
mod styles;

pub struct AppState {
    pub profile_path: PathBuf,
    pub cache: Arc<dyn cache::StyleCache>,
}

#[options("/<_..>")]
fn options() -> rocket::http::Status {
    rocket::http::Status::NoContent
}

#[launch]
async fn rocket() -> _ {
    let base = std::env::current_dir().expect("cwd");
    let images_dir = base.join("data/images");
    let cache_dir = base.join("cache");
    if let Err(e) = std::fs::create_dir_all(&cache_dir) {
        eprintln!("could not create cache dir: {e}");
    }

    let state = AppState {
        profile_path: base.join("data/profile.json"),
        cache: cache::build(cache_dir).await,
    };

    let r = rocket::build()
        .attach(cors::Cors)
        .manage(state)
        .mount(
            "/api",
            routes![
                profile::get_profile,
                resume::get_resume,
                styles::post_styles,
                options,
            ],
        )
        .mount(
            "/api/images",
            rocket::fs::FileServer::from(images_dir).rank(10),
        );

    // In release builds, also serve the bundled SPA from PUBLIC_DIR (default
    // `./public`). In dev we let Vite serve the frontend on its own port.
    #[cfg(not(debug_assertions))]
    let r = {
        let public_dir = std::env::var("PUBLIC_DIR").unwrap_or_else(|_| "./public".to_string());
        r.mount("/", rocket::fs::FileServer::from(public_dir).rank(20))
    };

    r
}
