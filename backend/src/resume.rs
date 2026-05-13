use anyhow::{anyhow, Result};
use rocket::http::{ContentType, Header, Status};
use rocket::request::Request;
use rocket::response::{self, Responder, Response};
use rocket::{get, State};
use std::io::Cursor;

use crate::AppState;

const TEMPLATE: &str = include_str!("../templates/resume.typ");

pub struct PdfResponse(pub Vec<u8>);

impl<'r> Responder<'r, 'static> for PdfResponse {
    fn respond_to(self, _req: &'r Request<'_>) -> response::Result<'static> {
        let len = self.0.len();
        Response::build()
            .header(ContentType::PDF)
            .header(Header::new(
                "Content-Disposition",
                "attachment; filename=\"joshua-holmes-resume.pdf\"",
            ))
            .sized_body(len, Cursor::new(self.0))
            .ok()
    }
}

#[get("/resume.pdf")]
pub async fn get_resume(state: &State<AppState>) -> Result<PdfResponse, Status> {
    let profile_bytes = tokio::fs::read(&state.profile_path).await.map_err(|e| {
        eprintln!("[resume] cannot read profile: {e}");
        Status::InternalServerError
    })?;
    let profile_json = String::from_utf8(profile_bytes).map_err(|e| {
        eprintln!("[resume] profile not utf-8: {e}");
        Status::InternalServerError
    })?;

    let pdf_bytes = tokio::task::spawn_blocking(move || render_pdf(&profile_json))
        .await
        .map_err(|e| {
            eprintln!("[resume] join error: {e}");
            Status::InternalServerError
        })?
        .map_err(|e| {
            eprintln!("[resume] render error: {e:#}");
            Status::InternalServerError
        })?;

    Ok(PdfResponse(pdf_bytes))
}

fn render_pdf(profile_json: &str) -> Result<Vec<u8>> {
    use typst::foundations::{Dict, IntoValue, Str};
    use typst_as_lib::TypstEngine;

    let engine = TypstEngine::builder()
        .main_file(TEMPLATE)
        .search_fonts_with(typst_as_lib::typst_kit_options::TypstKitFontOptions::default())
        .build();

    let mut inputs = Dict::new();
    inputs.insert(Str::from("profile"), profile_json.into_value());

    let doc = engine
        .compile_with_input(inputs)
        .output
        .map_err(|errs| anyhow!("typst compile errors: {errs:?}"))?;

    let pdf = typst_pdf::pdf(&doc, &typst_pdf::PdfOptions::default())
        .map_err(|errs| anyhow!("typst pdf export errors: {errs:?}"))?;

    Ok(pdf)
}
