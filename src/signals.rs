use nu_protocol::{Handlers, SignalAction, Signals, engine::EngineState};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

/// Set up SIGINT handling for nushell. Returns the signal-hook registration ID
/// and the handler closure, so the dispatcher can temporarily unregister the
/// handler during brush execution (to avoid conflicting with tokio::signal).
pub(crate) fn ctrlc_protection(
    engine_state: &mut EngineState,
) -> (signal_hook::SigId, Arc<dyn Fn() + Send + Sync>) {
    let interrupt = Arc::new(AtomicBool::new(false));
    engine_state.set_signals(Signals::new(interrupt.clone()));

    let signal_handlers = Handlers::new();

    // Register a handler to kill all background jobs on interrupt.
    signal_handlers
        .register_unguarded({
            let jobs = engine_state.jobs.clone();
            Box::new(move |action| {
                if action == SignalAction::Interrupt
                    && let Ok(mut jobs) = jobs.lock()
                {
                    let _ = jobs.kill_all();
                }
            })
        })
        .expect("Failed to register interrupt signal handler");

    engine_state.signal_handlers = Some(signal_handlers.clone());

    // Build the handler closure as an Arc so it can be re-registered later
    let handler: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
        interrupt.store(true, Ordering::Relaxed);
        signal_handlers.run(SignalAction::Interrupt);
    });

    // Register with signal-hook instead of ctrlc, so we get a SigId for unregistering
    let handler_clone = handler.clone();
    let sig_id = unsafe {
        signal_hook::low_level::register(signal_hook::consts::SIGINT, move || {
            handler_clone();
        })
    }
    .expect("Error setting Ctrl-C handler via signal-hook");

    (sig_id, handler)
}
