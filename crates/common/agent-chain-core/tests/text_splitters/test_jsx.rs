use agent_chain_core::TextSplitter;
use agent_chain_core::{JSFrameworkTextSplitter, KeepSeparator, TextSplitterConfig};

const FAKE_JSX_TEXT: &str = "\nimport React from 'react';\nimport OtherComponent from './OtherComponent';\n\nfunction MyComponent() {\n  const [count, setCount] = React.useState(0);\n\n  const handleClick = () => {\n    setCount(count + 1);\n  };\n\n  return (\n    <div>\n      <h1>Counter: {count}</h1>\n      <button onClick={handleClick}>\n        Increment\n      </button>\n      <OtherComponent />\n    </div>\n  );\n}\n\nexport default MyComponent;\n";

const FAKE_VUE_TEXT: &str = "\n<template>\n  <div>\n    <h1>{{ title }}</h1>\n    <button @click=\"increment\">\n      Count is: {{ count }}\n    </button>\n  </div>\n</template>\n\n<script>\nexport default {\n  data() {\n    return {\n      title: 'Counter App',\n      count: 0\n    }\n  },\n  methods: {\n    increment() {\n      this.count++\n    }\n  }\n}\n</script>\n\n<style>\nbutton {\n  color: blue;\n}\n</style>\n";

const FAKE_SVELTE_TEXT: &str = "\n<script>\n  let count = 0\n\n  function increment() {\n    count += 1\n  }\n</script>\n\n<main>\n  <h1>Counter App</h1>\n  <button on:click={increment}>\n    Count is: {count}\n  </button>\n</main>\n\n<style>\n  button {\n    color: blue;\n  }\n</style>\n";

#[test]
fn test_jsx_text_splitter() {
    let config =
        TextSplitterConfig::new(30, 0, None, Some(KeepSeparator::Start), None, None).unwrap();
    let splitter = JSFrameworkTextSplitter::new(None, config);
    let splits = splitter.split_text(FAKE_JSX_TEXT).unwrap();

    let expected_splits = [
        "\nimport React from 'react';\nimport OtherComponent from './OtherComponent';\n",
        "\nfunction MyComponent() {\n  const [count, setCount] = React.useState(0);",
        "\n\n  const handleClick = () => {\n    setCount(count + 1);\n  };",
        "return (",
        "<div>",
        "<h1>Counter: {count}</h1>\n      ",
        "<button onClick={handleClick}>\n        Increment\n      </button>\n      ",
        "<OtherComponent />\n    </div>\n  );\n}\n",
        "export default MyComponent;",
    ];

    let trimmed_splits: Vec<String> = splits.iter().map(|s| s.trim().to_string()).collect();
    let trimmed_expected: Vec<String> = expected_splits
        .iter()
        .map(|s| s.trim().to_string())
        .collect();
    assert_eq!(trimmed_splits, trimmed_expected);
}

#[test]
fn test_vue_text_splitter() {
    let config =
        TextSplitterConfig::new(30, 0, None, Some(KeepSeparator::Start), None, None).unwrap();
    let splitter = JSFrameworkTextSplitter::new(None, config);
    let splits = splitter.split_text(FAKE_VUE_TEXT).unwrap();

    let expected_splits = [
        "<template>",
        "<div>",
        "<h1>{{ title }}</h1>",
        "<button @click=\"increment\">\n      Count is: {{ count }}\n    </button>\n  </div>\n</template>",
        "<script>",
        "export",
        "default {\n  data() {\n    return {\n      title: 'Counter App',\n      count: 0\n    }\n  },\n  methods: {\n    increment() {\n      this.count++\n    }\n  }\n}\n</script>",
        "<style>\nbutton {\n  color: blue;\n}\n</style>",
    ];

    let trimmed_splits: Vec<String> = splits.iter().map(|s| s.trim().to_string()).collect();
    let trimmed_expected: Vec<String> = expected_splits
        .iter()
        .map(|s| s.trim().to_string())
        .collect();
    assert_eq!(trimmed_splits, trimmed_expected);
}

#[test]
fn test_svelte_text_splitter() {
    let config =
        TextSplitterConfig::new(30, 0, None, Some(KeepSeparator::Start), None, None).unwrap();
    let splitter = JSFrameworkTextSplitter::new(None, config);
    let splits = splitter.split_text(FAKE_SVELTE_TEXT).unwrap();

    let expected_splits = [
        "<script>\n  let count = 0",
        "\n\n  function increment() {\n    count += 1\n  }\n</script>",
        "<main>",
        "<h1>Counter App</h1>",
        "<button on:click={increment}>\n    Count is: {count}\n  </button>\n</main>",
        "<style>\n  button {\n    color: blue;\n  }\n</style>",
    ];

    let trimmed_splits: Vec<String> = splits.iter().map(|s| s.trim().to_string()).collect();
    let trimmed_expected: Vec<String> = expected_splits
        .iter()
        .map(|s| s.trim().to_string())
        .collect();
    assert_eq!(trimmed_splits, trimmed_expected);
}
