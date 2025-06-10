#[allow(dead_code)]
use bitte::bitte;

// Example 1: Apply bitte to an entire trait
#[bitte]
trait AsyncRepository {
    async fn find_by_id(&self, id: u64) -> Option<String>;
    async fn save(&mut self, data: String) -> Result<u64, String>;
    async fn delete(&mut self, id: u64) -> Result<(), String>;

    // Non-async methods remain unchanged
    fn cache_size(&self) -> usize;
}

// Example 2: Apply bitte with explicit bounds
#[bitte(Send, Sync)]
trait AsyncService {
    async fn process(&self, input: Vec<u8>) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
}

// Example 3: Apply bitte to individual methods with different configurations
trait AsyncMixedTrait {
    #[bitte]
    async fn default_bounds(&self) -> String;

    #[bitte(?Send)]
    async fn no_send(&self) -> String;

    #[bitte(?Sync)]
    async fn no_sync(&self) -> String;

    #[bitte(?Send, ?Sync)]
    async fn no_bounds(&self) -> String;

    // This method keeps its async nature
    async fn still_async(&self) -> String;
}

// Example implementations

// With #[bitte] on the impl block, you can write natural async methods
struct InMemoryRepo {
    data: std::collections::HashMap<u64, String>,
}

#[bitte]
impl AsyncRepository for InMemoryRepo {
    async fn find_by_id(&self, id: u64) -> Option<String> {
        self.data.get(&id).cloned()
    }

    async fn save(&mut self, data: String) -> Result<u64, String> {
        let id = rand::random::<u64>();
        self.data.insert(id, data);
        Ok(id)
    }

    async fn delete(&mut self, id: u64) -> Result<(), String> {
        self.data.remove(&id);
        Ok(())
    }

    fn cache_size(&self) -> usize {
        self.data.len()
    }
}

// Example showing manual desugaring (without #[bitte] on impl)
struct FileRepo {
    path: std::path::PathBuf,
}

impl AsyncRepository for FileRepo {
    fn find_by_id(&self, id: u64) -> impl std::future::Future<Output = Option<String>> {
        async move {
            // Simulate async file read
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            Some(format!("Data for id {}", id))
        }
    }

    fn save(&mut self, _data: String) -> impl std::future::Future<Output = Result<u64, String>> {
        async move {
            // Simulate async file write
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            Ok(rand::random::<u64>())
        }
    }

    fn delete(&mut self, _id: u64) -> impl std::future::Future<Output = Result<(), String>> {
        async move {
            // Simulate async file delete
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            Ok(())
        }
    }

    fn cache_size(&self) -> usize {
        0 // File-based repo doesn't cache
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut repo = InMemoryRepo {
        data: std::collections::HashMap::new(),
    };

    // Save some data
    let id = repo.save("Hello, bitte!".to_string()).await?;
    println!("Saved data with id: {}", id);

    // Find the data
    if let Some(data) = repo.find_by_id(id).await {
        println!("Found data: {}", data);
    }

    // Check cache size
    println!("Cache size: {}", repo.cache_size());

    // Delete the data
    repo.delete(id).await?;
    println!("Data deleted");

    Ok(())
}
