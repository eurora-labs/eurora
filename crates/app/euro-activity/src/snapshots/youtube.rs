use agent_chain_core::messages::{
    ContentBlock, ContentBlocks, ImageContentBlock, PlainTextContentBlock,
};
use euro_native_messaging::types::NativeYoutubeSnapshot;
use serde::{Deserialize, Serialize};

use crate::{error::ActivityError, types::SnapshotFunctionality};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YoutubeSnapshot {
    pub id: String,
    pub current_time: f32,
    #[serde(skip_serializing)]
    pub video_frame: Option<String>,
    pub video_duration: Option<f32>,
    pub video_title: Option<String>,
    pub video_url: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

impl YoutubeSnapshot {
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

    pub fn try_from(snapshot: NativeYoutubeSnapshot) -> Result<Self, ActivityError> {
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

    pub fn get_progress_percentage(&self) -> Option<f32> {
        self.video_duration.map(|duration| {
            if duration > 0.0 {
                (self.current_time / duration).clamp(0.0, 1.0)
            } else {
                0.0
            }
        })
    }

    pub fn format_current_time(&self) -> String {
        let minutes = (self.current_time / 60.0) as u32;
        let seconds = (self.current_time % 60.0) as u32;
        format!("{:02}:{:02}", minutes, seconds)
    }

    pub fn format_duration(&self) -> Option<String> {
        self.video_duration.map(|duration| {
            let minutes = (duration / 60.0) as u32;
            let seconds = (duration % 60.0) as u32;
            format!("{:02}:{:02}", minutes, seconds)
        })
    }

    pub fn is_near_end(&self) -> bool {
        if let Some(duration) = self.video_duration {
            let remaining = duration - self.current_time;
            remaining <= 30.0 || (remaining / duration) <= 0.1
        } else {
            false
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = chrono::Utc::now().timestamp() as u64;
    }
}

impl SnapshotFunctionality for YoutubeSnapshot {
    fn construct_messages(&self) -> ContentBlocks {
        let snapshot_json = serde_json::to_string(&self).unwrap_or_default();

        let context = match &self.video_title {
            Some(title) => format!("YouTube snapshot for '{}'", title),
            None => "YouTube snapshot".to_string(),
        };

        let snapshot_block = PlainTextContentBlock::builder()
            .context(context)
            .title("youtube_snapshot.json".to_string())
            .mime_type("application/json".to_string())
            .text(snapshot_json)
            .build();

        let mut blocks: Vec<ContentBlock> = vec![snapshot_block.into()];

        if let Some(image) = &self.video_frame
            && !image.is_empty()
        {
            match ImageContentBlock::builder()
                .base64(image.to_string())
                .mime_type("image/png".to_string())
                .build()
            {
                Ok(block) => blocks.push(ContentBlock::Image(block)),
                Err(e) => tracing::warn!("Failed to create image block: {e}"),
            }
        }

        blocks.into()
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
