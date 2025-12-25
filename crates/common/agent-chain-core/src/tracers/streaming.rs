//! Internal tracers used for stream_log and astream events implementations.
//!
//! This module provides the streaming callback handler trait used for
//! astream_events and astream_log implementations.
//! Mirrors `langchain_core.tracers._streaming`.

use std::pin::Pin;
use uuid::Uuid;

use futures::Stream;

/// A trait for streaming callback handlers.
///
/// This is a common mixin that the callback handlers for both astream events
/// and astream log inherit from.
///
/// The `tap_output_aiter` method is invoked in some contexts to produce
/// callbacks for intermediate results.
pub trait StreamingCallbackHandler<T>: Send + Sync {
    /// Used for internal astream_log and astream events implementations.
    ///
    /// Tap the output async iterator to stream its values.
    ///
    /// # Arguments
    ///
    /// * `run_id` - The ID of the run.
    /// * `output` - The output async iterator to tap.
    ///
    /// # Returns
    ///
    /// An async iterator that yields the same values as the input.
    fn tap_output_aiter(
        &self,
        run_id: Uuid,
        output: Pin<Box<dyn Stream<Item = T> + Send>>,
    ) -> Pin<Box<dyn Stream<Item = T> + Send>>;

    /// Used for internal astream_log and astream events implementations.
    ///
    /// Tap the output iterator to stream its values.
    ///
    /// # Arguments
    ///
    /// * `run_id` - The ID of the run.
    /// * `output` - The output iterator to tap.
    ///
    /// # Returns
    ///
    /// An iterator that yields the same values as the input.
    fn tap_output_iter(
        &self,
        run_id: Uuid,
        output: Box<dyn Iterator<Item = T> + Send>,
    ) -> Box<dyn Iterator<Item = T> + Send>;
}

/// Default implementation that passes through without modification.
pub struct PassthroughStreamingHandler;

impl<T: Send + 'static> StreamingCallbackHandler<T> for PassthroughStreamingHandler {
    fn tap_output_aiter(
        &self,
        _run_id: Uuid,
        output: Pin<Box<dyn Stream<Item = T> + Send>>,
    ) -> Pin<Box<dyn Stream<Item = T> + Send>> {
        output
    }

    fn tap_output_iter(
        &self,
        _run_id: Uuid,
        output: Box<dyn Iterator<Item = T> + Send>,
    ) -> Box<dyn Iterator<Item = T> + Send> {
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use futures::stream;

    #[tokio::test]
    async fn test_passthrough_streaming_handler() {
        let handler = PassthroughStreamingHandler;
        let run_id = Uuid::new_v4();

        let input_stream = stream::iter(vec![1, 2, 3]);
        let boxed_stream: Pin<Box<dyn Stream<Item = i32> + Send>> = Box::pin(input_stream);

        let output_stream = handler.tap_output_aiter(run_id, boxed_stream);
        let result: Vec<i32> = output_stream.collect().await;

        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_passthrough_iter_handler() {
        let handler = PassthroughStreamingHandler;
        let run_id = Uuid::new_v4();

        let input_iter = vec![1, 2, 3].into_iter();
        let boxed_iter: Box<dyn Iterator<Item = i32> + Send> = Box::new(input_iter);

        let output_iter = handler.tap_output_iter(run_id, boxed_iter);
        let result: Vec<i32> = output_iter.collect();

        assert_eq!(result, vec![1, 2, 3]);
    }
}