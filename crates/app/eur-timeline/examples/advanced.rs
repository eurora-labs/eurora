use std::time::Duration;

use eur_timeline::{TimelineConfig, TimelineManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Custom configuration
    let config = TimelineConfig::builder()
        .max_activities(500)
        .collection_interval(Duration::from_secs(5))
        .disable_focus_tracking()
        .build();

    let mut timeline = TimelineManager::with_config(config);

    // Start collection
    timeline.start().await?;

    // Stop when done
    timeline.stop().await?;

    Ok(())
}
