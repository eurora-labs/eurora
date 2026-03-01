use std::fmt::Debug;
use std::sync::Arc;

use async_trait::async_trait;
use futures::StreamExt;
use futures::stream::BoxStream;
use serde::Serialize;

use crate::error::{Error, Result};
use crate::load::{Serializable, Serialized, SerializedConstructorData};

use super::base::{DynRunnable, Runnable, RunnableLambda, RunnableSerializable};
use super::config::{RunnableConfig, ensure_config, get_callback_manager_for_config, patch_config};

pub struct RunnableBranch<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    branches: Vec<(DynRunnable<I, bool>, DynRunnable<I, O>)>,
    default: DynRunnable<I, O>,
    name: Option<String>,
}

impl<I, O> Debug for RunnableBranch<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunnableBranch")
            .field("branches_count", &self.branches.len())
            .field("name", &self.name)
            .finish()
    }
}

#[bon::bon]
impl<I, O> RunnableBranch<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    #[builder]
    pub fn new(
        branches: Vec<(DynRunnable<I, bool>, DynRunnable<I, O>)>,
        default: DynRunnable<I, O>,
        #[builder(into)] name: Option<String>,
    ) -> Result<Self> {
        if branches.is_empty() {
            return Err(Error::Other(
                "RunnableBranch requires at least one condition branch".to_string(),
            ));
        }

        Ok(Self {
            branches,
            default,
            name,
        })
    }
}

pub struct RunnableBranchFluentBuilder<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    branches: Vec<(DynRunnable<I, bool>, DynRunnable<I, O>)>,
    _phantom: std::marker::PhantomData<(I, O)>,
}

impl<I, O> RunnableBranchFluentBuilder<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    pub fn new() -> Self {
        Self {
            branches: Vec::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn branch<CF, RF>(mut self, condition: CF, runnable: RF) -> Self
    where
        CF: Fn(I) -> Result<bool> + Send + Sync + 'static,
        RF: Fn(I) -> Result<O> + Send + Sync + 'static,
    {
        let condition_runnable: DynRunnable<I, bool> =
            Arc::new(RunnableLambda::builder().func(condition).build());
        let branch_runnable: DynRunnable<I, O> =
            Arc::new(RunnableLambda::builder().func(runnable).build());
        self.branches.push((condition_runnable, branch_runnable));
        self
    }

    pub fn branch_arc(
        mut self,
        condition: DynRunnable<I, bool>,
        runnable: DynRunnable<I, O>,
    ) -> Self {
        self.branches.push((condition, runnable));
        self
    }

    pub fn default<DF>(self, default_fn: DF) -> Result<RunnableBranch<I, O>>
    where
        DF: Fn(I) -> Result<O> + Send + Sync + 'static,
    {
        let default_runnable: DynRunnable<I, O> =
            Arc::new(RunnableLambda::builder().func(default_fn).build());
        RunnableBranch::builder()
            .branches(self.branches)
            .default(default_runnable)
            .build()
    }

    pub fn default_arc(self, default: DynRunnable<I, O>) -> Result<RunnableBranch<I, O>> {
        RunnableBranch::builder()
            .branches(self.branches)
            .default(default)
            .build()
    }
}

impl<I, O> Default for RunnableBranchFluentBuilder<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<I, O> Runnable for RunnableBranch<I, O>
where
    I: Send + Sync + Clone + Debug + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    type Input = I;
    type Output = O;

    fn name(&self) -> Option<String> {
        self.name
            .clone()
            .or_else(|| Some("RunnableBranch".to_string()))
    }

    fn get_input_schema(&self, config: Option<&RunnableConfig>) -> serde_json::Value {
        let schema = self.default.get_input_schema(config);
        if schema.get("type").is_some() {
            return schema;
        }
        for (condition, runnable) in &self.branches {
            let schema = runnable.get_input_schema(config);
            if schema.get("type").is_some() {
                return schema;
            }
            let schema = condition.get_input_schema(config);
            if schema.get("type").is_some() {
                return schema;
            }
        }
        self.default.get_input_schema(config)
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

        let result = (|| {
            for (idx, (condition, runnable)) in self.branches.iter().enumerate() {
                let condition_config = patch_config(
                    Some(config.clone()),
                    Some(run_manager.get_child(Some(&format!("condition:{}", idx + 1)))),
                    None,
                    None,
                    None,
                    None,
                );

                let expression_value = condition.invoke(input.clone(), Some(condition_config))?;

                if expression_value {
                    let branch_config = patch_config(
                        Some(config.clone()),
                        Some(run_manager.get_child(Some(&format!("branch:{}", idx + 1)))),
                        None,
                        None,
                        None,
                        None,
                    );

                    return runnable.invoke(input.clone(), Some(branch_config));
                }
            }

            let default_config = patch_config(
                Some(config.clone()),
                Some(run_manager.get_child(Some("branch:default"))),
                None,
                None,
                None,
                None,
            );

            self.default.invoke(input, Some(default_config))
        })();

        match &result {
            Ok(_) => {
                run_manager.on_chain_end(&std::collections::HashMap::new());
            }
            Err(e) => {
                run_manager.on_chain_error(e);
            }
        }

        result
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

        for (condition, runnable) in self.branches.iter() {
            let expression_value = condition
                .ainvoke(input.clone(), Some(config.clone()))
                .await?;

            if expression_value {
                return runnable.ainvoke(input.clone(), Some(config.clone())).await;
            }
        }

        self.default.ainvoke(input, Some(config)).await
    }

