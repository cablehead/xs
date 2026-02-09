use crate::commands::CommandRegistry;
use crate::generators::GeneratorRegistry;
use crate::handlers::HandlerRegistry;
use crate::nu;
use crate::store::{FollowOption, ReadOptions, Store};

pub async fn serve(
    store: Store,
    engine: nu::Engine,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut engine = engine;
    nu::add_core_commands(&mut engine, &store)?;
    engine.add_alias(".rm", ".remove")?;

    let mut handlers = HandlerRegistry::new();
    let mut generators = GeneratorRegistry::new();
    let mut commands = CommandRegistry::new();

    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;

    // Phase 1: Historical replay
    while let Some(frame) = recver.recv().await {
        if frame.topic == "xs.threshold" {
            break;
        }
        handlers.process_historical(&frame);
        generators.process_historical(&frame);
        commands.process_historical(&frame, &engine, &store).await;
    }

    // Phase 2: Materialize
    handlers.materialize(&store, &engine).await?;
    generators.materialize(&store, &engine).await?;
    commands.materialize(&store, &engine).await?;

    // Phase 3: Live
    while let Some(frame) = recver.recv().await {
        handlers.process_live(&frame, &store, &engine).await?;
        generators.process_live(&frame, &store, &engine).await?;
        commands.process_live(&frame, &store, &engine).await?;
    }

    Ok(())
}
