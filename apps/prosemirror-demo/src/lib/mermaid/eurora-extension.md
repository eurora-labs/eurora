# Eurora Extension Architecture

The Eurora browser extension is designed to capture and process content from various web pages, including YouTube videos, PDF documents, and general articles. It communicates with the native Eurora application through Chrome's native messaging API.

## Class Diagram

```mermaid
classDiagram
    %% Main Components
    class BackgroundScript {
        +initialize()
        +handleInstallation()
    }

    class NativeMessagingWorker {
        -nativePort: chrome.runtime.Port
        -messageQueue: any[]
        +connectToNativeHost(): Promise~boolean~
        +processQueue()
        +sendMessageToNativeHost(payload, tabId)
        +handleGenerateReport()
        +handleGenerateSnapshot()
    }

    %% Content Scripts
    class ContentScript {
        <<interface>>
        +initialize()
        +handleMessages()
    }

    class YouTubeWatcher {
        -videoId: string
        -videoTranscript: any
        -youtubePlayer: HTMLVideoElement
        +getYouTubePlayer(): HTMLVideoElement
        +getCurrentVideoFrame(): EurImage
        +getCurrentVideoTime(): number
        +sendTranscriptToBackground(transcript)
        +getCurrentVideoId()
        +getYouTubeTranscript(videoId): Promise
    }

    class PDFWatcher {
        -pdfViewerApplication: any
        +getPdfState(): Promise~PdfState~
        +getPageContent(application): Promise~string~
    }

    class ArticleWatcher {
        +extractArticleContent(): string
    }

    %% Data Models
    class EurImage {
        +dataBase64: string
        +width: number
        +height: number
        +format: ProtoImageFormat
    }

    class PdfState {
        +type: 'PDF_STATE'
        +url: string
        +title: string
        +content: string
        +selectedText: string
    }

    class ProtoNativeYoutubeState {
        +type: 'YOUTUBE_STATE'
        +url: string
        +title: string
        +transcript: string
        +currentTime: number
        +videoFrameBase64: string
        +videoFrameWidth: number
        +videoFrameHeight: number
        +videoFrameFormat: ProtoImageFormat
    }

    class ProtoNativeYoutubeSnapshot {
        +type: 'YOUTUBE_SNAPSHOT'
        +currentTime: number
        +videoFrameBase64: string
        +videoFrameWidth: number
        +videoFrameHeight: number
        +videoFrameFormat: ProtoImageFormat
    }

    class ProtoNativeArticleAsset {
        +type: 'ARTICLE_ASSET'
        +content: string
        +textContent: string
        +title: string
        +siteName: string
        +language: string
        +excerpt: string
        +length: number
        +selectedText: string
    }

    %% UI Components
    class PopupUI {
        +render()
    }

    %% Relationships
    BackgroundScript --> NativeMessagingWorker: uses
    ContentScript <|-- YouTubeWatcher: implements
    ContentScript <|-- PDFWatcher: implements
    ContentScript <|-- ArticleWatcher: implements

    YouTubeWatcher --> EurImage: creates
    YouTubeWatcher --> ProtoNativeYoutubeState: creates
    YouTubeWatcher --> ProtoNativeYoutubeSnapshot: creates

    PDFWatcher --> PdfState: creates

    ArticleWatcher --> ProtoNativeArticleAsset: creates

    NativeMessagingWorker --> BackgroundScript: communicates with
    NativeMessagingWorker --> YouTubeWatcher: sends messages to
    NativeMessagingWorker --> PDFWatcher: sends messages to
    NativeMessagingWorker --> ArticleWatcher: sends messages to

    BackgroundScript --> PopupUI: initializes
```

## Component Structure

The extension is organized into three main components:

1. **Background Script**: Manages the extension lifecycle and coordinates communication between content scripts and the native application.

2. **Content Scripts**: Specialized scripts that run in the context of web pages to extract content:
    - **YouTube Watcher**: Extracts video information, transcripts, and captures frames from YouTube videos
    - **PDF Watcher**: Extracts content from PDF documents viewed in the browser
    - **Article Watcher**: Uses Mozilla's Readability library to extract clean content from article pages

3. **Popup UI**: Provides a user interface for the extension, showing the Eurora logo and links to the website and GitHub repository.

## Communication Flow

```mermaid
sequenceDiagram
    participant WebPage
    participant ContentScript
    participant BackgroundScript
    participant NativeMessagingWorker
    participant EuroraApp

    WebPage->>ContentScript: User visits page
    ContentScript->>ContentScript: Initialize and monitor page
    BackgroundScript->>NativeMessagingWorker: Connect to native host
    NativeMessagingWorker->>EuroraApp: Establish connection

    BackgroundScript->>ContentScript: Request content (GENERATE_ASSETS)
    ContentScript->>ContentScript: Extract content
    ContentScript->>BackgroundScript: Return content data
    BackgroundScript->>NativeMessagingWorker: Send to native host
    NativeMessagingWorker->>EuroraApp: Forward content data

    EuroraApp->>NativeMessagingWorker: Send response/command
    NativeMessagingWorker->>BackgroundScript: Forward response
    BackgroundScript->>ContentScript: Execute command if needed
```

This architecture allows the extension to seamlessly integrate with the Eurora desktop application, providing a bridge between web content and the AI-powered features of Eurora.
