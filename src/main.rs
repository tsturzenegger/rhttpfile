mod file_id;

use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use clap::Parser;
use core::str;
use file_id::FileId;
use rcgen::{generate_simple_self_signed, CertifiedKey};
use rocket::data::{Limits, ToByteUnit};
use rocket::form::Form;
use rocket::fs::TempFile;
use rocket::http::hyper::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use rocket::http::Status;
use rocket::response::content::RawHtml;
use rocket::response::status::BadRequest;
use rocket::response::{Responder, Result};
use rocket::tokio::fs::File;
use rocket::{config::TlsConfig, launch, routes, Config};
use rocket::{get, post};
use rocket::{uri, FromForm, Request};
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::{env, fs, io};

const ID_LENGTH: usize = 32;

/// Simple http server to upload and download files.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// ip_addr e.g. 127.0.0.1
    #[arg(default_value = "127.0.0.1")]
    addr: String,

    /// ip_addr e.g. 8080
    #[arg(short, long, default_value_t = 8080)]
    port: u16,
}

#[derive(FromForm)]
struct Upload<'f> {
    file: TempFile<'f>,
}

struct ReNamedFile {
    name: PathBuf,
    file: File,
}

impl ReNamedFile {
    async fn new(path: impl AsRef<Path>) -> Option<ReNamedFile> {
        let path = path.as_ref();
        let name = path.file_name()?.into();
        let file = File::open(path).await.ok()?;

        Some(ReNamedFile { name, file })
    }
}

/// Specific Responder for decoding Base64 file and recreating original file name.
impl<'r> Responder<'r, 'static> for ReNamedFile {
    fn respond_to(self, request: &'r Request<'_>) -> Result<'static> {
        let content_type = "application/octet-stream";
        let file_name = self.name.display().to_string();
        let Ok(file_name) = URL_SAFE.decode(&file_name[0..file_name.len() - ID_LENGTH]) else {
            return Err(Status::BadRequest);
        };
        let Ok(file_name) = str::from_utf8(&file_name) else {
            return Err(Status::BadRequest);
        };
        let content_disposition = format!("attachment; filename=\"{}\"", file_name);

        let mut response = self.file.respond_to(request)?;
        response.set_raw_header(CONTENT_TYPE.as_str(), content_type);
        response.set_raw_header(CONTENT_DISPOSITION.as_str(), content_disposition);

        Ok(response)
    }
}

#[post("/", format = "multipart/form-data", data = "<form>")]
async fn upload(mut form: Form<Upload<'_>>) -> core::result::Result<String, BadRequest<String>> {
    let Some(file_name) = form.file.raw_name() else {
        return Err(BadRequest("Invalid file name.".to_string()));
    };
    let id = FileId::new(
        file_name.dangerous_unsafe_unsanitized_raw().as_str(),
        ID_LENGTH,
    );
    form.file
        .persist_to(id.file_path())
        .await
        .map_err(|e| BadRequest(e.to_string()))?;
    Ok(uri!(retrieve(id)).to_string())
}

#[get("/<id>")]
async fn retrieve(id: FileId<'_>) -> Option<ReNamedFile> {
    ReNamedFile::new(id.file_path()).await
}

#[get("/")]
fn index() -> RawHtml<&'static str> {
    RawHtml(include_str!("../index.html"))
}

#[launch]
fn rocket() -> _ {
    let cli = Cli::parse();
    let tls_config = generate_certs().expect("could not generate or load certs");
    let config = Config {
        tls: Some(tls_config),
        limits: Limits::new()
            .limit("file", 1.gibibytes())
            .limit("data-form", 1.gibibytes()),
        address: IpAddr::from_str(&cli.addr).expect("Invalid IP adress"),
        port: cli.port,
        ..Default::default()
    };
    rocket::custom(config).mount("/", routes![index, upload, retrieve])
}

/// Generates TLS Cert if they are not present in ./certs folder.
fn generate_certs() -> core::result::Result<TlsConfig, io::Error> {
    let mut current_exe = env::current_exe()?;
    current_exe.pop();
    let certs_dir = current_exe.join("certs");
    if !certs_dir.join("key.pem").exists() || !certs_dir.join("cert.pem").exists() {
        let CertifiedKey { cert, key_pair } =
            generate_simple_self_signed(vec!["localhost".to_string()])
                .expect("Could not generate self signed cert.");
        fs::create_dir_all(&certs_dir).expect("could not create upload dir.");
        fs::write(certs_dir.join("key.pem"), key_pair.serialize_pem())?;
        fs::write(certs_dir.join("cert.pem"), cert.pem())?;
    }
    Ok(TlsConfig::from_paths(
        certs_dir.join("cert.pem"),
        certs_dir.join("key.pem"),
    ))
}
