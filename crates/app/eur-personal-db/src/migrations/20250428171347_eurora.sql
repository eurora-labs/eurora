-- Create tables for Eurora database

-- Table for tracking each individual activity
CREATE TABLE activity (
    id TEXT PRIMARY KEY,         -- UUID
    name TEXT NOT NULL,          -- Activity name
    app_name TEXT NOT NULL,      -- Application name
    window_name TEXT NOT NULL,   -- Window name
    started_at TEXT NOT NULL,    -- ISO8601 datetime when activity started
    ended_at TEXT                -- ISO8601 datetime when activity ended (nullable)
);

-- Table for references to heavier prompt helpers
CREATE TABLE activity_asset (
    id TEXT PRIMARY KEY,         -- UUID
    activity_id TEXT NOT NULL,   -- Foreign key to activity
    data TEXT NOT NULL,          -- JSON blob stored as text
    created_at TEXT NOT NULL,    -- ISO8601 datetime when asset was created
    updated_at TEXT NOT NULL,    -- ISO8601 datetime when asset was last updated
    FOREIGN KEY (activity_id) REFERENCES activity(id)
);

-- Table for video chunks (recordings)
CREATE TABLE video_chunk (
    id TEXT PRIMARY KEY,         -- UUID
    file_path TEXT NOT NULL      -- Path to the video file
);

-- Table for frames within video chunks
CREATE TABLE frame (
    id TEXT PRIMARY KEY,         -- UUID
    video_chunk_id TEXT NOT NULL,-- Foreign key to video_chunk
    relative_index INTEGER NOT NULL, -- Index of the frame within the video
    FOREIGN KEY (video_chunk_id) REFERENCES video_chunk(id)
);

-- Table for linking activities to frames (snapshots)
CREATE TABLE activity_snapshot (
    id TEXT PRIMARY KEY,         -- UUID
    frame_id TEXT NOT NULL,      -- Foreign key to frame
    activity_id TEXT NOT NULL,   -- Foreign key to activity
    FOREIGN KEY (frame_id) REFERENCES frame(id),
    FOREIGN KEY (activity_id) REFERENCES activity(id)
);

-- Table for text extracted from frames
CREATE TABLE frame_text (
    id TEXT PRIMARY KEY,         -- UUID
    frame_id TEXT NOT NULL,      -- Foreign key to frame
    text TEXT NOT NULL,          -- Extracted text content
    text_json TEXT,              -- JSON representation of text data (nullable)
    ocr_engine TEXT NOT NULL,    -- Name of OCR engine used
    FOREIGN KEY (frame_id) REFERENCES frame(id)
);

-- Create indexes for foreign keys to improve query performance
CREATE INDEX idx_activity_asset_activity_id ON activity_asset(activity_id);
CREATE INDEX idx_activity_snapshot_activity_id ON activity_snapshot(activity_id);
CREATE INDEX idx_activity_snapshot_frame_id ON activity_snapshot(frame_id);
CREATE INDEX idx_frame_video_chunk_id ON frame(video_chunk_id);
CREATE INDEX idx_frame_text_frame_id ON frame_text(frame_id);