use std::fmt::Debug;
use std::sync::Arc;

use async_trait::async_trait;
use futures::StreamExt;
use futures::stream::BoxStream;

use bon::bon;

use crate::error::{Error, Result};

use super::base::{DynRunnable, Runnable};
use super::config::{
    ConfigOrList, RunnableConfig, ensure_config, get_callback_manager_for_config, get_config_list,
    patch_config,
};

pub type FallbackErrorPredicate = Arc<dyn Fn(&Error) -> bool + Send + Sync>;

pub type ExceptionInserter<I> = Arc<dyn Fn(&I, &str, &Error) -> I + Send + Sync>;

pub struct RunnableWithFallbacks<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    pub runnable: DynRunnable<I, O>,
    pub fallbacks: Vec<DynRunnable<I, O>>,
    pub error_predicate: Option<FallbackErrorPredicate>,
    pub exception_key: Option<String>,
    exception_inserter: Option<ExceptionInserter<I>>,
    name: Option<String>,
}

impl<I, O> Debug for RunnableWithFallbacks<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunnableWithFallbacks")
            .field("runnable", &"<runnable>")
            .field("fallbacks_count", &self.fallbacks.len())
            .field(
                "error_predicate",
                &self.error_predicate.as_ref().map(|_| "..."),
            )
            .field("exception_key", &self.exception_key)
            .field("name", &self.name)
            .finish()
    }
}

#[bon]
impl<I, O> RunnableWithFallbacks<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    pub fn new<R>(runnable: R, fallbacks: Vec<DynRunnable<I, O>>) -> Self
    where
        R: Runnable<Input = I, Output = O> + Send + Sync + 'static,
    {
        Self::from_dyn()
            .runnable(Arc::new(runnable) as DynRunnable<I, O>)
            .fallbacks(fallbacks)
            .call()
    }

    #[builder]
    pub fn from_dyn(
        runnable: DynRunnable<I, O>,
        fallbacks: Vec<DynRunnable<I, O>>,
        error_predicate: Option<FallbackErrorPredicate>,
        exception_key: Option<String>,
        exception_inserter: Option<ExceptionInserter<I>>,
        name: Option<String>,
    ) -> Self {
        Self {
            runnable,
            fallbacks,
            error_predicate,
            exception_key,
            exception_inserter,
            name,
        }
    }

    pub fn runnables(&self) -> impl Iterator<Item = &DynRunnable<I, O>> {
        std::iter::once(&self.runnable).chain(self.fallbacks.iter())
    }

    fn should_fallback(&self, error: &Error) -> bool {
        match &self.error_predicate {
            Some(predicate) => predicate(error),
            None => true,
        }
    }
}

