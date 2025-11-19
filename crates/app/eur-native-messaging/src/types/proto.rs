mod proto_types {
    tonic::include_proto!("browser_bridge");
}

pub use proto_types::*;

// The code below is useful only to avoid super ugly frame type
// to parent frame conversions

impl From<RequestFrame> for Frame {
    fn from(value: RequestFrame) -> Self {
        Frame {
            kind: Some(frame::Kind::Request(value)),
        }
    }
}

impl From<ResponseFrame> for Frame {
    fn from(value: ResponseFrame) -> Self {
        Frame {
            kind: Some(frame::Kind::Response(value)),
        }
    }
}

impl From<EventFrame> for Frame {
    fn from(value: EventFrame) -> Self {
        Frame {
            kind: Some(frame::Kind::Event(value)),
        }
    }
}

impl From<ErrorFrame> for Frame {
    fn from(value: ErrorFrame) -> Self {
        Frame {
            kind: Some(frame::Kind::Error(value)),
        }
    }
}

impl From<CancelFrame> for Frame {
    fn from(value: CancelFrame) -> Self {
        Frame {
            kind: Some(frame::Kind::Cancel(value)),
        }
    }
}
