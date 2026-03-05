# Output Parsers Migration Plan

Bring the Rust output parser pipeline to behavioral equivalence with the Python
`langchain_core.output_parsers` module. Each section is ordered by priority.

---

## 1. Input Type Coercion

**Problem**: Python's `BaseOutputParser.invoke()` accepts `Union[str, BaseMessage]`
and normalises internally. Rust only accepts `BaseMessage`.

```python
# Python — base.py:193-213
def invoke(self, input: str | BaseMessage, ...):
    if isinstance(input, BaseMessage):
        ...parse_result([ChatGeneration(message=inner_input)])
    else:
        ...parse_result([Generation(text=inner_input)])
```

**Changes needed** (`base.rs`):

- Introduce a `LanguageModelOutput` enum (or equivalent) that holds either a
  `String` or a `BaseMessage`.
- Update `BaseOutputParser::invoke()` to accept `LanguageModelOutput` and
  dispatch to the correct `Generation` / `ChatGeneration` construction.
- Update `RunnableOutputParser` so its `Runnable::Input` type matches.

---

## 2. Runnable Piping / `RunnableSequence`

**Problem**: Python composes runnables with `|` to create a `RunnableSequence`.
Rust has no operator overloading or chaining equivalent.

```python
# Python — base.py:616-635
chain = model | parser          # __or__ → RunnableSequence
result = chain.invoke(input)
```

**Changes needed** (`runnables/base.rs`):

- Implement a `RunnableSequence<R1, R2>` struct that holds two runnables where
  `R2::Input == R1::Output`.
- Add a `.pipe(other)` method to the `Runnable` trait that returns
  `RunnableSequence<Self, R2>`.
- Optionally implement `std::ops::BitOr` on `Runnable` implementors to enable
  `model | parser` syntax.
- `RunnableSequence` must forward `invoke`, `ainvoke`, `stream`, and `transform`.

---

## 3. Async `ainvoke` — Stop Blocking

**Problem**: `RunnableOutputParser::ainvoke()` calls the synchronous `invoke()`
directly, blocking the async runtime. Python offloads to a thread pool via
`run_in_executor`.

```python
# Python — base.py:60
async def aparse_result(self, result, *, partial=False):
    return await run_in_executor(None, self.parse_result, result, partial=partial)
```

**Changes needed** (`base.rs`):

- In `RunnableOutputParser::ainvoke()`, wrap the synchronous call in
  `tokio::task::spawn_blocking` (or the equivalent for the runtime in use).
- Similarly update `BaseOutputParser::aparse_result` default implementation.

---

## 4. Streaming `transform()` on the Runnable Trait

**Problem**: Python's `Runnable` base class has `transform()` and `atransform()`
methods. `RunnableSequence` chains them for end-to-end streaming. The Rust
`Runnable` trait has no `transform()`.

**Changes needed** (`runnables/base.rs`):

- Add a `transform()` method to the `Runnable` trait:
  ```rust
  fn transform<'a>(
      &'a self,
      input: BoxStream<'a, Self::Input>,
      config: Option<RunnableConfig>,
  ) -> BoxStream<'a, Result<Self::Output>>;
  ```
- Provide a default implementation that collects the stream, calls `invoke`,
  and yields the result.
- `RunnableSequence::transform()` should pipe the first runnable's transform
  output into the second runnable's transform input.

---

## 5. Streaming Input Type Coercion

**Problem**: Python's `BaseTransformOutputParser._transform()` accepts
`Iterator[str | BaseMessage]`, handling both types. Rust's `transform()` only
accepts `BoxStream<BaseMessage>`.

```python
# Python — transform.py:31-39
def _transform(self, input: Iterator[str | BaseMessage]) -> Iterator[T]:
    for chunk in input:
        if isinstance(chunk, BaseMessage):
            yield self.parse_result([ChatGeneration(message=chunk)])
        else:
            yield self.parse_result([Generation(text=chunk)])
```

**Changes needed** (`output_parsers/transform.rs`):

