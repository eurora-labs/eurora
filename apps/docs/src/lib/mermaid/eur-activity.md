# Activity Strategy Class Diagram

```mermaid
classDiagram
    %% Core Traits
    class ActivityStrategy {
        <<trait>>
        +retrieve_assets() Vec~Box~dyn ActivityAsset~~
        +retrieve_snapshots() Vec~Box~dyn ActivitySnapshot~~
        +gather_state() String
        +get_name() &String
        +get_icon() &String
        +get_process_name() &String
    }
    
    class ActivityAsset {
        <<trait>>
        +get_name() &String
        +get_icon() Option~&String~
        +construct_message() Message
    }
    
    class ActivitySnapshot {
        <<trait>>
        +construct_message() Message
        +get_updated_at() u64
        +get_created_at() u64
    }
    
    %% Strategy Implementations
    class BrowserStrategy {
        -client: Mutex~TauriIpcClient~Channel~~
        -name: String
        -icon: String
        -process_name: String
        +new(name, icon, process_name) Result~Self~
        +get_supported_processes() Vec~&str~
        +get_client() TauriIpcClient~Channel~
    }
    
    class DefaultStrategy {
        +name: String
        +process_name: String
        +icon: String
        +new(name, icon, process_name) Result~Self~
    }
    
    %% Asset Implementations
    class YoutubeAsset {
        +url: String
        +title: String
        +transcript: Vec~TranscriptLine~
        +current_time: f32
        +video_frame: DynamicImage
    }
    
    class ArticleAsset {
        +url: String
        +title: String
        +content: String
    }
    
    %% Snapshot Implementations
    class YoutubeSnapshot {
        +video_frame: DynamicImage
    }
    
    class ArticleSnapshot {
        +highlight: Option~String~
    }
    
    %% Main Activity Class
    class Activity {
        +name: String
        +icon: String
        +process_name: String
        +start: DateTime~Utc~
        +end: Option~DateTime~Utc~~
        +snapshots: Vec~Box~dyn ActivitySnapshot~~
        +assets: Vec~Box~dyn ActivityAsset~~
        +new(name, icon, process_name, assets) Self
        +get_display_assets() Vec~DisplayAsset~
    }
    
    class DisplayAsset {
        +name: String
        +icon: String
        +new(name, icon) Self
    }
    
    %% Browser State
    class BrowserState {
        <<enum>>
        Youtube(ProtoYoutubeState)
        Article(ProtoArticleState)
        Pdf(ProtoPdfState)
        +content_type() String
        +youtube() Option~ProtoYoutubeState~
        +article() Option~ProtoArticleState~
        +pdf() Option~ProtoPdfState~
    }
    
    %% Relationships
    ActivityStrategy <|.. BrowserStrategy : implements
    ActivityStrategy <|.. DefaultStrategy : implements
    ActivityAsset <|.. YoutubeAsset : implements
    ActivityAsset <|.. ArticleAsset : implements
    ActivitySnapshot <|.. YoutubeSnapshot : implements
    ActivitySnapshot <|.. ArticleSnapshot : implements
    Activity o-- ActivityAsset : contains
    Activity o-- ActivitySnapshot : contains
    BrowserStrategy -- BrowserState : uses
```

# Activity Sequence Diagram

```mermaid
sequenceDiagram
    participant Client
    participant StrategySelector as select_strategy_for_process()
    participant BrowserStrategy
    participant DefaultStrategy
    participant NativeMessaging as Native Messaging
    participant Activity
    
    Client->>StrategySelector: select_strategy_for_process(process_name, display_name, icon)
    
    alt is browser process
        StrategySelector->>BrowserStrategy: new(name, icon, process_name)
        BrowserStrategy->>NativeMessaging: create_grpc_ipc_client()
        NativeMessaging-->>BrowserStrategy: TauriIpcClient
        StrategySelector-->>Client: Box<dyn ActivityStrategy>
    else is not browser process
        StrategySelector->>DefaultStrategy: new(name, icon, process_name)
        DefaultStrategy-->>StrategySelector: DefaultStrategy
        StrategySelector-->>Client: Box<dyn ActivityStrategy>
    end
    
    Client->>+BrowserStrategy: retrieve_assets()
    BrowserStrategy->>+NativeMessaging: get_state()
    NativeMessaging-->>-BrowserStrategy: StateResponse
    
    alt YouTube content
        BrowserStrategy->>BrowserStrategy: Create YoutubeAsset
    else Article content
        BrowserStrategy->>BrowserStrategy: Create ArticleAsset
    else PDF content
        BrowserStrategy->>BrowserStrategy: (Not implemented)
    end
    
    BrowserStrategy-->>-Client: Vec<Box<dyn ActivityAsset>>
    
    Client->>+BrowserStrategy: retrieve_snapshots()
    BrowserStrategy->>+NativeMessaging: get_snapshot()
    NativeMessaging-->>-BrowserStrategy: SnapshotResponse
    
    alt YouTube content
        BrowserStrategy->>BrowserStrategy: Create YoutubeSnapshot
    else Article content
        BrowserStrategy->>BrowserStrategy: Create ArticleSnapshot
    end
    
    BrowserStrategy-->>-Client: Vec<Box<dyn ActivitySnapshot>>
    
    Client->>Activity: new(name, icon, process_name, assets)
    Client->>Activity: Add snapshots
    
    Note over Client,Activity: Activity now contains all assets and snapshots
