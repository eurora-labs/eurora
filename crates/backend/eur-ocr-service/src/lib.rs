//! The Eurora OCR service that provides gRPC endpoints for image transcription with JWT authentication.

use anyhow::{Result, anyhow};
use eur_auth::{Claims, JwtConfig, validate_access_token};
use eur_ocr::{self, OcrStrategy};
use eur_proto::proto_ocr_service::proto_ocr_service_server::ProtoOcrService;
use eur_proto::proto_ocr_service::{TranscribeImageRequest, TranscribeImageResponse};
use eur_proto::shared::ProtoImage;
use futures;
use futures::future;
use tonic::{Request, Response, Status};
use tracing::{info, warn};

/// Extract and validate JWT token from request metadata
pub fn authenticate_request<T>(request: &Request<T>, jwt_config: &JwtConfig) -> Result<Claims> {
    // Get authorization header
    let auth_header = request
        .metadata()
        .get("authorization")
        .ok_or_else(|| anyhow!("Missing authorization header"))?;

    // Convert to string
    let auth_str = auth_header
        .to_str()
        .map_err(|_| anyhow!("Invalid authorization header format"))?;

    // Extract Bearer token
    if !auth_str.starts_with("Bearer ") {
        return Err(anyhow!("Authorization header must start with 'Bearer '"));
    }

    let token = &auth_str[7..]; // Remove "Bearer " prefix

    // Validate access token using shared function
    validate_access_token(token, jwt_config)
}

#[derive(Debug)]
pub struct OcrService {
    jwt_config: JwtConfig,
}

impl Default for OcrService {
    fn default() -> Self {
        Self {
            jwt_config: JwtConfig::default(),
        }
    }
}

impl OcrService {
    pub fn new(jwt_config: Option<JwtConfig>) -> Self {
        Self {
            jwt_config: jwt_config.unwrap_or_default(),
        }
    }
}

#[tonic::async_trait]
impl ProtoOcrService for OcrService {
    async fn transcribe_image(
        &self,
        request: Request<TranscribeImageRequest>,
    ) -> Result<Response<TranscribeImageResponse>, Status> {
        eprintln!("Received OCR request");

        // Authenticate the request
        let _claims = match authenticate_request(&request, &self.jwt_config) {
            Ok(claims) => {
                eprintln!("Authenticated OCR request for user: {}", claims.username);
                claims
            }
            Err(e) => {
                warn!("Authentication failed for OCR request: {}", e);
                return Err(Status::unauthenticated(
                    "Invalid or missing authentication token",
                ));
            }
        };

        let request_inner = request.into_inner();

        let tess_strategy = eur_ocr::TesseractOcr::default();

        // Create a vector of futures
        let futures = request_inner
            .images
            .iter()
            .map(|image| async {
                match convert_proto_image_to_dynamic_image(image.clone()).await {
                    Ok(dynamic_image) => {
                        let text = tess_strategy.recognize(&dynamic_image);
                        Ok(text)
                    }
                    Err(e) => {
                        warn!("Image conversion failed: {}", e);
                        Err(format!("Image conversion failed: {}", e))
                    }
                }
            })
            .collect::<Vec<_>>();

        // Await all futures concurrently
        let results = future::join_all(futures).await;

        // Check if any operations failed
        let mut texts = Vec::new();
        for result in results {
            match result {
                Ok(text) => texts.push(text),
                Err(error_msg) => {
                    warn!("OCR operation failed: {}", error_msg);
                    return Err(Status::internal(format!(
                        "OCR operation failed: {}",
                        error_msg
                    )));
                }
            }
        }

        Ok(Response::new(TranscribeImageResponse { texts }))
    }
}

async fn convert_proto_image_to_dynamic_image(image: ProtoImage) -> Result<image::DynamicImage> {
    // Convert ProtoImage to DynamicImage
    let image_data = image.data;
    let width = image.width;
    let height = image.height;

    // Create a DynamicImage from the raw data
    let img = image::RgbImage::from_raw(width as u32, height as u32, image_data)
        .ok_or_else(|| anyhow!("Failed to create DynamicImage"))?;

    Ok(image::DynamicImage::ImageRgb8(img))
}
