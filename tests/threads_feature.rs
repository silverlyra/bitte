#![allow(dead_code)]

#[cfg(feature = "threads")]
mod threads_enabled {
    use bitte::bitte;

    // When threads feature is enabled, default should add Send + Sync
    #[bitte]
    trait AsyncTraitThreadsDefault {
        async fn method(&self) -> u32;
    }

    // Explicit override should still work
    #[bitte(?Send, ?Sync)]
    trait AsyncTraitThreadsOverride {
        async fn method(&self) -> u32;
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        // Helper to test that a future is Send
        fn assert_send<T: Send>(_t: T) {}

        // Helper to test that a future is Sync
        fn assert_sync<T: Sync>(_t: T) {}

        struct TestImpl;

        impl AsyncTraitThreadsDefault for TestImpl {
            fn method(&self) -> impl std::future::Future<Output = u32> + Send
            where
                Self: Sync,
            {
                async move { 42 }
            }
        }

        impl AsyncTraitThreadsOverride for TestImpl {
            fn method(&self) -> impl std::future::Future<Output = u32> {
                async move { 42 }
            }
        }

        #[test]
        fn test_threads_bounds() {
            let test_impl = TestImpl;
            let future = AsyncTraitThreadsDefault::method(&test_impl);

            // The future should be Send when threads feature is enabled
            assert_send(future);
        }
    }
}

#[cfg(not(feature = "threads"))]
mod threads_disabled {
    use bitte::bitte;

    // When threads feature is disabled, default should not add bounds
    #[bitte]
    trait AsyncTraitNoThreads {
        async fn method_no_threads(&self) -> u32;
    }

    // Explicit Send should still work
    #[bitte(Send)]
    trait AsyncTraitExplicitSend {
        async fn method_explicit(&self) -> u32;
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        struct TestImpl;

        impl AsyncTraitNoThreads for TestImpl {
            fn method_no_threads(&self) -> impl std::future::Future<Output = u32> {
                async move { 42 }
            }
        }

        impl AsyncTraitExplicitSend for TestImpl {
            fn method_explicit(&self) -> impl std::future::Future<Output = u32> + Send {
                async move { 42 }
            }
        }

        #[tokio::test]
        async fn test_no_threads_behavior() {
            let test_impl = TestImpl;
            let result = test_impl.method_no_threads().await;
            assert_eq!(result, 42);
        }
    }
}
