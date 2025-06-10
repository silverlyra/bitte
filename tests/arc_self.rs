use bitte::bitte;
use std::sync::Arc;

#[bitte]
trait ArcService {
    async fn arc_method(self: Arc<Self>) -> String;
    async fn arc_ref_method(self: Arc<Self>) -> String;
    async fn regular_ref(&self) -> String;
    async fn regular_mut(&mut self) -> String;
    async fn by_value(self) -> String;
}

#[derive(Clone)]
struct ServiceImpl {
    name: String,
}

impl ArcService for ServiceImpl {
    fn arc_method(self: Arc<Self>) -> impl Future<Output = String> + Send
    where
        Self: Sync,
    {
        async move { format!("Arc: {}", self.name) }
    }

    fn arc_ref_method(self: Arc<Self>) -> impl Future<Output = String> + Send
    where
        Self: Sync,
    {
        async move { format!("Arc: {}", self.name) }
    }

    fn regular_ref(&self) -> impl Future<Output = String>
    where
        Self: Sync,
    {
        let name = self.name.clone();
        async move { format!("&self: {}", name) }
    }

    fn regular_mut(&mut self) -> impl Future<Output = String> + Send {
        self.name.push_str(" (modified)");
        let name = self.name.clone();
        async move { format!("&mut self: {}", name) }
    }

    fn by_value(self) -> impl Future<Output = String> {
        async move { format!("self: {}", self.name) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_arc_methods() {
        let service = Arc::new(ServiceImpl {
            name: "test_service".to_string(),
        });

        // Test Arc<Self> method
        let result = service.clone().arc_method().await;
        assert_eq!(result, "Arc: test_service");

        // Test Arc<Self> method (both methods now take Arc<Self>)
        let result = service.clone().arc_ref_method().await;
        assert_eq!(result, "Arc: test_service");

        // Test regular &self method on Arc
        let result = service.regular_ref().await;
        assert_eq!(result, "&self: test_service");
    }

    #[tokio::test]
    async fn test_regular_methods() {
        let mut service = ServiceImpl {
            name: "mutable_service".to_string(),
        };

        // Test &mut self
        let result = service.regular_mut().await;
        assert_eq!(result, "&mut self: mutable_service (modified)");
        assert_eq!(service.name, "mutable_service (modified)");

        // Test by value
        let service2 = ServiceImpl {
            name: "consumed".to_string(),
        };
        let result = service2.by_value().await;
        assert_eq!(result, "self: consumed");
    }

    #[test]
    fn test_arc_futures_are_send_and_sync() {
        fn assert_send<T: Send>(_: T) {}
        fn assert_sync<T: Sync>(_: T) {}

        let service = Arc::new(ServiceImpl {
            name: "test".to_string(),
        });

        // Arc<Self> methods should produce Send + Sync futures
        let future1 = service.clone().arc_method();
        assert_send(future1);

        let future2 = service.clone().arc_method();
        assert_sync(future2);

        // Arc<Self> methods should also be Send + Sync
        let future3 = service.clone().arc_ref_method();
        assert_send(future3);

        let future4 = service.arc_ref_method();
        assert_sync(future4);
    }

    #[test]
    fn test_ref_futures_bounds() {
        fn assert_send<T: Send>(_: T) {}

        let service = ServiceImpl {
            name: "test".to_string(),
        };

        // &self methods should produce Send futures when Self: Sync
        let future = service.regular_ref();
        assert_send(future);
    }
}

// Test with generic Arc<Self>
#[bitte]
trait GenericArcService<T: Send + Sync + 'static> {
    async fn process(self: Arc<Self>, value: T) -> T;
}

struct GenericServiceImpl;

impl<T: Send + Sync + 'static> GenericArcService<T> for GenericServiceImpl {
    fn process(self: Arc<Self>, value: T) -> impl Future<Output = T> + Send
    where
        Self: Sync,
    {
        async move {
            // Simulate some async work
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            value
        }
    }
}

#[cfg(test)]
mod generic_arc_tests {
    use super::*;

    #[tokio::test]
    async fn test_generic_arc() {
        let service = Arc::new(GenericServiceImpl);

        let result = service.clone().process(42).await;
        assert_eq!(result, 42);

        let result = service.process("hello".to_string()).await;
        assert_eq!(result, "hello");
    }
}
