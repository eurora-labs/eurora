//! Example demonstrating the extensible activity system
//!
//! This example shows how to:
//! 1. Use the strategy registry system
//! 2. Configure activity collection
//! 3. Add custom strategies (framework for future extensions)
//! 4. Collect activities from different applications

use eur_activity::{
    ActivityConfig, ActivityConfigBuilder, BrowserStrategyFactory, DefaultStrategyFactory,
    PrivacyConfig, ProcessContext, SnapshotFrequency, StrategyConfig, StrategyRegistry,
    get_registry, initialize_registry, select_strategy_for_process,
};
use ferrous_focus::IconData;
use std::time::Duration;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt().init();

    println!("🚀 Extensible Activity System Demo");
    println!("==================================\n");

    // 1. Demonstrate registry initialization
    demonstrate_registry_initialization().await?;

    // 2. Demonstrate configuration system
    demonstrate_configuration_system().await?;

    // 3. Demonstrate strategy selection
    demonstrate_strategy_selection().await?;

    // 4. Demonstrate extensibility for future strategies
    demonstrate_extensibility_framework().await?;

    println!("\n✅ Demo completed successfully!");
    Ok(())
}

async fn demonstrate_registry_initialization() -> Result<(), Box<dyn std::error::Error>> {
    println!("1️⃣  Registry Initialization");
    println!("---------------------------");

    // Initialize the global registry
    let registry = initialize_registry();
    let registry_guard = registry.lock().await;

    // Show registered strategies
    let strategies = registry_guard.get_strategies();
    println!("📋 Registered strategies:");
    for strategy in &strategies {
        println!(
            "   • {} ({}): {}",
            strategy.name, strategy.id, strategy.description
        );
        println!("     Category: {:?}", strategy.category);
        println!(
            "     Supported processes: {:?}",
            strategy.supported_processes
        );
    }

    println!("   Total strategies: {}\n", strategies.len());
    Ok(())
}

async fn demonstrate_configuration_system() -> Result<(), Box<dyn std::error::Error>> {
    println!("2️⃣  Configuration System");
    println!("-------------------------");

    // Create a custom configuration
    let config = ActivityConfigBuilder::new()
        .enable_collection(true)
        .default_collection_interval(Duration::from_secs(5))
        .max_assets_per_activity(15)
        .collect_content(true)
        .anonymize_data(false)
        .ignore_application("sensitive-app".to_string())
        .configure_strategy(
            "browser".to_string(),
            StrategyConfig {
                enabled: true,
                priority: 90,
                collection_interval: Duration::from_secs(3),
                asset_types: vec![
                    "youtube".to_string(),
                    "article".to_string(),
                    "twitter".to_string(),
                ],
                snapshot_frequency: SnapshotFrequency::Interval(Duration::from_secs(10)),
                settings: std::collections::HashMap::new(),
            },
        )
        .build();

    println!("⚙️  Configuration created:");
    println!("   • Collection enabled: {}", config.global.enabled);
    println!(
        "   • Default interval: {:?}",
        config.global.default_collection_interval
    );
    println!(
        "   • Max assets per activity: {}",
        config.global.max_assets_per_activity
    );
    println!(
        "   • Privacy - collect content: {}",
        config.global.privacy.collect_content
    );
    println!(
        "   • Privacy - anonymize data: {}",
        config.global.privacy.anonymize_data
    );

    // Show strategy-specific configuration
    let browser_config = config.get_strategy_config("browser");
    println!(
        "   • Browser strategy priority: {}",
        browser_config.priority
    );
    println!("   • Browser asset types: {:?}", browser_config.asset_types);

    // Validate configuration
    match config.validate() {
        Ok(()) => println!("   ✅ Configuration is valid"),
        Err(e) => println!("   ❌ Configuration error: {}", e),
    }

    println!();
    Ok(())
}

async fn demonstrate_strategy_selection() -> Result<(), Box<dyn std::error::Error>> {
    println!("3️⃣  Strategy Selection");
    println!("----------------------");

    // Test different process types
    let test_processes = vec![
        ("firefox", "Firefox Browser"),
        ("chrome", "Google Chrome"),
        ("brave-browser", "Brave Browser"),
        ("code", "Visual Studio Code"),
        ("slack", "Slack"),
        ("unknown-app", "Unknown Application"),
    ];

    for (process_name, display_name) in test_processes {
        println!("🔍 Testing process: {} ({})", process_name, display_name);

        let context = ProcessContext::new(
            process_name.to_string(),
            display_name.to_string(),
            IconData::default(),
        );

        // Try to select a strategy
        match select_strategy_for_process(
            process_name,
            display_name.to_string(),
            IconData::default(),
        )
        .await
        {
            Ok(strategy) => {
                println!("   ✅ Strategy selected: {}", strategy.get_name());
                println!("      Process: {}", strategy.get_process_name());
            }
            Err(e) => {
                println!("   ❌ No strategy found: {}", e);
            }
        }

        // Show what the registry would select
        let registry = get_registry();
        let mut registry_guard = registry.lock().await;

        match registry_guard.select_strategy(&context).await {
            Ok(strategy) => {
                println!("   📋 Registry selected: {}", strategy.get_name());
            }
            Err(e) => {
                println!("   📋 Registry error: {}", e);
            }
        }

        println!();
    }

    Ok(())
}

async fn demonstrate_extensibility_framework() -> Result<(), Box<dyn std::error::Error>> {
    println!("4️⃣  Extensibility Framework");
    println!("----------------------------");

    println!("🔧 Current architecture supports:");
    println!("   • Dynamic strategy registration");
    println!("   • Plugin-like factory system");
    println!("   • Configurable strategy priorities");
    println!("   • Async strategy creation");
    println!("   • Type-safe process matching");

    println!("\n🚀 Future extensions can add:");
    println!("   • IDE strategies (VS Code, IntelliJ, etc.)");
    println!("   • Communication strategies (Slack, Discord, Teams)");
    println!("   • Productivity strategies (Notion, Obsidian)");
    println!("   • Design strategies (Figma, Adobe Creative Suite)");
    println!("   • Terminal/CLI strategies");
    println!("   • Custom application-specific strategies");

    println!("\n📝 Extension pattern:");
    println!("   1. Implement ActivityStrategy trait");
    println!("   2. Create StrategyFactory implementation");
    println!("   3. Register factory with registry");
    println!("   4. Configure strategy settings");
    println!("   5. Strategy automatically participates in selection");

    // Show how easy it would be to add a new strategy
    println!("\n💡 Example: Adding VS Code strategy");
    println!("   ```rust");
    println!("   pub struct VSCodeStrategyFactory;");
    println!("   ");
    println!("   #[async_trait]");
    println!("   impl StrategyFactory for VSCodeStrategyFactory {{");
    println!(
        "       async fn create_strategy(&self, context: &ProcessContext) -> Result<Box<dyn ActivityStrategy>> {{"
    );
    println!("           Ok(Box::new(VSCodeStrategy::new(context)?))");
    println!("       }}");
    println!("       ");
    println!(
        "       fn supports_process(&self, process_name: &str, _: Option<&str>) -> MatchScore {{"
    );
    println!(
        "           if process_name.contains(\"code\") || process_name.contains(\"vscode\") {{"
    );
    println!("               MatchScore::HIGH");
    println!("           }} else {{");
    println!("               MatchScore::NO_MATCH");
    println!("           }}");
    println!("       }}");
    println!("   }}");
    println!("   ");
    println!("   // Register the factory");
    println!("   registry.register_factory(Arc::new(VSCodeStrategyFactory::new()));");
    println!("   ```");

    Ok(())
}
