#![allow(dead_code)]

use bitte::bitte;

// Test trait with async methods
#[bitte]
trait AsyncService {
    async fn process(&self, input: &str) -> String;
    async fn validate(&self, data: &[u8]) -> bool;
    fn sync_method(&self) -> u32;
}

// Test trait with different bound configurations
#[bitte(?Send)]
trait LocalAsyncService {
    async fn local_process(&self) -> String;
}

#[bitte(Send, Sync)]
trait ThreadSafeService {
    async fn concurrent_process(&self) -> u32;
}

// Basic implementation with #[bitte]
struct BasicService {
    prefix: String,
}

impl AsyncService for BasicService {
    fn process(&self, input: &str) -> impl Future<Output = String>
    where
        Self: Sync,
    {
        let prefix = self.prefix.clone();
        let input = input.to_string();
        async move {
            // Simulate some async work
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            format!("{}: {}", prefix, input)
        }
    }

    fn validate(&self, data: &[u8]) -> impl Future<Output = bool>
    where
        Self: Sync,
    {
        let is_empty = data.is_empty();
        async move {
            // Another async operation
            tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
            !is_empty
        }
    }

    fn sync_method(&self) -> u32 {
        42
    }
}

// Implementation with custom bounds
struct LocalService;

#[bitte(?Send)]
impl LocalAsyncService for LocalService {
    async fn local_process(&self) -> String {
        // This doesn't need to be Send
        let local_data = std::rc::Rc::new("local data");
        tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        format!("Processed: {}", local_data)
    }
}

// Thread-safe implementation
struct ConcurrentService;

#[bitte(Send, Sync)]
impl ThreadSafeService for ConcurrentService {
    async fn concurrent_process(&self) -> u32 {
        tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        100
    }
}

// Test mixed async/sync methods
#[bitte]
trait MixedService {
    async fn async_method(&self) -> Result<String, String>;
    fn sync_method(&self) -> i32;
    async fn another_async(&mut self, value: i32) -> i32;
}

struct MixedImpl {
    counter: i32,
}

#[bitte]
impl MixedService for MixedImpl {
    async fn async_method(&self) -> Result<String, String> {
        tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        Ok(format!("Counter: {}", self.counter))
    }

    fn sync_method(&self) -> i32 {
        self.counter
    }

    async fn another_async(&mut self, value: i32) -> i32 {
        tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        self.counter += value;
        self.counter
    }
}

// Test generic implementations
#[bitte]
trait GenericService<T> {
    async fn process_generic(&self, item: T) -> T;
}

struct GenericImpl;

#[bitte]
impl<T: Clone + Send + Sync + 'static> GenericService<T> for GenericImpl {
    async fn process_generic(&self, item: T) -> T {
        tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        item.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_impl_block() {
        let service = BasicService {
            prefix: "Test".to_string(),
        };

        let result = service.process("hello").await;
        assert_eq!(result, "Test: hello");

        let valid = service.validate(b"data").await;
        assert!(valid);

        let sync_result = service.sync_method();
        assert_eq!(sync_result, 42);
    }

    #[tokio::test]
    async fn test_local_service() {
        let service = LocalService;
        let result = service.local_process().await;
        assert!(result.contains("Processed:"));
    }

    #[tokio::test]
    async fn test_concurrent_service() {
        let service = ConcurrentService;
        let result = service.concurrent_process().await;
        assert_eq!(result, 100);
    }

    #[tokio::test]
    async fn test_mixed_service() {
        let mut service = MixedImpl { counter: 5 };

        let async_result = service.async_method().await;
        assert_eq!(async_result.unwrap(), "Counter: 5");

        let sync_result = service.sync_method();
        assert_eq!(sync_result, 5);

        let new_count = service.another_async(3).await;
        assert_eq!(new_count, 8);
    }

    #[tokio::test]
    async fn test_generic_impl() {
        let service = GenericImpl;

        let string_result = service.process_generic("test").await;
        assert_eq!(string_result, "test");

        let num_result = service.process_generic(42).await;
        assert_eq!(num_result, 42);
    }

    // Test that Send bounds work correctly
    #[test]
    fn test_send_bounds() {
        fn assert_send<T: Send>(_: T) {}

        let service = ConcurrentService;
        let future = service.concurrent_process();
        assert_send(future);
    }
}