#[async_trait]
impl<I, O> Runnable for RunnableWithFallbacks<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    type Input = I;
    type Output = O;

    fn name(&self) -> Option<String> {
        self.name.clone()
    }

    fn get_input_schema(&self, config: Option<&RunnableConfig>) -> serde_json::Value {
        self.runnable.get_input_schema(config)
    }

    fn get_output_schema(&self, config: Option<&RunnableConfig>) -> serde_json::Value {
        self.runnable.get_output_schema(config)
    }

    fn invoke(&self, input: Self::Input, config: Option<RunnableConfig>) -> Result<Self::Output> {
        let config = ensure_config(config);
        let callback_manager = get_callback_manager_for_config(&config);

        let run_manager = callback_manager
            .on_chain_start()
            .serialized(&std::collections::HashMap::new())
            .inputs(&std::collections::HashMap::new())
            .maybe_run_id(config.run_id)
            .call();

        let mut first_error: Option<Error> = None;
        let mut last_error: Option<Error> = None;
        let mut current_input = input;

        for runnable in self.runnables() {
            if let (Some(key), Some(inserter), Some(err)) =
                (&self.exception_key, &self.exception_inserter, &last_error)
            {
                current_input = inserter(&current_input, key, err);
            }

            let child_config = patch_config(
                Some(config.clone()),
                Some(run_manager.get_child(None)),
                None,
                None,
                None,
                None,
            );

            match runnable.invoke(current_input.clone(), Some(child_config)) {
                Ok(output) => {
                    run_manager.on_chain_end(&std::collections::HashMap::new());
                    return Ok(output);
                }
                Err(e) => {
                    if self.should_fallback(&e) {
                        if first_error.is_none() {
                            first_error = Some(Error::other(e.to_string()));
                        }
                        last_error = Some(e);
                    } else {
                        run_manager.on_chain_error(&e);
                        return Err(e);
                    }
                }
            }
        }

        let error =
            first_error.unwrap_or_else(|| Error::other("No error stored at end of fallbacks."));
        run_manager.on_chain_error(&error);
        Err(error)
    }

    async fn ainvoke(
        &self,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> Result<Self::Output>
    where
        Self: 'static,
    {
        let config = ensure_config(config);

        let mut first_error: Option<Error> = None;
        let mut last_error: Option<Error> = None;
        let mut current_input = input;

        for runnable in self.runnables() {
            if let (Some(key), Some(inserter), Some(err)) =
                (&self.exception_key, &self.exception_inserter, &last_error)
            {
                current_input = inserter(&current_input, key, err);
            }

            match runnable
                .ainvoke(current_input.clone(), Some(config.clone()))
                .await
            {
                Ok(output) => {
                    return Ok(output);
                }
                Err(e) => {
                    if self.should_fallback(&e) {
                        if first_error.is_none() {
                            first_error = Some(Error::other(e.to_string()));
                        }
                        last_error = Some(e);
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        Err(first_error.unwrap_or_else(|| Error::other("No error stored at end of fallbacks.")))
    }

    fn batch(
        &self,
        inputs: Vec<Self::Input>,
        config: Option<ConfigOrList>,
        return_exceptions: bool,
    ) -> Vec<Result<Self::Output>>
    where
        Self: 'static,
    {
        if inputs.is_empty() {
            return Vec::new();
        }

        let configs = match get_config_list(config, inputs.len()) {
            Ok(c) => c,
            Err(e) => return vec![Err(e)],
        };
        let n = inputs.len();

        let mut to_return: Vec<Option<Result<Self::Output>>> = (0..n).map(|_| None).collect();
        let mut run_again: Vec<(usize, Self::Input)> = inputs.into_iter().enumerate().collect();
        let mut handled_exception_indices: Vec<usize> = Vec::new();
        let mut first_to_raise: Option<Error> = None;

        for runnable in self.runnables() {
            if run_again.is_empty() {
                break;
            }

            let batch_inputs: Vec<Self::Input> =
                run_again.iter().map(|(_, inp)| inp.clone()).collect();
            let batch_configs: Vec<RunnableConfig> =
                run_again.iter().map(|(i, _)| configs[*i].clone()).collect();

            let outputs = runnable.batch(
                batch_inputs,
                Some(ConfigOrList::List(batch_configs)),
                true, // Always return exceptions to handle them ourselves
            );

            let mut next_run_again = Vec::new();

            for ((i, input), output) in run_again.iter().zip(outputs) {
                match output {
                    Ok(out) => {
                        to_return[*i] = Some(Ok(out));
                        handled_exception_indices.retain(|&idx| idx != *i);
                    }
                    Err(e) => {
                        if self.should_fallback(&e) {
                            if !handled_exception_indices.contains(i) {
                                handled_exception_indices.push(*i);
                            }
                            let next_input = if let (Some(key), Some(inserter)) =
                                (&self.exception_key, &self.exception_inserter)
                            {
                                inserter(input, key, &e)
                            } else {
                                input.clone()
                            };
                            to_return[*i] = Some(Err(e));
                            next_run_again.push((*i, next_input));
                        } else if return_exceptions {
                            to_return[*i] = Some(Err(e));
                        } else if first_to_raise.is_none() {
                            first_to_raise = Some(e);
                        }
                    }
                }
            }

            if first_to_raise.is_some() {
                let mut results = Vec::with_capacity(to_return.len());
                let mut error_consumed = false;
                for opt in to_return {
                    match opt {
                        Some(result) => results.push(result),
                        None => {
                            if !error_consumed {
                                results.push(Err(first_to_raise
                                    .take()
                                    .expect("first_to_raise set when errors exist")));
                                error_consumed = true;
                            } else {
                                results.push(Err(Error::other("Batch aborted due to error")));
                            }
                        }
                    }
                }
                return results;
            }

            run_again = next_run_again;
        }

        to_return
            .into_iter()
            .map(|opt| opt.unwrap_or_else(|| Err(Error::other("No result for index"))))
            .collect()
    }

    async fn abatch(
        &self,
        inputs: Vec<Self::Input>,
        config: Option<ConfigOrList>,
        return_exceptions: bool,
    ) -> Vec<Result<Self::Output>>
    where
        Self: 'static,
    {
        if inputs.is_empty() {
            return Vec::new();
        }

        let configs = match get_config_list(config, inputs.len()) {
            Ok(c) => c,
            Err(e) => return vec![Err(e)],
        };
        let n = inputs.len();

        let mut to_return: Vec<Option<Result<Self::Output>>> = (0..n).map(|_| None).collect();
        let mut run_again: Vec<(usize, Self::Input)> = inputs.into_iter().enumerate().collect();
        let mut handled_exception_indices: Vec<usize> = Vec::new();
        let mut first_to_raise: Option<Error> = None;

        for runnable in self.runnables() {
            if run_again.is_empty() {
                break;
            }

            let batch_inputs: Vec<Self::Input> =
                run_again.iter().map(|(_, inp)| inp.clone()).collect();
            let batch_configs: Vec<RunnableConfig> =
                run_again.iter().map(|(i, _)| configs[*i].clone()).collect();

            let outputs = runnable
                .abatch(
                    batch_inputs,
                    Some(ConfigOrList::List(batch_configs)),
                    true, // Always return exceptions to handle them ourselves
                )
                .await;

            let mut next_run_again = Vec::new();

            for ((i, input), output) in run_again.iter().zip(outputs) {
                match output {
                    Ok(out) => {
                        to_return[*i] = Some(Ok(out));
                        handled_exception_indices.retain(|&idx| idx != *i);
                    }
                    Err(e) => {
                        if self.should_fallback(&e) {
                            if !handled_exception_indices.contains(i) {
                                handled_exception_indices.push(*i);
                            }
                            let next_input = if let (Some(key), Some(inserter)) =
                                (&self.exception_key, &self.exception_inserter)
                            {
                                inserter(input, key, &e)
                            } else {
                                input.clone()
                            };
                            to_return[*i] = Some(Err(e));
                            next_run_again.push((*i, next_input));
                        } else if return_exceptions {
                            to_return[*i] = Some(Err(e));
                        } else if first_to_raise.is_none() {
                            first_to_raise = Some(e);
                        }
                    }
                }
            }

            if first_to_raise.is_some() {
                let mut results = Vec::with_capacity(to_return.len());
                let mut error_consumed = false;
                for opt in to_return {
                    match opt {
                        Some(result) => results.push(result),
                        None => {
                            if !error_consumed {
                                results.push(Err(first_to_raise
                                    .take()
                                    .expect("first_to_raise set when errors exist")));
                                error_consumed = true;
                            } else {
                                results.push(Err(Error::other("Batch aborted due to error")));
                            }
                        }
                    }
                }
                return results;
            }

            run_again = next_run_again;
        }

        to_return
            .into_iter()
            .map(|opt| opt.unwrap_or_else(|| Err(Error::other("No result for index"))))
            .collect()
    }

    fn stream(
        &self,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> BoxStream<'_, Result<Self::Output>> {
        let config = ensure_config(config);

        Box::pin(async_stream::stream! {
            let mut first_error: Option<Error> = None;
            let mut last_error: Option<Error> = None;
            let mut current_input = input;

            for runnable in self.runnables() {
                if let (Some(key), Some(inserter), Some(err)) =
                    (&self.exception_key, &self.exception_inserter, &last_error)
                {
                    current_input = inserter(&current_input, key, err);
                }

                let mut stream = runnable.stream(current_input.clone(), Some(config.clone()));

                match stream.next().await {
                    Some(Ok(chunk)) => {
                        yield Ok(chunk);

                        while let Some(result) = stream.next().await {
                            yield result;
                        }
                        return;
                    }
                    Some(Err(e)) => {
                        if self.should_fallback(&e) {
                            if first_error.is_none() {
                                first_error = Some(Error::other(e.to_string()));
                            }
                            last_error = Some(e);
                        } else {
                            yield Err(e);
                            return;
                        }
                    }
                    None => {
                        if first_error.is_none() {
                            first_error = Some(Error::other("Empty stream from runnable"));
                        }
                    }
                }
            }

            yield Err(first_error.unwrap_or_else(|| Error::other("No error stored at end of fallbacks.")));
        })
    }

    fn astream(
        &self,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> BoxStream<'_, Result<Self::Output>>
    where
        Self: 'static,
    {
        let config = ensure_config(config);

        Box::pin(async_stream::stream! {
            let mut first_error: Option<Error> = None;
            let mut last_error: Option<Error> = None;
            let mut current_input = input;

            for runnable in self.runnables() {
                if let (Some(key), Some(inserter), Some(err)) =
                    (&self.exception_key, &self.exception_inserter, &last_error)
                {
                    current_input = inserter(&current_input, key, err);
                }

                let mut stream = runnable.astream(current_input.clone(), Some(config.clone()));

                match stream.next().await {
                    Some(Ok(chunk)) => {
                        yield Ok(chunk);

                        while let Some(result) = stream.next().await {
                            yield result;
                        }
                        return;
                    }
                    Some(Err(e)) => {
                        if self.should_fallback(&e) {
                            if first_error.is_none() {
                                first_error = Some(Error::other(e.to_string()));
                            }
                            last_error = Some(e);
                        } else {
                            yield Err(e);
                            return;
                        }
                    }
                    None => {
                        if first_error.is_none() {
                            first_error = Some(Error::other("Empty stream from runnable"));
                        }
                    }
                }
            }

            yield Err(first_error.unwrap_or_else(|| Error::other("No error stored at end of fallbacks.")));
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runnables::base::RunnableLambda;

    #[test]
    fn test_fallback_on_error() {
        let primary =
            RunnableLambda::builder().func(|_x: i32| -> Result<i32> { Err(Error::other("primary failed")) }).build();

        let fallback = RunnableLambda::builder().func(|x: i32| -> Result<i32> { Ok(x * 2) }).build();

        let with_fallbacks = RunnableWithFallbacks::new(primary, vec![Arc::new(fallback)]);

        let result = with_fallbacks.invoke(5, None).unwrap();
        assert_eq!(result, 10);
    }

    #[test]
    fn test_primary_succeeds() {
        let primary = RunnableLambda::builder().func(|x: i32| -> Result<i32> { Ok(x + 1) }).build();

        let fallback = RunnableLambda::builder().func(|x: i32| -> Result<i32> { Ok(x * 2) }).build();

        let with_fallbacks = RunnableWithFallbacks::new(primary, vec![Arc::new(fallback)]);

        let result = with_fallbacks.invoke(5, None).unwrap();
        assert_eq!(result, 6); // Primary succeeded, not fallback
    }

    #[test]
    fn test_all_fail() {
        let primary =
            RunnableLambda::builder().func(|_x: i32| -> Result<i32> { Err(Error::other("primary failed")) }).build();

        let fallback =
            RunnableLambda::builder().func(|_x: i32| -> Result<i32> { Err(Error::other("fallback failed")) }).build();

        let with_fallbacks = RunnableWithFallbacks::new(primary, vec![Arc::new(fallback)]);

        let result = with_fallbacks.invoke(5, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_fallbacks() {
        let primary =
            RunnableLambda::builder().func(|_x: i32| -> Result<i32> { Err(Error::other("primary failed")) }).build();

        let fallback1 =
            RunnableLambda::builder().func(|_x: i32| -> Result<i32> { Err(Error::other("fallback1 failed")) }).build();

        let fallback2 = RunnableLambda::builder().func(|x: i32| -> Result<i32> { Ok(x * 3) }).build();

        let with_fallbacks =
            RunnableWithFallbacks::new(primary, vec![Arc::new(fallback1), Arc::new(fallback2)]);

        let result = with_fallbacks.invoke(5, None).unwrap();
        assert_eq!(result, 15); // Second fallback succeeded
    }

    #[test]
    fn test_with_fallbacks_ext() {
        let primary =
            RunnableLambda::builder().func(|_x: i32| -> Result<i32> { Err(Error::other("primary failed")) }).build();

        let fallback = RunnableLambda::builder().func(|x: i32| -> Result<i32> { Ok(x * 2) }).build();

        let with_fallbacks = primary.with_fallbacks(vec![Arc::new(fallback)]);

        let result = with_fallbacks.invoke(5, None).unwrap();
        assert_eq!(result, 10);
    }

    #[tokio::test]
    async fn test_fallback_async() {
        let primary =
            RunnableLambda::builder().func(|_x: i32| -> Result<i32> { Err(Error::other("primary failed")) }).build();

        let fallback = RunnableLambda::builder().func(|x: i32| -> Result<i32> { Ok(x * 2) }).build();

        let with_fallbacks = RunnableWithFallbacks::new(primary, vec![Arc::new(fallback)]);

        let result = with_fallbacks.ainvoke(5, None).await.unwrap();
        assert_eq!(result, 10);
    }

    #[test]
    fn test_batch_fallback() {
        let primary = RunnableLambda::builder().func(|x: i32| -> Result<i32> {
            if x > 5 {
                Err(Error::other("too large"))
            } else {
                Ok(x + 1)
            }
        }).build();

        let fallback = RunnableLambda::builder().func(|x: i32| -> Result<i32> { Ok(x * 2) }).build();

        let with_fallbacks = RunnableWithFallbacks::new(primary, vec![Arc::new(fallback)]);

        let results = with_fallbacks.batch(vec![3, 10, 5], None, false);

        assert_eq!(results[0].as_ref().unwrap(), &4);
        assert_eq!(results[1].as_ref().unwrap(), &20);
        assert_eq!(results[2].as_ref().unwrap(), &6);
    }

    #[tokio::test]
    async fn test_stream_fallback() {
        use futures::StreamExt;

        let primary =
            RunnableLambda::builder().func(|_x: i32| -> Result<i32> { Err(Error::other("primary failed")) }).build();

        let fallback = RunnableLambda::builder().func(|x: i32| -> Result<i32> { Ok(x * 2) }).build();

        let with_fallbacks = RunnableWithFallbacks::new(primary, vec![Arc::new(fallback)]);

        let mut stream = with_fallbacks.stream(5, None);
        let result = stream.next().await.unwrap().unwrap();
        assert_eq!(result, 10);
    }

    #[test]
    fn test_runnables_iterator() {
        let primary = RunnableLambda::builder().func(|x: i32| -> Result<i32> { Ok(x) }).build();
        let fallback1 = RunnableLambda::builder().func(|x: i32| -> Result<i32> { Ok(x) }).build();
        let fallback2 = RunnableLambda::builder().func(|x: i32| -> Result<i32> { Ok(x) }).build();

        let with_fallbacks =
            RunnableWithFallbacks::new(primary, vec![Arc::new(fallback1), Arc::new(fallback2)]);

        let count = with_fallbacks.runnables().count();
        assert_eq!(count, 3); // primary + 2 fallbacks
    }
}
