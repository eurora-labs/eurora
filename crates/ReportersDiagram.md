````mermaid
classDiagram
    BrowserController --> BrowserState : returns
    BrowserState --> YoutubeState : Youtube
    BrowserState --> ArticleState : Article
    BrowserState --> PDFState : PDF

    Timeline --> BrowserController : getState()
    Timeline --> MetricsController : getState()
    Timeline --> ScreenController : getState()
    Timeline --> AudioController : getState()
    Timeline --> Fragment : Fragment

    namespace eur-browser-reporter { 
        class BrowserController {
            +register_handlers()
            +getState() : BrowserState
            -handle_feature_action(NativeMessage): -> Result
            -handle_feature_query(NativeMessage): -> Result
        }
        class BrowserState {
            &lt;&lt;enum>> 
            Youtube(YoutubeState)
            Article(ArticleState)
            Document(DocumentState)
        }
        class YoutubeState {
            +url: String
            +title: String
            +current_time: u64
            +transcript: String
        }


        class ArticleState {
            +url: String
            +title: String
            +html: String
            +highlight: String
        }

        class PDFState {
            +url: String
            +title: String
            +html: String
            +highlight: String
        }
    }

    namespace eur-timeline {
        class Timeline {
            -fragments: Vec&lt;Fragment>
            -capacity: usize
            -interval_seconds: u64

            +new(capacity: usize, interval_seconds: u64)
            +get_all_fragments(): Vec&lt;Fragment>
            +start_collection(): Result&lt;>
        }

        class Fragment {
            +timestamp: u64
            +browser_state: Option&lt;BrowserState>
            +metrics_state: Option&lt;MetricsState>
            +screen_state: Option&lt;ScreenState>
            +audio_state: Option&lt;AudioState>
        }
    }

    namespace eur-metrics-reporter {
        class MetricsController {
            +getState(): MetricsState
        }

        class MetricsState {
            +cpuUsage: f32
            +ramUsage: f32
            +networkUsage: f32
        }
    }

    namespace eur-screen-reporter {
        class ScreenController {
            +getState(): ScreenState
        }

        class ScreenState {
            +monitorCaptures: Vec&lt;ImageBuffer>
        }
    }

    namespace eur-audio-reporter {
        class AudioController {
            +getState(): AudioState
        }

        class AudioState {
            transcription: String
        }
    }

    NativeMessagingServer <-- BrowserController: accesses via instance()
    NativeMessagingServer <-- MainTauri: initializes via init()

    namespace eur-native-messaging {
        class NativeMessagingServer {
            -OnceCell&lt;Arc&lt;Self>> NATIVE_MESSAGING_SERVER 
            -Mutex&lt;HashMap&lt;String, Vec&lt;MessageHandler>>> handlers
            +init() -> Result&lt;Arc&lt;Self>>
            +instance() -> Result&lt;Arc&lt;Self>>
            +subscribe(message_type: &str, handler: MessageHandler)
            +process_message(message: Value) -> Result&lt;Value>
            +start() -> Result&lt;()>
        }   
    }

    MainTauri <-- Timeline: initializes via init()

    namespace eur-tauri {
        class MainTauri {
        }
    }


````