- Use the same `LanguageModelOutput` enum from item 1 as the stream item type.
- Update `BaseTransformOutputParser::transform()` and
  `BaseCumulativeTransformOutputParser::cumulative_transform()` to dispatch
  correctly on each variant.

---

## 6. Partial Parse Semantics — Return `None` Instead of `Err`

**Problem**: When `partial=True`, Python returns `None` on parse failure (the
caller skips the chunk). Rust returns `Err(...)`, which the cumulative transform
silently discards with `let Ok(...) else { continue; }`.

```python
# Python — json.py:79-81
if partial:
    try:
        return parse_json_markdown(text)
    except JSONDecodeError:
        return None
```

**Changes needed** (`output_parsers/json.rs`, `pydantic.rs`):

- Change `parse_result()` return type (when `partial=true`) to
  `Result<Option<Self::Output>>`, or keep `Result` but return `Ok(None)` on
  expected partial failures.
- Update `BaseCumulativeTransformOutputParser::cumulative_transform()` in
  `transform.rs` to check for `None` explicitly rather than silently discarding
  errors.

---

## 7. Config Propagation / Run Tracking

**Problem**: Python wraps every `invoke` and `transform` call in
`_call_with_config` / `_transform_stream_with_config`, which handles run
callbacks, tracing, and metadata. Rust has no equivalent.

**Changes needed**:

- This is a broader `Runnable` infrastructure concern and does not need to
  block the output-parser-specific work.
- When the general `Runnable` run-tracking infrastructure is built, output
  parser `invoke` / `transform` calls should go through it.
- For now, document that config propagation is deferred.

---

## 8. Tool Call Return Types

**Problem**: Python's `JsonOutputToolsParser` returns proper `ToolCall` objects
(via `create_tool_call`). Rust returns raw `serde_json::Value`.

```python
# Python — openai_tools.py:73-77
if return_id:
    parsed["id"] = raw_tool_call.get("id")
    parsed = create_tool_call(**parsed)
```

**Changes needed** (`output_parsers/openai_tools.rs`):

- Define a `ToolCall` struct (or reuse one from elsewhere in the crate) with
  `name`, `args`, `id`, and `type` fields.
- Have `parse_tool_call()` return `Result<Option<ToolCall>>` when `return_id`
  is true.
- Update `JsonOutputToolsParser::parse_result()` and
  `PydanticToolsParser::parse_result()` to return `Vec<ToolCall>`.

---

## 9. Format Instructions Parity

**Problem**: The Rust `JSON_FORMAT_INSTRUCTIONS` constant contains extra
strictness directives not present in Python. While arguably helpful, this
diverges from the reference implementation.

**Changes needed** (`output_parsers/format_instructions.rs`):

- Align the constant text with the Python version verbatim.
- If the extra strictness text is desired, move it to a separate
  `STRICT_JSON_FORMAT_INSTRUCTIONS` constant.

---

## 10. XML Parser Security Option

**Problem**: Python's `XMLOutputParser` lets the user choose between
`defusedxml` (XXE-safe) and `xml` parsers. Rust always uses `quick_xml` with
no configurable security posture.

**Changes needed** (`output_parsers/xml.rs`):

- `quick_xml` is not vulnerable to XXE by default, so this is low priority.
- Add a `parser` config field to `XMLOutputParser` for API parity, even if
  both options map to the same underlying parser in Rust.

---

## Summary

| # | Area                          | Priority | Effort |
|---|-------------------------------|----------|--------|
| 1 | Input type coercion           | High     | Medium |
| 2 | `RunnableSequence` / piping   | High     | Large  |
| 3 | Async `ainvoke` blocking fix  | High     | Small  |
| 4 | `transform()` on Runnable     | High     | Medium |
| 5 | Streaming input coercion      | Medium   | Small  |
| 6 | Partial parse `None` semantics| Medium   | Small  |
| 7 | Config / run tracking         | Low      | Large  |
| 8 | `ToolCall` return types       | Medium   | Medium |
| 9 | Format instructions parity    | Low      | Small  |
| 10| XML parser security option    | Low      | Small  |