    fn stream(
        &self,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> BoxStream<'_, Result<Self::Output>> {
        let config = ensure_config(config);

        Box::pin(async_stream::stream! {
            'outer: {
                for (condition, runnable) in self.branches.iter() {
                    let expression_value = match condition.invoke(input.clone(), Some(config.clone())) {
                        Ok(v) => v,
                        Err(e) => {
                            yield Err(e);
                            break 'outer;
                        }
                    };

                    if expression_value {
                        let mut stream = runnable.stream(input.clone(), Some(config.clone()));
                        while let Some(chunk_result) = stream.next().await {
                            yield chunk_result;
                        }
                        break 'outer;
                    }
                }

                let mut stream = self.default.stream(input, Some(config.clone()));
                while let Some(chunk_result) = stream.next().await {
                    yield chunk_result;
                }
            }
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
            'outer: {
                for (condition, runnable) in self.branches.iter() {
                    let expression_value = match condition.ainvoke(input.clone(), Some(config.clone())).await {
                        Ok(v) => v,
                        Err(e) => {
                            yield Err(e);
                            break 'outer;
                        }
                    };

                    if expression_value {
                        let mut stream = runnable.astream(input.clone(), Some(config.clone()));
                        while let Some(chunk_result) = stream.next().await {
                            yield chunk_result;
                        }
                        break 'outer;
                    }
                }

                let mut stream = self.default.astream(input, Some(config.clone()));
                while let Some(chunk_result) = stream.next().await {
                    yield chunk_result;
                }
            }
        })
    }
}

impl<I, O> Serializable for RunnableBranch<I, O>
where
    I: Send + Sync + Clone + Debug + Serialize + 'static,
    O: Send + Sync + Clone + Debug + 'static,
{
    fn is_lc_serializable() -> bool {
        true
    }

    fn get_lc_namespace() -> Vec<String> {
        vec![
            "langchain".to_string(),
            "schema".to_string(),
            "runnable".to_string(),
        ]
    }

    fn to_json(&self) -> Serialized {
        let kwargs = std::collections::HashMap::new();

        Serialized::Constructor(SerializedConstructorData {
            lc: 1,
            id: Self::get_lc_namespace(),
            kwargs,
            name: None,
            graph: None,
        })
    }
}

