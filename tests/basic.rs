#![allow(dead_code)]

use bitte::bitte;

// Test applying bitte to an entire trait
#[bitte]
trait AsyncTraitDefault {
    async fn method(&self) -> u32;
    async fn method_with_args(&self, x: i32, y: String) -> Result<(), String>;
    fn sync_method(&self) -> u32; // This should remain unchanged
}

// Test applying bitte with Send and Sync explicitly enabled
#[bitte(Send, Sync)]
trait AsyncTraitWithBounds {
    async fn method(&self) -> u32;
}

// Test applying bitte with Send and Sync explicitly disabled
#[bitte(?Send, ?Sync)]
trait AsyncTraitNoBounds {
    async fn method(&self) -> u32;
}

// Test applying bitte to individual methods
trait AsyncTraitIndividual {
    #[bitte]
    async fn method_default(&self) -> u32;

    #[bitte(Send)]
    async fn method_send_only(&self) -> u32;

    #[bitte(?Send, ?Sync)]
    async fn method_no_bounds(&self) -> u32;

    async fn untransformed(&self) -> u32; // This should remain async
}

// Test with more complex return types
#[bitte]
trait ComplexReturns {
    async fn returns_result(&self) -> Result<Vec<String>, Box<dyn std::error::Error>>;
    async fn returns_option(&self) -> Option<i32>;
    async fn returns_unit(&self);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;

    // These tests mainly check that the code compiles and has the right types

    #[test]
    fn test_basic_compilation() {
        // Just check that the transformed traits compile
        fn _check_types<T: AsyncTraitDefault>(_t: T) {}
        fn _check_with_bounds<T: AsyncTraitWithBounds>(_t: T) {}
        fn _check_no_bounds<T: AsyncTraitNoBounds>(_t: T) {}
        fn _check_individual<T: AsyncTraitIndividual>(_t: T) {}
        fn _check_complex<T: ComplexReturns>(_t: T) {}
    }

    // Test that we can verify the return type is indeed impl Future
    fn _assert_future_return<F, T>(_f: F)
    where
        F: Fn() -> T,
        T: Future,
    {
    }

    struct TestImpl;

    impl AsyncTraitDefault for TestImpl {
        fn method(&self) -> impl std::future::Future<Output = u32> {
            async move { 42 }
        }

        fn method_with_args(
            &self,
            _x: i32,
            _y: String,
        ) -> impl std::future::Future<Output = Result<(), String>> {
            async move { Ok(()) }
        }

        fn sync_method(&self) -> u32 {
            42
        }
    }

    #[tokio::test]
    async fn test_runtime_behavior() {
        let test_impl = TestImpl;
        let result = test_impl.method().await;
        assert_eq!(result, 42);

        let result = test_impl.method_with_args(1, "test".to_string()).await;
        assert!(result.is_ok());
    }
}
