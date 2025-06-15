use bitte::bitte;

#[bitte]
trait AsyncTraitWithDefaults {
    // Required method
    async fn required_method(&self) -> String;

    // Default async method
    async fn default_method(&self) -> String {
        "default implementation".to_string()
    }

    // Default async method that calls another method
    async fn composed_default(&self) -> String {
        let required = self.required_method().await;
        format!("composed: {}", required)
    }

    // Default async method with parameters (takes ownership to avoid lifetime issues)
    async fn default_with_params(&self, prefix: String) -> String {
        format!("{}: default", prefix)
    }

    // Non-async default method (should remain unchanged)
    fn sync_default(&self) -> &'static str {
        "sync default"
    }
}

struct CustomImpl;

#[bitte]
impl AsyncTraitWithDefaults for CustomImpl {
    async fn required_method(&self) -> String {
        "custom implementation".to_string()
    }

    // Override one default method
    async fn default_method(&self) -> String {
        "overridden default".to_string()
    }
}

struct MinimalImpl;

#[bitte]
impl AsyncTraitWithDefaults for MinimalImpl {
    async fn required_method(&self) -> String {
        "minimal".to_string()
    }
    // Uses all default implementations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_custom_impl() {
        let custom = CustomImpl;

        assert_eq!(custom.required_method().await, "custom implementation");
        assert_eq!(custom.default_method().await, "overridden default");
        assert_eq!(
            custom.composed_default().await,
            "composed: custom implementation"
        );
        assert_eq!(
            custom.default_with_params("test".to_string()).await,
            "test: default"
        );
        assert_eq!(custom.sync_default(), "sync default");
    }

    #[tokio::test]
    async fn test_minimal_impl() {
        let minimal = MinimalImpl;

        assert_eq!(minimal.required_method().await, "minimal");
        assert_eq!(minimal.default_method().await, "default implementation");
        assert_eq!(minimal.composed_default().await, "composed: minimal");
        assert_eq!(
            minimal.default_with_params("hello".to_string()).await,
            "hello: default"
        );
        assert_eq!(minimal.sync_default(), "sync default");
    }

    #[tokio::test]
    async fn test_future_is_send() {
        fn assert_send<T: Send>(_: T) {}

        let custom = CustomImpl;
        assert_send(custom.required_method());
        assert_send(custom.default_method());
        assert_send(custom.composed_default());
    }
}

// Test with generic trait
#[bitte]
trait GenericAsyncWithDefaults<T: Send + Sync + 'static> {
    async fn process(&self, value: T) -> T;

    async fn process_twice(&self, value: T) -> T
    where
        T: Clone,
    {
        let once = self.process(value.clone()).await;
        self.process(once).await
    }
}

struct GenericImpl;

#[bitte]
impl<T: Send + Sync + 'static> GenericAsyncWithDefaults<T> for GenericImpl {
    async fn process(&self, value: T) -> T {
        value
    }
}

#[cfg(test)]
mod generic_tests {
    use super::*;

    #[tokio::test]
    async fn test_generic_defaults() {
        let generic_impl = GenericImpl;

        let result = generic_impl.process(42).await;
        assert_eq!(result, 42);

        let result = generic_impl.process_twice(10).await;
        assert_eq!(result, 10);
    }
}
