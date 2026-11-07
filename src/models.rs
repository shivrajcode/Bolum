//! Model catalog and verified downloads.
//!
//! Every artifact is pinned by SHA-256 from its Hugging Face LFS object id and
//! re-verified after download, the same guarantee Handy gives for its hosted
//! blobs. Downloads resume from `.part` files.

use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::settings::models_dir;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Engine {
    Parakeet,
    Canary,
}

pub struct FileSpec {
    pub name: &'static str,
    pub url: &'static str,
    pub sha256: &'static str,
    pub size: u64,
}

pub struct CatalogModel {
    pub id: &'static str,
    pub name: &'static str,
    pub engine: Engine,
    pub description: &'static str,
    pub languages: &'static str,
    pub files: &'static [FileSpec],
}

const NEMO_PRE: FileSpec = FileSpec {
    name: "nemo128.onnx",
    url: "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/resolve/main/nemo128.onnx",
    sha256: "a9fde1486ebfcc08f328d75ad4610c67835fea58c73ba57e3209a6f6cf019e9f",
    size: 139_764,
};

pub const CATALOG: &[CatalogModel] = &[
    CatalogModel {
        id: "parakeet-tdt-0.6b-v2",
        name: "NVIDIA Parakeet TDT 0.6B v2",
        engine: Engine::Parakeet,
        description: "Excellent English accuracy and still fast on CPU. ONNX int8.",
        languages: "en",



        files: &[
            FileSpec {
                name: "encoder-model.int8.onnx",
                url: "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v2-onnx/resolve/main/encoder-model.int8.onnx",
                sha256: "3e0581fda6ab843888b51e56d7ee78b6d5bc3237ec113af1f732d1d5286aa155",
                size: 652_184_014,
            },
            FileSpec {
                name: "decoder_joint-model.int8.onnx",
                url: "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v2-onnx/resolve/main/decoder_joint-model.int8.onnx",
                sha256: "a449f49acd68979d418651dd2dcb737cc0f1bf0225e009e29ee326354edbf7d3",
                size: 8_998_286,
            },
            NEMO_PRE,
            FileSpec {
                name: "vocab.txt",
                url: "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v2-onnx/resolve/main/vocab.txt",
                sha256: "ec182b70dd42113aff6c5372c75cac58c952443eb22322f57bbd7f53977d497d",
                size: 9_384,
            },
        ],
    },
    CatalogModel {
        id: "parakeet-tdt-0.6b-v3",
        name: "NVIDIA Parakeet TDT 0.6B v3",
        engine: Engine::Parakeet,
        description: "Fast and accurate across 25 European languages. ONNX int8.",
        languages: "25 languages",



        files: &[
            FileSpec {
                name: "encoder-model.int8.onnx",
                url: "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/resolve/main/encoder-model.int8.onnx",
                sha256: "6139d2fa7e1b086097b277c7149725edbab89cc7c7ae64b23c741be4055aff09",
                size: 652_183_999,
            },
            FileSpec {
                name: "decoder_joint-model.int8.onnx",
                url: "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/resolve/main/decoder_joint-model.int8.onnx",
                sha256: "eea7483ee3d1a30375daedc8ed83e3960c91b098812127a0d99d1c8977667a70",
                size: 18_202_004,
            },
            NEMO_PRE,
            FileSpec {
                name: "vocab.txt",
                url: "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/resolve/main/vocab.txt",
                sha256: "d58544679ea4bc6ac563d1f545eb7d474bd6cfa467f0a6e2c1dc1c7d37e3c35d",
                size: 93_939,
            },
        ],
    },
    CatalogModel {
        id: "canary-180m-flash",
        name: "NVIDIA Canary 180M Flash",
        engine: Engine::Canary,
        description: "Tiny encoder-decoder with English, German, Spanish and French, plus translation between them. ONNX int8.",
        languages: "en, de, es, fr",



        files: &[
            FileSpec {
                name: "encoder-model.int8.onnx",
                url: "https://huggingface.co/istupakov/canary-180m-flash-onnx/resolve/main/encoder-model.int8.onnx",
                sha256: "996d1c89e6cbc891a7c88bf410884c178ffa474f7b13084522ac74a5e144cc81",
                size: 133_710_896,
            },
            FileSpec {
                name: "decoder-model.int8.onnx",
                url: "https://huggingface.co/istupakov/canary-180m-flash-onnx/resolve/main/decoder-model.int8.onnx",
                sha256: "9dd9c447872088c912e916d73751f9621a54085d5bc46788454fe904db51a914",
                size: 79_520_211,
            },
            NEMO_PRE,
            FileSpec {
                name: "vocab.txt",
                url: "https://huggingface.co/istupakov/canary-180m-flash-onnx/resolve/main/vocab.txt",
                sha256: "2dae6fc7815f9640645e0c765522b278ee0cef49b482d91f6913e334628d3e77",
                size: 53_555,
            },
        ],
    },
];

