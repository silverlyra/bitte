# Bitte

[![Crate](https://img.shields.io/crates/v/bitte.svg)](https://lib.rs/bitte)
[![Documentation](https://docs.rs/bitte/badge.svg)](https://docs.rs/bitte)

Starting with [Rust 1.75][impl-trait], it’s possible to use `async fn` in a `trait`:

```rust
pub trait UserDatabase {
    async fn get_user(&self, id: u64) -> Result<User, …>;
}
```

But if you use an `async` function in a `pub trait`, Rust issues a [warning][async_fn_in_trait]:

> ⚠️ **warning**: use of `async fn` in public traits is discouraged as auto trait bounds cannot be specified  
> ℹ️ _**help:**_ you can alternatively desugar to a normal `fn` that returns `impl Future`

This warning means that the `async fn` won’t be usable in a multithreaded program. For example, if [Tokio][]’s multithreaded runtime is used, you cannot [`spawn`][tokio-spawn] a task to run the `Future` returned by `get_user`. Tokio may execute the future on other thread(s), so the future needs to be marked as [sendable][send-sync] to other threads.

The compiler warning recommends “desugar”ing the `async fn` into something like:

```rust
pub trait UserDatabase {
    fn get_user(&self, id: u64) -> impl Future<Output = Result<User, …>> + Send
      where Self: Sync;
}
```

By using the `impl Future` syntax instead, you’re able to apply the necessary `Send` and/or `Sync` bounds which allow your trait to be used in multi-threaded programs.

When you do this, any type that wants to implement your trait is also unable to use `async fn`, and must apply a similar desugaring:

```rust
impl UserDatabase for Pool<Postgres> {
    fn get_user(&self, id: u64) -> impl Future<Output = Result<User, …>> + Send {
        async move {
            self.query(…).await
        }
    } 
}
```

Instead of doing this desugaring by hand, you can use `bitte`:

```rust
use bitte::bitte;

#[bitte]
pub trait UserDatabase {
    async fn get_user(&self, id: u64) -> Result<User, …>;
}

#[bitte]
impl UserDatabase for Pool<Postgres> {
    async fn get_user(&self, id: u64) -> Result<User, …> {
        // ...
    }
}
```

By default, Bitte won’t add any `Send` or `Sync` bounds; you can switch that default by enabling the `threads` feature, or individually by writing `#[bitte(Send, Sync)]`.

[impl-trait]: https://blog.rust-lang.org/2023/12/28/Rust-1.75.0/#async-fn-and-return-position-impl-trait-in-traits
[async_fn_in_trait]: https://doc.rust-lang.org/stable/nightly-rustc/rustc_lint/async_fn_in_trait/static.ASYNC_FN_IN_TRAIT.html
[tokio]: https://tokio.rs
[tokio-spawn]: https://docs.rs/tokio/latest/tokio/task/fn.spawn.html
[send-sync]: https://doc.rust-lang.org/nomicon/send-and-sync.html

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
bitte = "0.0.1"
```

For automatic `Send` + `Sync` bounds:

```toml
[dependencies]
bitte = { version = "0.0.1", features = ["threads"] }
```

## Usage

Apply `#[bitte]` to transform all async methods in a trait:

```rust
use bitte::bitte;

#[bitte]
trait AsyncRepository {
    async fn find_by_id(&self, id: u64) -> Option<String>;
    async fn save(&mut self, data: String) -> Result<u64, String>;
    
    // Non-async methods remain unchanged
    fn cache_size(&self) -> usize;
}
```

This transforms to:

```rust
trait AsyncRepository {
    fn find_by_id(&self, id: u64) -> impl std::future::Future<Output = Option<String>>;
    fn save(&mut self, data: String) -> impl std::future::Future<Output = Result<u64, String>>;
    
    fn cache_size(&self) -> usize;
}
```

### Implementation with `#[bitte]`

Apply `#[bitte]` to impl blocks to write natural async methods:

```rust
struct MyRepo {
    data: HashMap<u64, String>,
}

#[bitte]
impl AsyncRepository for MyRepo {
    async fn find_by_id(&self, id: u64) -> Option<String> {
        self.data.get(&id).cloned()
    }
    
    async fn save(&mut self, data: String) -> Result<u64, String> {
        let id = rand::random();
        self.data.insert(id, data);
        Ok(id)
    }
    
    fn cache_size(&self) -> usize {
        self.data.len()
    }
}
```

### Desugaring individual methods

Apply `#[bitte]` to specific methods:

```rust
trait AsyncMixedTrait {
    #[bitte]
    async fn transformed(&self) -> String;
    
    // This won’t be desugared to impl Trait
    async fn still_async(&self) -> String;
}
```

### Applying thread safety trait bounds

When the `threads` feature is enabled, `Send` + `Sync` bounds are automatically added:

```rust
#[bitte]  // With threads feature: adds Send + Sync
trait AsyncService {
    async fn process(&self, input: Vec<u8>) -> Vec<u8>;
}
```

Transforms to:

```rust
trait AsyncService {
    fn process(&self, input: Vec<u8>) -> impl std::future::Future<Output = Vec<u8>> + Send
    where
        Self: Sync;
}
```

#### Explicit `Send` and/or `Sync`

Override the default behavior:

```rust
#[bitte(Send, Sync)]  // Explicitly enable
trait AlwaysThreadSafe {
    async fn method(&self) -> u32;
}

#[bitte(?Send, ?Sync)]  // Explicitly disable
trait LocalOnly {
    async fn method(&self) -> u32;
}

// Mix and match per method
trait MixedBounds {
    #[bitte(?Send)]     // No Send bound
    async fn local_only(&self) -> u32;
    
    #[bitte(Send)]      // Force Send bound
    async fn thread_safe(&self) -> u32;
    
    #[bitte(?Send, ?Sync)]  // No bounds
    async fn no_bounds(&self) -> u32;
}
```

## Feature Flags

- `threads`: Add `Send` and/or `Sync` bounds to desugared trait and impl `fn`s

## Implementation

There are two ways to implement traits transformed by `bitte`:

### With `#[bitte]` on impl block (recommended)

Apply `#[bitte]` to your impl block to write natural async methods:

```rust
#[bitte]
impl AsyncRepository for MyRepo {
    async fn find_by_id(&self, id: u64) -> Option<String> {
        // Natural async syntax
        Some(format!("item-{}", id))
    }
    
    async fn save(&mut self, data: String) -> Result<u64, String> {
        // Your async implementation
        Ok(42)
    }
    
    fn cache_size(&self) -> usize {
        0
    }
}
```

### Manual implementation

You can also manually implement the desugared methods:

```rust
impl AsyncRepository for MyRepo {
    fn find_by_id(&self, id: u64) -> impl std::future::Future<Output = Option<String>> {
        async move {
            Some(format!("item-{}", id))
        }
    }
    
    fn save(&mut self, data: String) -> impl std::future::Future<Output = Result<u64, String>> {
        async move {
            Ok(42)
        }
    }
    
    fn cache_size(&self) -> usize {
        0
    }
}
```

## Comparison with async-trait

Prior to Rust 1.75, most code that needed `async` in traits used the [`async-trait`][async-trait] crate. 

```rust
use async_trait::async_trait;

#[async_trait]
pub trait UserDatabase {
    async fn get_user(&self, id: u64) -> Result<User, …>;
}
```

The `async_trait` macro also desugars `async fn`s, but turns them into a `Box<dyn Future>` instead:

```rust
pub trait UserDatabase {
    fn get_user<'async_trait>(
        &'async_trait self,
        id: u64,
    ) -> Pin<Box<dyn Future<Output = Result<User, …>> + Send + 'async_trait>>
    where
        Self: Sync + 'async_trait;
}
```

You may still want to use [async-trait][] – it’s not version 0.0.1, it’s already used in 8,000+ crates, its desugared traits are [`dyn`-compatible][dyn-compatible], it lets you support older Rust versions, and it handles references in trait `fn` parameters.

[async-trait]: https://lib.rs/async-trait
[dyn-compatible]: https://doc.rust-lang.org/reference/items/traits.html#dyn-compatibility
