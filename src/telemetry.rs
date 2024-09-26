use tracing::{subscriber::set_global_default, Subscriber};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{fmt::MakeWriter, layer::SubscriberExt, EnvFilter, Registry};

// NOTE: We're leveraging multiple `Layer`s of the `tracing-subscriber`
// crate that we compose in order to create a *processing pipeline* for
// our span data. Our pipeline will utilize *three* layers:
//
// 1. `tracing_subscriber::filter::EnvFilter` discards spans based on
//    their log levels (just like `env_logger` does) via `RUST_LOG`.
// 2. `tracing_bunyan_formatter::JsonStorageLayer` processes spans
//    and stores their information in JSON for downstream layers.
// 3. `tracing_bunyan_formatter::BunyanFormatterLayer` outputs logs
//    in `bunyan` (npm package) compatible JSON format.
//
// The layering is made possible using `tracing_subscriber::Registry`.
// It implements `Subscriber` but doesn't record traces itself.

// NOTE: The `tracing` crate provides a `log` feature which enables
// loggers of the `log` crate to log tracing events. The opposite is
// not true, there isn't a `tracing` feature for the `log` crate.
// Instead, we use the `tracing-log` adapter which makes it possible
// to redirect logs (for example created by `actix-web`) to our
// tracing subscriber.

/// Compose multiple layers into a `tracing`'s subscriber.
///
/// # Implementation Notes
///
/// We are using `impl Subscriber` as return type to avoid having to
/// spell out the actual type of the returned subscriber, which is
/// indeed quite complex.
/// We need to explicitly call out that the returned subscriber is
/// `Send` and `Sync` to make it possible to pass it to `init_subscriber`
/// later on.
/// We pass a sink to the function so that we can choose `std::io::sink`
/// for testing in order to send all logs to the void.
pub fn get_subscriber<Sink>(
    name: String,
    env_filter: String,
    sink: Sink,
) -> impl Subscriber + Send + Sync
where
    // This "weird" syntax is a higher-ranked trait bound (HRTB).
    // It basically means that `Sink` implements the `MakeWriter`
    // trait for all choices of the lifetime parameter `'a`.
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    // `tracing_subscriber::EnvFilter` filters the log levels from the
    // environment variable `RUST_LOG`. If it isn't set we fall back to
    // logs of the level *info* or above.
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));
    let formatting_layer = BunyanFormattingLayer::new(name, sink);
    // The `with` method is provided by `SubscriberExt`, an extension
    // trait for `Subscriber` exposed by `tracing_subscriber`. It allows
    // us to add the layers to the registry.
    Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}

/// Register a subscriber as global default to process span data.
///
/// It should only be called once!
pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    // Redirect all `log`'s events to our subscriber.
    LogTracer::init().expect("failed to set logger");
    // `set_global_default` can be used to specify what subscriber should
    // be used to process spans.
    set_global_default(subscriber).expect("failed to set subscriber");
}