pub fn model_dir(id: &str) -> PathBuf {
    models_dir().join(id)
}

pub fn find(id: &str) -> Result<&'static CatalogModel> {
    CATALOG
        .iter()
        .find(|m| m.id == id)
        .ok_or_else(|| anyhow!("unknown model '{id}'"))
}

pub fn total_size(model: &CatalogModel) -> u64 {
    model.files.iter().map(|f| f.size).sum()
}

pub fn is_downloaded(model: &CatalogModel) -> bool {
    model.files.iter().all(|f| {
        std::fs::metadata(model_dir(model.id).join(f.name))
            .map(|m| m.len() == f.size)
            .unwrap_or(false)
    })
}

#[derive(Default)]
pub struct Downloads {
    cancels: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl Downloads {
    pub fn is_downloading(&self, id: &str) -> bool {
        self.cancels.lock().unwrap().contains_key(id)
    }

    /// Registers a download, or returns `None` when one is already running.
    pub fn start(&self, id: &str) -> Option<Arc<AtomicBool>> {
        let mut cancels = self.cancels.lock().unwrap();
        if cancels.contains_key(id) {
            return None;
        }
        let flag = Arc::new(AtomicBool::new(false));
        cancels.insert(id.to_string(), Arc::clone(&flag));
        Some(flag)
    }

    pub fn finish(&self, id: &str) {
        self.cancels.lock().unwrap().remove(id);
    }

    pub fn cancel(&self, id: &str) {
        if let Some(flag) = self.cancels.lock().unwrap().get(id) {
            flag.store(true, Ordering::Relaxed);
        }
    }
}

/// Downloads every missing file of `model`, verifying SHA-256 before renaming.
/// `on_progress` receives the bytes received so far and the model's total.
pub async fn download(
    model: &'static CatalogModel,
    cancel: Arc<AtomicBool>,
    on_progress: impl Fn(u64, u64),
) -> Result<()> {
    let dir = model_dir(model.id);
    std::fs::create_dir_all(&dir).context("create model directory")?;
    let total: u64 = total_size(model);
    let client = reqwest::Client::builder()
        .user_agent(concat!(
            "bolum/",
            env!("CARGO_PKG_VERSION"),
            " (+https://github.com/shivraj598/Bolum)"
        ))
        .connect_timeout(std::time::Duration::from_secs(20))
        .read_timeout(std::time::Duration::from_secs(60))
        .build()?;

    for file in model.files {
        let final_path = dir.join(file.name);
        if std::fs::metadata(&final_path)
            .map(|m| m.len() == file.size)
            .unwrap_or(false)
        {
            continue;
        }
        let part_path = dir.join(format!("{}.part", file.name));
        let mut already = std::fs::metadata(&part_path).map(|m| m.len()).unwrap_or(0);
        if already >= file.size {
            // A part file that already has the full size only needs verifying.
            let complete = already == file.size
                && sha256_file(&part_path)
                    .map(|sum| sum.eq_ignore_ascii_case(file.sha256))
                    .unwrap_or(false);
            if complete {
                std::fs::rename(&part_path, &final_path)?;
                continue;
            }
            let _ = std::fs::remove_file(&part_path);
            already = 0;
        }

        let mut request = client.get(file.url);
        if already > 0 {
            request = request.header("Range", format!("bytes={already}-"));
        }
        let response = request.send().await?.error_for_status()?;
        if already > 0 && !resumes_at(&response, already) {
            already = 0; // the server ignored the range or started elsewhere
        }
        let mut stream = response.bytes_stream();
        let mut out = std::fs::OpenOptions::new()
            .create(true)
            .append(already > 0)
            .write(true)
            .truncate(already == 0)
            .open(&part_path)?;

        let done_before: u64 = model
            .files
            .iter()
            .take_while(|f| f.name != file.name)
            .map(|f| f.size)
            .sum();
        let mut received = already;
        while let Some(chunk) = stream.next().await {
            if cancel.load(Ordering::Relaxed) {
                return Err(anyhow!("download cancelled"));
            }
            let chunk = chunk?;
            out.write_all(&chunk)?;
            received += chunk.len() as u64;
            on_progress(done_before + received, total);
        }
        out.flush()?;
        drop(out);

        let actual = sha256_file(&part_path)?;
        if !actual.eq_ignore_ascii_case(file.sha256) {
            let _ = std::fs::remove_file(&part_path);
            return Err(anyhow!(
                "checksum mismatch for {}: expected {}, got {}",
                file.name,
                file.sha256,
                actual
            ));
        }
        std::fs::rename(&part_path, &final_path)?;
    }
    Ok(())
}

/// True when a 206 response really continues the `.part` file at `already`.
fn resumes_at(response: &reqwest::Response, already: u64) -> bool {
    if response.status() != reqwest::StatusCode::PARTIAL_CONTENT {
        return false;
    }
    response
        .headers()
        .get(reqwest::header::CONTENT_RANGE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("bytes "))
        .and_then(|value| value.split('-').next())
        .and_then(|value| value.trim().parse::<u64>().ok())
        == Some(already)
}

pub fn sha256_file(path: &PathBuf) -> Result<String> {
    let mut hasher = Sha256::new();
    let mut file = std::fs::File::open(path)?;
    let mut buffer = vec![0u8; 1 << 20];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex_encode(&hasher.finalize()))
}

fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

pub fn delete(model: &CatalogModel) -> Result<()> {
    match std::fs::remove_dir_all(model_dir(model.id)) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn catalog_ids_are_unique() {
        let mut seen = HashSet::new();
        for model in CATALOG {
            assert!(seen.insert(model.id), "duplicate model id {}", model.id);
        }
    }

    #[test]
    fn every_file_is_pinned_to_a_hugging_face_url() {
        for model in CATALOG {
            assert!(!model.files.is_empty(), "{} has no files", model.id);
            for file in model.files {
                assert!(
                    file.sha256.len() == 64
                        && file
                            .sha256
                            .chars()
                            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
                    "{}/{}: sha256 must be 64 lowercase hex digits",
                    model.id,
                    file.name
                );
                assert!(file.size > 0, "{}/{} has no size", model.id, file.name);
                assert!(
                    file.url.starts_with("https://huggingface.co/")
                        && file.url.contains("/resolve/"),
                    "{}/{}: unexpected url {}",
                    model.id,
                    file.name,
                    file.url
                );
            }
        }
    }

    #[test]
    fn each_engine_gets_the_files_it_loads() {
        for model in CATALOG {
            let names: Vec<&str> = model.files.iter().map(|f| f.name).collect();
            let expected: &[&str] = match model.engine {
                Engine::Parakeet => &[
                    "encoder-model.int8.onnx",
                    "decoder_joint-model.int8.onnx",
                    "nemo128.onnx",
                    "vocab.txt",
                ],
                Engine::Canary => &[
                    "encoder-model.int8.onnx",
                    "decoder-model.int8.onnx",
                    "nemo128.onnx",
                    "vocab.txt",
                ],
            };
            assert_eq!(names, expected, "{} ships the wrong files", model.id);
        }
    }

    #[test]
    fn find_rejects_unknown_ids() {
        assert!(find("whisper-enormous").is_err());
    }

    #[test]
    fn total_size_adds_every_file() {
        let model = find("parakeet-tdt-0.6b-v2").unwrap();
        assert_eq!(total_size(model), 652_184_014 + 8_998_286 + 139_764 + 9_384);
    }

    #[test]
    fn sha256_file_matches_a_known_digest() {
        let path = std::env::temp_dir().join(format!("bolum-sha-{}.txt", std::process::id()));
        std::fs::write(&path, b"abc").unwrap();
        let digest = sha256_file(&path);
        let _ = std::fs::remove_file(&path);
        assert_eq!(
            digest.unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    fn response(status: u16, content_range: Option<&str>) -> reqwest::Response {
        let mut builder = http::Response::builder().status(status);
        if let Some(range) = content_range {
            builder = builder.header("content-range", range);
        }
        reqwest::Response::from(builder.body(Vec::<u8>::new()).unwrap())
    }

    #[test]
    fn resumes_only_when_the_server_continues_the_part_file() {
        assert!(resumes_at(&response(206, Some("bytes 100-199/200")), 100));
        // A different offset would splice the wrong bytes onto the part file.
        assert!(!resumes_at(&response(206, Some("bytes 0-199/200")), 100));
        // A server that ignores Range answers 200 with the whole body.
        assert!(!resumes_at(&response(200, None), 100));
        assert!(!resumes_at(&response(206, None), 100));
    }
}
