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
    /// ip address
    #[arg(default_value = "127.0.0.1")]
    addr: String,

    /// port
    #[arg(default_value_t = 8080)]
    port: u16,

    /// upload limit (mebibytes)
    #[arg(short, long, default_value_t = 1000)]
    upload_limit: usize,

    /// directory with the tls certificates
    #[arg(long, default_value = "certs")]
    certs_dir: String,

    /// file name of key
    #[arg(long, default_value = "key.pem")]
    key_file_name: String,

    /// file name of cert
    #[arg(long, default_value = "cert.pem")]
    cert_file_name: String,

    /// self signed cert subject alt name
    #[arg(long, default_value = "localhost")]
    subject_alt_name: String,
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
    let tls_config = generate_certs(&cli).expect("could not generate or load certs");
    let config = Config {
        tls: Some(tls_config),
        limits: Limits::new()
            .limit("file", cli.upload_limit.mebibytes())
            .limit("data-form", cli.upload_limit.mebibytes()),
        address: IpAddr::from_str(&cli.addr).expect("Invalid IP adress"),
        port: cli.port,
        ..Default::default()
    };
    rocket::custom(config).mount("/", routes![index, upload, retrieve])
}

/// Generates TLS Cert if they are not present in ./certs folder.
fn generate_certs(cli: &Cli) -> core::result::Result<TlsConfig, io::Error> {
    let mut current_exe = env::current_exe()?;
    current_exe.pop();
    let certs_dir = current_exe.join(&cli.certs_dir);
    if !certs_dir.join(&cli.key_file_name).exists() || !certs_dir.join(&cli.cert_file_name).exists()
    {
        let CertifiedKey { cert, key_pair } =
            generate_simple_self_signed(vec![cli.subject_alt_name.clone()])
                .expect("Could not generate self signed cert.");
        fs::create_dir_all(&certs_dir).expect("could not create upload dir.");
        fs::write(certs_dir.join(&cli.key_file_name), key_pair.serialize_pem())?;
        fs::write(certs_dir.join(&cli.cert_file_name), cert.pem())?;
    }
    Ok(TlsConfig::from_paths(
        certs_dir.join(&cli.cert_file_name),
        certs_dir.join(&cli.key_file_name),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rocket::http::Status;
    use rocket::{http::ContentType, local::asynchronous::Client};

    // Launch the rocket instance for testing
    fn rocket() -> rocket::Rocket<rocket::Build> {
        rocket::build().mount("/", routes![upload, index, retrieve])
    }

    #[rocket::async_test]
    async fn hello_world() {
        let client = Client::tracked(rocket())
            .await
            .expect("valid rocket instance");
        let response = client.get(uri!(super::index)).dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        assert!(response.into_string().await.unwrap().contains("USAGE"));
    }

    async fn file_upload(file_name: &str) -> String {
        let client = Client::tracked(rocket())
            .await
            .expect("valid rocket instance");
        let form_data = format!(
            "--boundary\r\n\
             Content-Disposition: form-data; name=\"file\"; filename=\"{file_name}\"\r\n\
             Content-Type: text/plain\r\n\r\n
             This is a test file.\r\n\
             --boundary--\r\n"
        );
        let response = client
            .post("/")
            .header(
                ContentType::new("multipart", "form-data").with_params(("boundary", "boundary")),
            )
            .body(form_data)
            .dispatch()
            .await;
        let body = response.into_string().await.unwrap();
        body
    }
    #[rocket::async_test]
    async fn test_file_upload() {
        let body = file_upload("test_file.txt").await;
        assert!(body.contains("/dGVzdF9maWxlLnR4dA")); //base64 of test_file.txt
    }

    #[rocket::async_test]
    async fn test_file_download() {
        let link = file_upload("test_file.txt").await;
        let client = Client::tracked(rocket())
            .await
            .expect("valid rocket instance");
        let response = client.get(link).dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.unwrap();
        assert!(body.contains("This is a test file."));
    }

    #[rocket::async_test]
    async fn test_path_traversal() {
        let link = file_upload("../../../../../../../../../../../etc/passwd").await;
        let client = Client::tracked(rocket())
            .await
            .expect("valid rocket instance");
        let response = client.get(link).dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.unwrap();
        assert!(body.contains("This is a test file."));
    }

    #[rocket::async_test]
    async fn test_special_chars() {
        let link = file_upload("a/\\b/some/.*file<.txt.zip").await;
        let client = Client::tracked(rocket())
            .await
            .expect("valid rocket instance");
        let response = client.get(link).dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.unwrap();
        assert!(body.contains("This is a test file."));
    }

    #[rocket::async_test]
    async fn test_long_filename() {
        let long_file_name = (0..100000)
            .map(|n| n.to_string())
            .collect::<Vec<String>>()
            .join(", ");
        let link = file_upload(&long_file_name).await;
        let client = Client::tracked(rocket())
            .await
            .expect("valid rocket instance");
        let response = client.get(link).dispatch().await;
        assert_eq!(response.status(), Status::BadRequest);
    }

    #[test]
    fn test_cli() {
        let cli = Cli::parse();
        assert_eq!(cli.port, 8080);
    }
}
