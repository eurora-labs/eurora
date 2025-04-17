# Asset Strategy Class Diagram

```mermaid
classDiagram
    AssetContext o--> AssetStrategy
    AssetStrategy <|.. BrowserStrategy

    class AssetContext {
        -strategy: AssetStrategy

        +setStrategy(strategy: AssetStrategy)
        +setStrategyByProcessName(process_name: String)
        
        +retrieveAssets()
    }

    class AssetStrategy {
        <<interface>>

        +execute()
    }

    class BrowserStrategy {
        +execute()
    }


```
