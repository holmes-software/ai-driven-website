use rocket::serde::json::{Json, Value};
use rocket::{get, State};
#[cfg(test)]
use serde::Deserialize;

use crate::AppState;

#[get("/profile")]
pub async fn get_profile(state: &State<AppState>) -> Result<Json<Value>, rocket::http::Status> {
    let bytes = tokio::fs::read(&state.profile_path)
        .await
        .map_err(|_| rocket::http::Status::InternalServerError)?;
    let value: Value =
        serde_json::from_slice(&bytes).map_err(|_| rocket::http::Status::InternalServerError)?;
    Ok(Json(value))
}

/// Strict schema mirror of `data/profile.json`. Used only to validate the file
/// in tests; the runtime endpoint forwards arbitrary JSON unchanged.
#[cfg(test)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // fields are validated via deserialization
pub struct Profile {
    pub about: About,
    pub experience: Vec<ExperienceEntry>,
    pub projects: Vec<Project>,
}

#[cfg(test)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct About {
    pub description: String,
    pub profile_pic: String,
}

#[cfg(test)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
pub struct ExperienceEntry {
    pub title: String,
    pub company: String,
    pub start: String,
    pub end: Option<String>,
    pub achievements: Vec<String>,
}

#[cfg(test)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
pub struct Project {
    pub title: String,
    pub description: String,
    pub image: String,
    pub link: String,
}

#[cfg(test)]
mod tests {
    use super::Profile;
    use std::path::PathBuf;

    fn profile_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/profile.json")
    }

    #[test]
    fn profile_json_is_valid_json() {
        let bytes = std::fs::read(profile_path()).expect("profile.json must exist");
        let _: serde_json::Value =
            serde_json::from_slice(&bytes).expect("profile.json must be valid JSON");
    }

    #[test]
    fn profile_json_matches_schema() {
        let bytes = std::fs::read(profile_path()).expect("profile.json must exist");
        let profile: Profile = serde_json::from_slice(&bytes)
            .expect("profile.json must deserialize into the documented schema");
        assert!(!profile.about.description.is_empty());
        assert!(!profile.about.profile_pic.is_empty());
        assert!(!profile.experience.is_empty(), "expected at least one job");
        for e in &profile.experience {
            assert!(
                !e.achievements.is_empty(),
                "{} missing achievements",
                e.company
            );
        }
        for p in &profile.projects {
            assert!(!p.image.is_empty(), "{} image path is empty", p.title);
            assert!(p.link.starts_with("http"), "{} link must be a URL", p.title);
        }
    }
}
