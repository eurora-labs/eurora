//! YouTube snapshot implementation

use eur_native_messaging::types::NativeYoutubeSnapshot;
use eur_proto::shared::ProtoImageFormat;
use ferrous_llm_core::{ContentPart, ImageSource, Message, MessageContent, Role};
use image::DynamicImage;
use serde::{Deserialize, Serialize};

use crate::{error::ActivityError, types::SnapshotFunctionality};

/// Helper function to safely load images from protocol buffer data
fn load_image_from_proto(
    proto_image: eur_proto::shared::ProtoImage,
) -> Result<DynamicImage, ActivityError> {
    let format = ProtoImageFormat::try_from(proto_image.format)
        .map_err(|_| ActivityError::ProtocolBuffer("Invalid image format".to_string()))?;

    let image = match format {
        ProtoImageFormat::Png => {
            image::load_from_memory_with_format(&proto_image.data, image::ImageFormat::Png)?
        }
        ProtoImageFormat::Jpeg => {
            image::load_from_memory_with_format(&proto_image.data, image::ImageFormat::Jpeg)?
        }
        ProtoImageFormat::Webp => {
            image::load_from_memory_with_format(&proto_image.data, image::ImageFormat::WebP)?
        }
        _ => image::load_from_memory(&proto_image.data)?,
    };

    Ok(image)
}

/// YouTube video snapshot with frame capture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YoutubeSnapshot {
    pub id: String,
    pub video_frame: Option<Vec<u8>>, // Serialized image data
    pub current_time: f32,
    pub video_duration: Option<f32>,
    pub video_title: Option<String>,
    pub video_url: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,

    #[serde(skip)]
    pub video_frame_image: Option<DynamicImage>, // Runtime image, not serialized
}

impl YoutubeSnapshot {
    /// Create a new YouTube snapshot
    pub fn new(
        id: Option<String>,
        video_frame: Option<DynamicImage>,
        current_time: f32,
        video_duration: Option<f32>,
        video_title: Option<String>,
        video_url: Option<String>,
    ) -> Self {
        let now = chrono::Utc::now().timestamp() as u64;

        // Serialize image to bytes if present
        let video_frame_bytes = video_frame.as_ref().and_then(|img| {
            let mut buffer = Vec::new();
            match img.write_to(
                &mut std::io::Cursor::new(&mut buffer),
                image::ImageFormat::Png,
            ) {
                Ok(_) => Some(buffer),
                Err(_) => None,
            }
        });
        let id = id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        Self {
            id,
            video_frame: video_frame_bytes,
            current_time,
            video_duration,
            video_title,
            video_url,
            created_at: now,
            updated_at: now,
            video_frame_image: video_frame,
        }
    }

    /// Try to create from protocol buffer snapshot
    pub fn try_from(snapshot: NativeYoutubeSnapshot) -> Result<Self, ActivityError> {
        // let video_frame_image = if let Some(proto_image) = snapshot.video_frame {
        //     Some(load_image_from_proto(proto_image)?)
        // } else {
        //     None
        // };
        let video_frame_image: Option<DynamicImage> = None;

        let now = chrono::Utc::now().timestamp() as u64;

        // Serialize image to bytes if present
        let video_frame_bytes = video_frame_image.as_ref().and_then(|img| {
            let mut buffer = Vec::new();
            match img.write_to(
                &mut std::io::Cursor::new(&mut buffer),
                image::ImageFormat::Png,
            ) {
                Ok(_) => Some(buffer),
                Err(_) => None,
            }
        });

        Ok(YoutubeSnapshot {
            id: uuid::Uuid::new_v4().to_string(),
            video_frame: video_frame_bytes,
            current_time: snapshot.current_time,
            video_duration: None,
            video_title: None,
            video_url: None,
            created_at: now,
            updated_at: now,
            video_frame_image,
        })
    }

    /// Get the video frame as a DynamicImage
    pub fn get_video_frame(&mut self) -> Option<&DynamicImage> {
        // If we don't have the runtime image but have serialized data, deserialize it
        if self.video_frame_image.is_none() && self.video_frame.is_some() {
            if let Some(bytes) = &self.video_frame {
                if let Ok(img) = image::load_from_memory(bytes) {
                    self.video_frame_image = Some(img);
                }
            }
        }

        self.video_frame_image.as_ref()
    }

    /// Get progress percentage (0.0 to 1.0)
    pub fn get_progress_percentage(&self) -> Option<f32> {
        self.video_duration.map(|duration| {
            if duration > 0.0 {
                (self.current_time / duration).min(1.0).max(0.0)
            } else {
                0.0
            }
        })
    }

    /// Format current time as MM:SS
    pub fn format_current_time(&self) -> String {
        let minutes = (self.current_time / 60.0) as u32;
        let seconds = (self.current_time % 60.0) as u32;
        format!("{:02}:{:02}", minutes, seconds)
    }

    /// Format duration as MM:SS
    pub fn format_duration(&self) -> Option<String> {
        self.video_duration.map(|duration| {
            let minutes = (duration / 60.0) as u32;
            let seconds = (duration % 60.0) as u32;
            format!("{:02}:{:02}", minutes, seconds)
        })
    }

    /// Check if video is near the end (within last 10% or 30 seconds)
    pub fn is_near_end(&self) -> bool {
        if let Some(duration) = self.video_duration {
            let remaining = duration - self.current_time;
            remaining <= 30.0 || (remaining / duration) <= 0.1
        } else {
            false
        }
    }

    /// Update the timestamp
    pub fn touch(&mut self) {
        self.updated_at = chrono::Utc::now().timestamp() as u64;
    }
}

impl SnapshotFunctionality for YoutubeSnapshot {
    /// Construct a message for LLM interaction
    fn construct_messages(&self) -> Vec<Message> {
        let mut content_parts = vec![ContentPart::Text {
            text: format!(
                "This is a frame from a YouTube video at {}s{}{}",
                self.current_time,
                if let Some(title) = &self.video_title {
                    format!(" titled '{}'", title)
                } else {
                    String::new()
                },
                if let Some(duration) = self.video_duration {
                    format!(" (total duration: {}s)", duration)
                } else {
                    String::new()
                }
            ),
        }];

        // Add image if available
        if let Some(image) = &self.video_frame_image {
            content_parts.push(ContentPart::Image {
                image_source: ImageSource::DynamicImage(image.clone()),
                detail: None,
            });
        }

        vec![Message {
            role: Role::User,
            content: MessageContent::Multimodal(content_parts),
        }]
    }

    fn get_updated_at(&self) -> u64 {
        self.updated_at
    }

    fn get_created_at(&self) -> u64 {
        self.created_at
    }

    fn get_id(&self) -> &str {
        &self.id
    }
}

impl From<NativeYoutubeSnapshot> for YoutubeSnapshot {
    fn from(snapshot: NativeYoutubeSnapshot) -> Self {
        Self::try_from(snapshot)
            .expect("Failed to convert NativeYoutubeSnapshot to YoutubeSnapshot")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
