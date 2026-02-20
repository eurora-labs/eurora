use std::pin::Pin;
use uuid::Uuid;

use futures::Stream;

pub trait StreamingCallbackHandler<T>: Send + Sync {
    fn tap_output_aiter(
        &self,
        run_id: Uuid,
        output: Pin<Box<dyn Stream<Item = T> + Send>>,
    ) -> Pin<Box<dyn Stream<Item = T> + Send>>;

    fn tap_output_iter(
        &self,
        run_id: Uuid,
        output: Box<dyn Iterator<Item = T> + Send>,
    ) -> Box<dyn Iterator<Item = T> + Send>;
}

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
