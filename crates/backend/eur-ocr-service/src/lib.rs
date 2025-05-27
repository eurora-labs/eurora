//! The Eurora monolith server that hosts the gRPC service for questions.

use anyhow::{Result, anyhow};
use dotenv::dotenv;
use eur_ocr::{self, OcrStrategy};
use eur_proto::proto_ocr_service::proto_ocr_service_server::{
    ProtoOcrService, ProtoOcrServiceServer,
};
use eur_proto::proto_ocr_service::{TranscribeImageRequest, TranscribeImageResponse};
use eur_proto::shared::ProtoImage;
use futures;
use futures::future;
use std::env;
use tonic::{Request, Response, Status, transport::Server};
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

#[derive(Default, Debug)]
pub struct OcrService {}

#[tonic::async_trait]
impl ProtoOcrService for OcrService {
    async fn transcribe_image(
        &self,
        request: Request<TranscribeImageRequest>,
    ) -> Result<Response<TranscribeImageResponse>, Status> {
        info!("Received ocr request");

        let request_inner = request.into_inner();

        let tess_strategy = eur_ocr::TesseractOcr::default();

        // Create a vector of futures
        let futures = request_inner
            .images
            .iter()
            .map(|image| async {
                let dynamic_image = convert_proto_image_to_dynamic_image(image.clone())
                    .await
                    .unwrap();

                tess_strategy.recognize(&dynamic_image)
            })
            .collect::<Vec<_>>();

        // Await all futures concurrently
        let strings = future::join_all(futures).await;

        Ok(Response::new(TranscribeImageResponse { texts: strings }))
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
