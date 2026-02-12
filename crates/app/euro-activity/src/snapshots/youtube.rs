//! YouTube snapshot implementation

use agent_chain_core::{BaseMessage, ContentPart, HumanMessage, ImageSource};
use euro_native_messaging::types::NativeYoutubeSnapshot;
use serde::{Deserialize, Serialize};

use crate::{error::ActivityError, types::SnapshotFunctionality};

/// YouTube video snapshot with frame capture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YoutubeSnapshot {
    pub id: String,
    pub current_time: f32,
    pub video_frame: Option<String>, // Runtime image, not serialized
    pub video_duration: Option<f32>,
    pub video_title: Option<String>,
    pub video_url: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

impl YoutubeSnapshot {
    /// Create a new YouTube snapshot
    pub fn new(
        id: Option<String>,
        video_frame: Option<String>,
        current_time: f32,
        video_duration: Option<f32>,
        video_title: Option<String>,
        video_url: Option<String>,
    ) -> Self {
        let now = chrono::Utc::now().timestamp() as u64;
        let id = id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        Self {
            id,
            video_frame,
            current_time,
            video_duration,
            video_title,
            video_url,
            created_at: now,
            updated_at: now,
        }
    }

    /// Try to create from protocol buffer snapshot
    pub fn try_from(snapshot: NativeYoutubeSnapshot) -> Result<Self, ActivityError> {
        // let video_frame_image = if let Some(proto_image) = snapshot.video_frame {
        //     Some(load_image_from_proto(proto_image)?)
        // } else {
        //     None
        // };

        let now = chrono::Utc::now().timestamp() as u64;

        Ok(YoutubeSnapshot {
            id: uuid::Uuid::new_v4().to_string(),
            video_frame: Some(snapshot.video_frame_base64),
            current_time: snapshot.current_time,
            video_duration: None,
            video_title: None,
            video_url: None,
            created_at: now,
            updated_at: now,
        })
    }

    /// Get progress percentage (0.0 to 1.0)
    pub fn get_progress_percentage(&self) -> Option<f32> {
        self.video_duration.map(|duration| {
            if duration > 0.0 {
                (self.current_time / duration).clamp(0.0, 1.0)
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
    fn construct_messages(&self) -> Vec<BaseMessage> {
        let mut content_parts = vec![];

        // Add image if available
        if let Some(image) = &self.video_frame
            && !image.is_empty()
        {
            content_parts.push(ContentPart::Image {
                source: ImageSource::Url {
                    url: format!("data:image/png;base64,{}", image.clone()),
                },
                detail: None,
            });

            vec![
                HumanMessage::builder()
                    .content(content_parts)
                    .build()
                    .into(),
            ]
        } else {
            vec![]
        }
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
