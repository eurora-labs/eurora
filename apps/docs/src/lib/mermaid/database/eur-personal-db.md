# Eurora SQLite Database Structure

This diagram visualizes the SQLite database schema used by Eurora for storing and managing various data types.

## Entity Relationship Diagram

```mermaid
erDiagram
    activity ||--o{ activity_asset : "has many assets"
    activity ||--o{ activity_snapshot : "has many snapshots"
    frame ||--o{ activity_snapshot : "appears in many snapshots"
    video_chunk ||--o{ frame : "contains many frames"
    frame ||--o{ frame_text : "has many text extractions"

    %% Table for tracking each individual activity. Can be different apps, different tabs or even more granular domain sub url activities (e.g. Youtube ?watch id's)
    activity {
        uuid id PK

        string name
        string app_name
        string window_name
        datetime started_at
        datetime ended_at
    }

    %% Table for references to heavier prompt helpers that don't need to be collected regularly
    activity_asset {
        uuid id PK
        uuid activity_id FK

        JSONB data
        datetime created_at
        datetime updated_at
    }

    activity_snapshot {
        uuid id PK
        uuid frame_id FK
        uuid activity_id FK
    }

    video_chunk {
        uuid id PK

        string file_path
    }

    frame {
        uuid id PK
        uuid video_chunk_id FK

        int relative_index
    }

    frame_text {
        uuid id PK
        uuid frame_id FK

        string text
        string text_json
        string ocr_engine
    }

```
