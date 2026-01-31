//! Re-export proto types for the browser bridge
//!
//! This module now only re-exports types since euro-native-messaging
//! acts as a client connecting to the euro-activity gRPC server.

pub use crate::types::proto::{
    EventFrame, Frame, RegisterFrame, RequestFrame, ResponseFrame,
    browser_bridge_client::BrowserBridgeClient, frame::Kind as FrameKind,
};
