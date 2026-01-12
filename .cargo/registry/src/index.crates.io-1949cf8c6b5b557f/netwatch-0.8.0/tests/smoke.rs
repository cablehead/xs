//! A very basic smoke test for netwatch, to make sure it doesn't error out immediately
//! in Wasm at all.
//!
//! We can't test browsers easily, because that would mean we need control over turning
//! the browser online/offline.
//!
//! However, this gives us a minimum guarantee that the Wasm build doesn't break fully.
use n0_watcher::Watcher;
use netwatch::netmon;
use testresult::TestResult;
#[cfg(not(wasm_browser))]
use tokio::test;
#[cfg(wasm_browser)]
use wasm_bindgen_test::wasm_bindgen_test as test;

// Enable this if you want to run these tests in the browser.
// Unfortunately it's either-or: Enable this and you can run in the browser, disable to run in nodejs.
// #[cfg(wasm_browser)]
// wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[test]
async fn smoke_test() -> TestResult {
    setup_logging();

    tracing::info!("Creating netmon::Monitor");
    let monitor = netmon::Monitor::new().await?;
    tracing::info!("netmon::Monitor created.");

    // Unfortunately this doesn't do anything in node.js, because it doesn't have
    // globalThis.navigator.onLine or globalThis.addEventListener("online"/"offline", ...) APIs,
    // so this is more of a test to see if we gracefully handle these situations & if our
    // .wasm files are without "env" imports.
    tracing::info!("subscribing to netmon");
    let mut sub = monitor.interface_state();

    let current = sub.get();
    tracing::info!(?current, "network change");
    tracing::info!("successfully subscribed to netmon");

    tracing::info!("dropping netmon::Monitor");
    drop(monitor);
    tracing::info!("dropped.");

    Ok(())
}

#[cfg(wasm_browser)]
fn setup_logging() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::level_filters::LevelFilter::DEBUG)
        .with_writer(
            // To avoide trace events in the browser from showing their JS backtrace
            tracing_subscriber_wasm::MakeConsoleWriter::default()
                .map_trace_level_to(tracing::Level::DEBUG),
        )
        // If we don't do this in the browser, we get a runtime error.
        .without_time()
        .with_ansi(false)
        .init();
}

#[cfg(not(wasm_browser))]
fn setup_logging() {
    tracing_subscriber::fmt().init();
}