impl<I, O> RunnableSerializable for RunnableBranch<I, O>
where
    I: Send + Sync + Clone + Debug + Serialize + 'static,
    O: Send + Sync + Clone + Debug + Serialize + 'static,
{
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runnable_branch_invoke_first_condition() {
        let branch = RunnableBranchFluentBuilder::new()
            .branch(|x: i32| Ok(x > 0), |x: i32| Ok(format!("positive: {}", x)))
            .branch(|x: i32| Ok(x < 0), |x: i32| Ok(format!("negative: {}", x)))
            .default(|_: i32| Ok("zero".to_string()))
            .unwrap();

        let result = branch.invoke(5, None).unwrap();
        assert_eq!(result, "positive: 5");
    }

    #[test]
    fn test_runnable_branch_invoke_second_condition() {
        let branch = RunnableBranchFluentBuilder::new()
            .branch(|x: i32| Ok(x > 0), |x: i32| Ok(format!("positive: {}", x)))
            .branch(|x: i32| Ok(x < 0), |x: i32| Ok(format!("negative: {}", x)))
            .default(|_: i32| Ok("zero".to_string()))
            .unwrap();

        let result = branch.invoke(-3, None).unwrap();
        assert_eq!(result, "negative: -3");
    }

    #[test]
    fn test_runnable_branch_invoke_default() {
        let branch = RunnableBranchFluentBuilder::new()
            .branch(|x: i32| Ok(x > 0), |x: i32| Ok(format!("positive: {}", x)))
            .branch(|x: i32| Ok(x < 0), |x: i32| Ok(format!("negative: {}", x)))
            .default(|_: i32| Ok("zero".to_string()))
            .unwrap();

        let result = branch.invoke(0, None).unwrap();
        assert_eq!(result, "zero");
    }

    #[test]
    fn test_runnable_branch_requires_at_least_one_branch() {
        let result: Result<RunnableBranch<i32, String>> =
            RunnableBranchFluentBuilder::new().default(|_: i32| Ok("default".to_string()));

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("at least one condition branch")
        );
    }

    #[test]
    fn test_runnable_branch_name() {
        let condition: DynRunnable<i32, bool> =
            Arc::new(RunnableLambda::builder().func(|x: i32| Ok(x > 0)).build());
        let branch_runnable: DynRunnable<i32, String> = Arc::new(
            RunnableLambda::builder()
                .func(|x: i32| Ok(x.to_string()))
                .build(),
        );
        let default: DynRunnable<i32, String> = Arc::new(
            RunnableLambda::builder()
                .func(|_: i32| Ok("default".to_string()))
                .build(),
        );

        let branch = RunnableBranch::builder()
            .branches(vec![(condition, branch_runnable)])
            .default(default)
            .name("my_branch")
            .build()
            .unwrap();

        assert_eq!(branch.name(), Some("my_branch".to_string()));
    }

    #[test]
    fn test_runnable_branch_default_name() {
        let branch = RunnableBranchFluentBuilder::new()
            .branch(|x: i32| Ok(x > 0), |x: i32| Ok(x.to_string()))
            .default(|_: i32| Ok("default".to_string()))
            .unwrap();

        assert_eq!(branch.name(), Some("RunnableBranch".to_string()));
    }

    #[test]
    fn test_runnable_branch_with_arc_runnables() {
        let condition: DynRunnable<i32, bool> =
            Arc::new(RunnableLambda::builder().func(|x: i32| Ok(x > 10)).build());
        let branch_runnable: DynRunnable<i32, String> = Arc::new(
            RunnableLambda::builder()
                .func(|x: i32| Ok(format!("big: {}", x)))
                .build(),
        );
        let default: DynRunnable<i32, String> = Arc::new(
            RunnableLambda::builder()
                .func(|x: i32| Ok(format!("small: {}", x)))
                .build(),
        );

        let branch = RunnableBranch::builder()
            .branches(vec![(condition, branch_runnable)])
            .default(default)
            .build()
            .unwrap();

        assert_eq!(branch.invoke(15, None).unwrap(), "big: 15");
        assert_eq!(branch.invoke(5, None).unwrap(), "small: 5");
    }

    #[tokio::test]
    async fn test_runnable_branch_ainvoke() {
        let branch = RunnableBranchFluentBuilder::new()
            .branch(|x: i32| Ok(x > 0), |x: i32| Ok(format!("positive: {}", x)))
            .branch(|x: i32| Ok(x < 0), |x: i32| Ok(format!("negative: {}", x)))
            .default(|_: i32| Ok("zero".to_string()))
            .unwrap();

        let result = branch.ainvoke(5, None).await.unwrap();
        assert_eq!(result, "positive: 5");

        let result = branch.ainvoke(-3, None).await.unwrap();
        assert_eq!(result, "negative: -3");

        let result = branch.ainvoke(0, None).await.unwrap();
        assert_eq!(result, "zero");
    }

    #[tokio::test]
    async fn test_runnable_branch_stream() {
        let branch = RunnableBranchFluentBuilder::new()
            .branch(|x: i32| Ok(x > 0), |x: i32| Ok(format!("positive: {}", x)))
            .default(|_: i32| Ok("non-positive".to_string()))
            .unwrap();

        let mut stream = branch.stream(5, None);
        let result = stream.next().await.unwrap().unwrap();
        assert_eq!(result, "positive: 5");
    }

    #[tokio::test]
    async fn test_runnable_branch_astream() {
        let branch = RunnableBranchFluentBuilder::new()
            .branch(|x: i32| Ok(x > 0), |x: i32| Ok(format!("positive: {}", x)))
            .default(|_: i32| Ok("non-positive".to_string()))
            .unwrap();

        let mut stream = branch.astream(5, None);
        let result = stream.next().await.unwrap().unwrap();
        assert_eq!(result, "positive: 5");
    }
}
