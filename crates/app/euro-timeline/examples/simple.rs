use euro_timeline::TimelineManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create with sensible defaults
    let mut timeline = TimelineManager::new().await;

    // Start collection (handles focus tracking automatically)
    timeline.start().await?;

    // Get current activity
    if let Some(activity) = timeline.get_current_activity().await {
        println!("Current: {}", activity.name);
    }

    // Stop when done
    timeline.stop().await?;
    Ok(())
}
