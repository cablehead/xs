use crate::commands::CommandRegistry;
use crate::generators::GeneratorRegistry;
use crate::handlers::HandlerRegistry;
use crate::nu;
use crate::nu::ModuleRegistry;
use crate::store::{FollowOption, ReadOptions, Store};

pub async fn serve(
    store: Store,
    engine: nu::Engine,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut engine = engine;
    nu::add_core_commands(&mut engine, &store)?;
    engine.add_alias(".rm", ".remove")?;

    let mut modules = ModuleRegistry::new();
    let mut handlers = HandlerRegistry::new();
    let mut generators = GeneratorRegistry::new();
    let mut commands = CommandRegistry::new();

    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;

    // Phase 1: Historical replay
    // Modules register VFS entries eagerly so the engine accumulates module state.
    // Handlers and generators snapshot the engine at each .register/.spawn frame,
    // capturing exactly the modules available at that point in the stream.
    while let Some(frame) = recver.recv().await {
        if frame.topic == "xs.threshold" {
            break;
        }
        modules.process_historical(&frame, &mut engine, &store);
        handlers.process_historical(&frame, &engine);
        generators.process_historical(&frame, &engine);
        commands.process_historical(&frame, &engine);
    }

    // Phase 2: Materialize -- handlers and generators use their paired engine snapshots
    modules.materialize(&store, &mut engine).await?;
    handlers.materialize(&store).await?;
    generators.materialize(&store).await?;
    commands.materialize(&store).await?;

    // Phase 3: Live -- modules process before others for same reason
    while let Some(frame) = recver.recv().await {
        modules.process_live(&frame, &store, &mut engine).await?;
        handlers.process_live(&frame, &store, &engine).await?;
        generators.process_live(&frame, &store, &engine).await?;
        commands.process_live(&frame, &store, &engine).await?;
    }

    Ok(())
}
