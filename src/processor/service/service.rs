use scru128::Scru128Id;
use tokio::task::JoinHandle;

use nu_protocol::{ByteStream, ByteStreamType, PipelineData, Signals, Span, Value};
use std::io::Read;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncReadExt;

use crate::nu;
use crate::nu::{value_to_json, ReturnOptions};
use crate::store::{FollowOption, Frame, ReadOptions, Store};
use serde_json::json;

#[derive(Clone, Debug, serde::Deserialize, Default)]
pub struct PtyOptions {
    pub cmd: String,
    pub cols: Option<u16>,
    pub rows: Option<u16>,
}

#[derive(Clone, Debug, serde::Deserialize, Default)]
pub struct ServiceScriptOptions {
    pub duplex: Option<bool>,
    pub return_options: Option<ReturnOptions>,
    pub pty: Option<PtyOptions>,
}

#[derive(Clone)]
pub struct ServiceLoop {
    pub topic: String,
}

#[derive(Clone)]
pub struct Task {
    pub id: Scru128Id,
    pub run_closure: nu_protocol::engine::Closure,
    pub return_options: Option<ReturnOptions>,
    pub duplex: bool,
    pub engine: nu::Engine,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone)]
pub enum ServiceEventKind {
    Running,
    /// output frame flushed; payload is raw bytes stored in CAS
    Recv {
        suffix: String,
        data: Vec<u8>,
    },
    /// output frame flushed; payload is a JSON record stored as frame metadata
    RecvMeta {
        suffix: String,
        meta: serde_json::Value,
    },
    Stopped(StopReason),
    ParseError {
        message: String,
    },
    Shutdown,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone)]
pub struct ServiceEvent {
    pub kind: ServiceEventKind,
    pub frame: Frame,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone)]
pub enum StopReason {
    Finished,
    Error { message: String },
    Terminate,
    Shutdown,
    Update { update_id: Scru128Id },
}

pub(crate) fn emit_event(
    store: &Store,
    loop_ctx: &ServiceLoop,
    source_id: Scru128Id,
    return_opts: Option<&ReturnOptions>,
    kind: ServiceEventKind,
) -> Result<ServiceEvent, Box<dyn std::error::Error + Send + Sync>> {
    let frame = match &kind {
        ServiceEventKind::Running => store.append(
            Frame::builder(format!("{topic}.running", topic = loop_ctx.topic))
                .meta(json!({ "source_id": source_id.to_string() }))
                .build(),
        )?,

        ServiceEventKind::Recv { suffix, data } => {
            let hash = store.cas_insert_bytes_sync(data)?;
            store.append(
                Frame::builder(format!(
                    "{topic}.{suffix}",
                    topic = loop_ctx.topic,
                    suffix = suffix
                ))
                .hash(hash)
                .maybe_ttl(return_opts.and_then(|o| o.ttl.clone()))
                .meta(json!({ "source_id": source_id.to_string() }))
                .build(),
            )?
        }

        ServiceEventKind::RecvMeta { suffix, meta } => {
            let mut merged = meta.clone();
            merged["source_id"] = json!(source_id.to_string());
            store.append(
                Frame::builder(format!(
                    "{topic}.{suffix}",
                    topic = loop_ctx.topic,
                    suffix = suffix
                ))
                .maybe_ttl(return_opts.and_then(|o| o.ttl.clone()))
                .meta(merged)
                .build(),
            )?
        }

        ServiceEventKind::Stopped(reason) => {
            let mut meta = json!({
                "source_id": source_id.to_string(),
                "reason": stop_reason_str(reason),
            });
            if let StopReason::Update { update_id } = reason {
                meta["update_id"] = json!(update_id.to_string());
            }
            if let StopReason::Error { message } = reason {
                meta["message"] = json!(message);
            }
            store.append(
                Frame::builder(format!("{topic}.stopped", topic = loop_ctx.topic))
                    .meta(meta)
                    .build(),
            )?
        }

        ServiceEventKind::ParseError { message } => store.append(
            Frame::builder(format!("{topic}.parse.error", topic = loop_ctx.topic))
                .meta(json!({
                    "source_id": source_id.to_string(),
                    "reason": message,
                }))
                .build(),
        )?,

        ServiceEventKind::Shutdown => store.append(
            Frame::builder(format!("{topic}.shutdown", topic = loop_ctx.topic))
                .meta(json!({ "source_id": source_id.to_string() }))
                .build(),
        )?,
    };

    Ok(ServiceEvent { kind, frame })
}

fn stop_reason_str(r: &StopReason) -> &'static str {
    match r {
        StopReason::Finished => "finished",
        StopReason::Error { .. } => "error",
        StopReason::Terminate => "terminate",
        StopReason::Shutdown => "shutdown",
        StopReason::Update { .. } => "update",
    }
}

pub fn spawn(store: Store, spawn_frame: Frame) -> JoinHandle<()> {
    tokio::spawn(async move { run(store, spawn_frame).await })
}

async fn read_spawn_script(store: &Store, spawn_frame: &Frame) -> Option<String> {
    let hash = spawn_frame.hash.clone()?;
    let mut reader = store.cas_reader(hash).await.ok()?;
    let mut script = String::new();
    reader.read_to_string(&mut script).await.ok()?;
    Some(script)
}

fn make_loop_ctx(spawn_frame: &Frame) -> ServiceLoop {
    ServiceLoop {
        topic: spawn_frame
            .topic
            .strip_suffix(".spawn")
            .unwrap_or(&spawn_frame.topic)
            .to_string(),
    }
}

async fn run(store: Store, spawn_frame: Frame) {
    let script = match read_spawn_script(&store, &spawn_frame).await {
        Some(s) => s,
        None => return,
    };

    let loop_ctx = make_loop_ctx(&spawn_frame);

    // Evaluate the script to get the config value. Use eval_script so we can
    // inspect the value before deciding whether this is a PTY or closure service.
    let mut engine = match crate::processor::build_engine(&store, &spawn_frame.id) {
        Ok(e) => e,
        Err(_) => return,
    };

    let config_value = match nu::eval_script(&mut engine, &script) {
        Ok(v) => v,
        Err(e) => {
            let _ = emit_event(
                &store,
                &loop_ctx,
                spawn_frame.id,
                None,
                ServiceEventKind::ParseError {
                    message: e.to_string(),
                },
            );
            return;
        }
    };

    let opts: ServiceScriptOptions = match serde_json::from_value(nu::value_to_json(&config_value))
    {
        Ok(o) => o,
        Err(e) => {
            let _ = emit_event(
                &store,
                &loop_ctx,
                spawn_frame.id,
                None,
                ServiceEventKind::ParseError {
                    message: e.to_string(),
                },
            );
            return;
        }
    };

    if let Some(pty_opts) = opts.pty {
        run_pty_loop(store, loop_ctx, spawn_frame.id, pty_opts).await;
        return;
    }

    // Closure-based service: extract the run closure
    let run_val = match config_value.get_data_by_key("run") {
        Some(v) => v,
        None => {
            let _ = emit_event(
                &store,
                &loop_ctx,
                spawn_frame.id,
                None,
                ServiceEventKind::ParseError {
                    message: "Script must define a 'run' closure or 'pty' options.".into(),
                },
            );
            return;
        }
    };
    let run_closure = match run_val.as_closure() {
        Ok(c) => c.clone(),
        Err(e) => {
            let _ = emit_event(
                &store,
                &loop_ctx,
                spawn_frame.id,
                None,
                ServiceEventKind::ParseError {
                    message: format!("'run' field must be a closure: {e}"),
                },
            );
            return;
        }
    };

    // Create and set the interrupt signal on the engine state
    let interrupt = Arc::new(AtomicBool::new(false));
    engine.state.set_signals(Signals::new(interrupt.clone()));

    let task = Task {
        id: spawn_frame.id,
        run_closure,
        return_options: opts.return_options,
        duplex: opts.duplex.unwrap_or(false),
        engine,
    };

    run_loop(store, loop_ctx, task).await;
}

async fn run_loop(store: Store, loop_ctx: ServiceLoop, mut task: Task) {
    // Create the first start frame and set up a persistent control subscription
    let start_event = emit_event(
        &store,
        &loop_ctx,
        task.id,
        task.return_options.as_ref(),
        ServiceEventKind::Running,
    )
    .expect("failed to emit running event");
    let mut start_id = start_event.frame.id;

    let control_rx_options = ReadOptions::builder()
        .follow(FollowOption::On)
        .after(start_id)
        .build();

    let mut control_rx = store.read(control_rx_options).await;

    enum LoopOutcome {
        Continue,
        Update(Box<Task>, Scru128Id),
        Terminate,
        Shutdown,
        Error(String),
    }

    impl core::fmt::Debug for LoopOutcome {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                LoopOutcome::Continue => write!(f, "Continue"),
                LoopOutcome::Update(_, id) => f.debug_tuple("Update").field(id).finish(),
                LoopOutcome::Terminate => write!(f, "Terminate"),
                LoopOutcome::Shutdown => write!(f, "Shutdown"),
                LoopOutcome::Error(e) => f.debug_tuple("Error").field(e).finish(),
            }
        }
    }

    impl From<&LoopOutcome> for StopReason {
        fn from(value: &LoopOutcome) -> Self {
            match value {
                LoopOutcome::Continue => StopReason::Finished,
                LoopOutcome::Update(_, id) => StopReason::Update { update_id: *id },
                LoopOutcome::Terminate => StopReason::Terminate,
                LoopOutcome::Shutdown => StopReason::Shutdown,
                LoopOutcome::Error(e) => StopReason::Error { message: e.clone() },
            }
        }
    }

    loop {
        let input_pipeline = if task.duplex {
            let options = ReadOptions::builder()
                .follow(FollowOption::On)
                .after(start_id)
                .build();
            let send_rx = store.read(options).await;
            build_input_pipeline(store.clone(), &loop_ctx, &task, send_rx).await
        } else {
            PipelineData::empty()
        };

        let (done_tx, done_rx) = tokio::sync::oneshot::channel();
        spawn_thread(
            store.clone(),
            loop_ctx.clone(),
            task.clone(),
            input_pipeline,
            done_tx,
        );

        let terminate_topic = format!("{topic}.terminate", topic = loop_ctx.topic);
        let spawn_topic = format!("{topic}.spawn", topic = loop_ctx.topic);
        tokio::pin!(done_rx);

        let outcome = 'ctrl: loop {
            tokio::select! {
                biased;
                maybe = control_rx.recv() => {
                    match maybe {
                        Some(frame) if frame.topic == terminate_topic => {
                            task.engine.state.signals().trigger();
                            task.engine.kill_job_by_name(&task.id.to_string());
                            let _ = (&mut done_rx).await;
                            break 'ctrl LoopOutcome::Terminate;
                        }
                        Some(frame) if frame.topic == "xs.stopping" => {
                            task.engine.state.signals().trigger();
                            task.engine.kill_job_by_name(&task.id.to_string());
                            let _ = (&mut done_rx).await;
                            break 'ctrl LoopOutcome::Shutdown;
                        }
                        Some(frame) if frame.topic == spawn_topic => {
                            if let Some(hash) = frame.hash.clone() {
                                if let Ok(mut reader) = store.cas_reader(hash).await {
                                    let mut script = String::new();
                                    if reader.read_to_string(&mut script).await.is_ok() {
                                        let mut new_engine = match crate::processor::build_engine(&store, &frame.id) {
                                            Ok(e) => e,
                                            Err(_) => continue,
                                        };
                                        match nu::parse_config(&mut new_engine, &script) {
                                            Ok(cfg) => {
                                                let opts: ServiceScriptOptions = cfg.deserialize_options().unwrap_or_default();
                                                let interrupt = Arc::new(AtomicBool::new(false));
                                                new_engine.state.set_signals(Signals::new(interrupt.clone()));

                                                task.engine.state.signals().trigger();
                                                task.engine.kill_job_by_name(&task.id.to_string());
                                                let _ = (&mut done_rx).await;

                                                let new_task = Task {
                                                    id: frame.id,
                                                    run_closure: cfg.run_closure,
                                                    return_options: opts.return_options,
                                                    duplex: opts.duplex.unwrap_or(false),
                                                    engine: new_engine,
                                                };

                                                break 'ctrl LoopOutcome::Update(Box::new(new_task), frame.id);
                                            }
                                            Err(e) => {
                                                let _ = emit_event(
                                                    &store,
                                                    &loop_ctx,
                                                    frame.id,
                                                    None,
                                                    ServiceEventKind::ParseError { message: e.to_string() },
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Some(_) => {}
                        None => break 'ctrl LoopOutcome::Error("control".into()),
                    }
                }
                res = &mut done_rx => {
                    break 'ctrl match res.unwrap_or(Err("thread failed".into())) {
                        Ok(()) => LoopOutcome::Continue,
                        Err(e) => LoopOutcome::Error(e),
                    };
                }
            }
        };

        let reason: StopReason = (&outcome).into();
        let _ = emit_event(
            &store,
            &loop_ctx,
            task.id,
            task.return_options.as_ref(),
            ServiceEventKind::Stopped(reason.clone()),
        );

        match outcome {
            LoopOutcome::Continue => {
                tokio::time::sleep(Duration::from_secs(1)).await;
                if let Ok(event) = emit_event(
                    &store,
                    &loop_ctx,
                    task.id,
                    task.return_options.as_ref(),
                    ServiceEventKind::Running,
                ) {
                    start_id = event.frame.id;
                }
            }
            LoopOutcome::Update(new_task, _) => {
                task = *new_task;
                if let Ok(event) = emit_event(
                    &store,
                    &loop_ctx,
                    task.id,
                    task.return_options.as_ref(),
                    ServiceEventKind::Running,
                ) {
                    start_id = event.frame.id;
                }
            }
            LoopOutcome::Terminate | LoopOutcome::Shutdown | LoopOutcome::Error(_) => {
                let _ = emit_event(
                    &store,
                    &loop_ctx,
                    task.id,
                    task.return_options.as_ref(),
                    ServiceEventKind::Shutdown,
                );
                break;
            }
        }
    }
}

#[cfg(unix)]
async fn run_pty_loop(
    store: Store,
    loop_ctx: ServiceLoop,
    source_id: Scru128Id,
    initial_pty_opts: PtyOptions,
) {
    use nix::libc;
    use nix::pty::openpty;
    use nix::sys::signal::{kill, Signal};
    use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
    use nix::unistd::{close, dup2, execvp, fork, setsid, ForkResult, Pid};
    use std::ffi::CString;
    use std::os::unix::io::{FromRawFd, IntoRawFd};

    fn spawn_pty_child(opts: &PtyOptions) -> Result<(std::os::unix::io::RawFd, Pid), String> {
        let cols = opts.cols.unwrap_or(80);
        let rows = opts.rows.unwrap_or(24);

        let ws = libc::winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };

        let pty = openpty(Some(&ws), None).map_err(|e| format!("openpty: {e}"))?;

        // Convert OwnedFd to raw fds before fork. After fork both processes
        // need to close fds manually, so raw fds are simpler and safer.
        let master_fd = pty.master.into_raw_fd();
        let slave_fd = pty.slave.into_raw_fd();

        match unsafe { fork() } {
            Ok(ForkResult::Child) => {
                let _ = close(master_fd);
                let _ = setsid();

                // Set controlling terminal
                unsafe {
                    libc::ioctl(slave_fd, libc::TIOCSCTTY as _, 0);
                }

                let _ = dup2(slave_fd, 0);
                let _ = dup2(slave_fd, 1);
                let _ = dup2(slave_fd, 2);
                if slave_fd > 2 {
                    let _ = close(slave_fd);
                }

                let cmd =
                    CString::new(opts.cmd.as_str()).unwrap_or_else(|_| CString::new("sh").unwrap());
                let args = [cmd.clone()];
                let _ = execvp(&cmd, &args);
                std::process::exit(1);
            }
            Ok(ForkResult::Parent { child }) => {
                let _ = close(slave_fd);
                Ok((master_fd, child))
            }
            Err(e) => Err(format!("fork: {e}")),
        }
    }

    fn kill_child(pid: Pid) {
        let _ = kill(pid, Signal::SIGTERM);
        // Give the child a moment to exit, then force kill
        std::thread::sleep(std::time::Duration::from_millis(50));
        if let Ok(WaitStatus::StillAlive) = waitpid(pid, Some(WaitPidFlag::WNOHANG)) {
            let _ = kill(pid, Signal::SIGKILL);
            let _ = waitpid(pid, None);
        }
    }

    let start_event = emit_event(
        &store,
        &loop_ctx,
        source_id,
        None,
        ServiceEventKind::Running,
    )
    .expect("failed to emit running event");
    let mut start_id = start_event.frame.id;

    let control_rx_options = ReadOptions::builder()
        .follow(FollowOption::On)
        .after(start_id)
        .build();
    let mut control_rx = store.read(control_rx_options).await;

    let mut current_id = source_id;
    let mut pty_opts = initial_pty_opts;

    loop {
        let (master_fd, child_pid) = match spawn_pty_child(&pty_opts) {
            Ok(v) => v,
            Err(e) => {
                let _ = emit_event(
                    &store,
                    &loop_ctx,
                    current_id,
                    None,
                    ServiceEventKind::Stopped(StopReason::Error { message: e }),
                );
                let _ = emit_event(
                    &store,
                    &loop_ctx,
                    current_id,
                    None,
                    ServiceEventKind::Shutdown,
                );
                return;
            }
        };

        // Safety: we own master_fd after fork, the child closed its copy
        let master_file = unsafe { std::fs::File::from_raw_fd(master_fd) };
        let master_read = tokio::fs::File::from_std(master_file.try_clone().unwrap());
        let master_write = tokio::fs::File::from_std(master_file);

        // Spawn task: read master fd -> emit recv frames
        let recv_store = store.clone();
        let recv_ctx = loop_ctx.clone();
        let recv_id = current_id;
        let (child_done_tx, child_done_rx) = tokio::sync::oneshot::channel::<()>();
        let recv_handle = tokio::spawn(async move {
            let mut reader = master_read;
            let mut buf = [0u8; 8192];
            loop {
                match reader.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        let _ = emit_event(
                            &recv_store,
                            &recv_ctx,
                            recv_id,
                            None,
                            ServiceEventKind::Recv {
                                suffix: "recv".into(),
                                data: buf[..n].to_vec(),
                            },
                        );
                    }
                    Err(_) => break,
                }
            }
            let _ = child_done_tx.send(());
        });

        // Spawn task: read send frames -> write to master fd
        let send_store = store.clone();
        let send_topic = format!("{}.send", loop_ctx.topic);
        let send_options = ReadOptions::builder()
            .follow(FollowOption::On)
            .after(start_id)
            .build();
        let send_rx = send_store.read(send_options).await;

        use tokio::io::AsyncWriteExt;
        let send_handle = tokio::spawn(async move {
            let mut writer = master_write;
            let mut rx = send_rx;
            while let Some(frame) = rx.recv().await {
                if frame.topic != send_topic {
                    continue;
                }
                if let Some(hash) = frame.hash {
                    if let Ok(bytes) = send_store.cas_read(&hash).await {
                        if writer.write_all(&bytes).await.is_err() {
                            break;
                        }
                        if writer.flush().await.is_err() {
                            break;
                        }
                    }
                }
            }
        });

        let terminate_topic = format!("{}.terminate", loop_ctx.topic);
        let spawn_topic = format!("{}.spawn", loop_ctx.topic);
        let resize_topic = format!("{}.resize", loop_ctx.topic);
        tokio::pin!(child_done_rx);

        enum PtyOutcome {
            ChildExited,
            Terminate,
            Shutdown,
            Update(Scru128Id, PtyOptions),
            Error(String),
        }

        let outcome = 'pty_ctrl: loop {
            tokio::select! {
                biased;
                maybe = control_rx.recv() => {
                    match maybe {
                        Some(frame) if frame.topic == terminate_topic => {
                            kill_child(child_pid);
                            let _ = (&mut child_done_rx).await;
                            break 'pty_ctrl PtyOutcome::Terminate;
                        }
                        Some(frame) if frame.topic == "xs.stopping" => {
                            kill_child(child_pid);
                            let _ = (&mut child_done_rx).await;
                            break 'pty_ctrl PtyOutcome::Shutdown;
                        }
                        Some(frame) if frame.topic == spawn_topic => {
                            if let Some(hash) = frame.hash.clone() {
                                if let Ok(bytes) = store.cas_read(&hash).await {
                                    if let Ok(script) = String::from_utf8(bytes) {
                                        let mut new_engine = match crate::processor::build_engine(&store, &frame.id) {
                                            Ok(e) => e,
                                            Err(_) => continue,
                                        };
                                        match nu::eval_script(&mut new_engine, &script) {
                                            Ok(val) => {
                                                let new_opts: ServiceScriptOptions =
                                                    match serde_json::from_value(nu::value_to_json(&val)) {
                                                        Ok(o) => o,
                                                        Err(e) => {
                                                            let _ = emit_event(
                                                                &store,
                                                                &loop_ctx,
                                                                frame.id,
                                                                None,
                                                                ServiceEventKind::ParseError { message: e.to_string() },
                                                            );
                                                            continue;
                                                        }
                                                    };
                                                if let Some(new_pty) = new_opts.pty {
                                                    kill_child(child_pid);
                                                    let _ = (&mut child_done_rx).await;
                                                    break 'pty_ctrl PtyOutcome::Update(frame.id, new_pty);
                                                }
                                                // New config is not PTY -- treat as terminate
                                                // so serve.rs can restart with the new config type.
                                                // This is an edge case (switching PTY -> closure).
                                                kill_child(child_pid);
                                                let _ = (&mut child_done_rx).await;
                                                break 'pty_ctrl PtyOutcome::Terminate;
                                            }
                                            Err(e) => {
                                                let _ = emit_event(
                                                    &store,
                                                    &loop_ctx,
                                                    frame.id,
                                                    None,
                                                    ServiceEventKind::ParseError { message: e.to_string() },
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Some(frame) if frame.topic == resize_topic => {
                            if let Some(ref meta) = frame.meta {
                                let cols = meta.get("cols").and_then(|v| v.as_u64()).unwrap_or(80) as u16;
                                let rows = meta.get("rows").and_then(|v| v.as_u64()).unwrap_or(24) as u16;
                                let ws = libc::winsize {
                                    ws_row: rows,
                                    ws_col: cols,
                                    ws_xpixel: 0,
                                    ws_ypixel: 0,
                                };
                                unsafe {
                                    libc::ioctl(master_fd, libc::TIOCSWINSZ as libc::c_ulong, &ws);
                                }
                                let _ = kill(child_pid, Signal::SIGWINCH);
                            }
                        }
                        Some(_) => {}
                        None => break 'pty_ctrl PtyOutcome::Error("control channel closed".into()),
                    }
                }
                _ = &mut child_done_rx => {
                    break 'pty_ctrl PtyOutcome::ChildExited;
                }
            }
        };

        recv_handle.abort();
        send_handle.abort();

        let stop_reason = match &outcome {
            PtyOutcome::ChildExited => StopReason::Finished,
            PtyOutcome::Terminate => StopReason::Terminate,
            PtyOutcome::Shutdown => StopReason::Shutdown,
            PtyOutcome::Update(update_id, _) => StopReason::Update {
                update_id: *update_id,
            },
            PtyOutcome::Error(e) => StopReason::Error { message: e.clone() },
        };

        let _ = emit_event(
            &store,
            &loop_ctx,
            current_id,
            None,
            ServiceEventKind::Stopped(stop_reason),
        );

        match outcome {
            PtyOutcome::ChildExited => {
                // Restart after delay, like closure services
                tokio::time::sleep(Duration::from_secs(1)).await;
                if let Ok(event) = emit_event(
                    &store,
                    &loop_ctx,
                    current_id,
                    None,
                    ServiceEventKind::Running,
                ) {
                    start_id = event.frame.id;
                }
            }
            PtyOutcome::Update(new_id, new_pty) => {
                current_id = new_id;
                pty_opts = new_pty;
                if let Ok(event) = emit_event(
                    &store,
                    &loop_ctx,
                    current_id,
                    None,
                    ServiceEventKind::Running,
                ) {
                    start_id = event.frame.id;
                }
            }
            PtyOutcome::Terminate | PtyOutcome::Shutdown | PtyOutcome::Error(_) => {
                let _ = emit_event(
                    &store,
                    &loop_ctx,
                    current_id,
                    None,
                    ServiceEventKind::Shutdown,
                );
                break;
            }
        }
    }
}

async fn build_input_pipeline(
    store: Store,
    loop_ctx: &ServiceLoop,
    task: &Task,
    rx: tokio::sync::mpsc::Receiver<Frame>,
) -> PipelineData {
    let topic = format!("{loop_topic}.send", loop_topic = loop_ctx.topic);
    let signals = task.engine.state.signals().clone();
    let mut rx = rx;
    let iter = std::iter::from_fn(move || loop {
        if signals.interrupted() {
            return None;
        }

        match rx.try_recv() {
            Ok(frame) => {
                if frame.topic == topic {
                    if let Some(hash) = frame.hash {
                        if let Ok(bytes) = store.cas_read_sync(&hash) {
                            if let Ok(content) = String::from_utf8(bytes) {
                                return Some(content);
                            }
                        }
                    }
                }
            }
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                std::thread::sleep(std::time::Duration::from_millis(10));
                continue;
            }
            Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                return None;
            }
        }
    });

    ByteStream::from_iter(
        iter,
        Span::unknown(),
        task.engine.state.signals().clone(),
        ByteStreamType::Unknown,
    )
    .into()
}

fn spawn_thread(
    store: Store,
    loop_ctx: ServiceLoop,
    mut task: Task,
    input_pipeline: PipelineData,
    done_tx: tokio::sync::oneshot::Sender<Result<(), String>>,
) {
    let handle = tokio::runtime::Handle::current();
    std::thread::spawn(move || {
        let res = run_pipeline(&handle, &store, &loop_ctx, &mut task, input_pipeline);
        let _ = done_tx.send(res);
    });
}

fn run_pipeline(
    handle: &tokio::runtime::Handle,
    store: &Store,
    loop_ctx: &ServiceLoop,
    task: &mut Task,
    input_pipeline: PipelineData,
) -> Result<(), String> {
    let pipeline = task
        .engine
        .run_closure_in_job(
            &task.run_closure,
            vec![],
            Some(input_pipeline),
            task.id.to_string(),
        )
        .map_err(|e| {
            let working_set = nu_protocol::engine::StateWorkingSet::new(&task.engine.state);
            nu_protocol::format_cli_error(None, &working_set, &*e, None)
        })?;

    let suffix = task
        .return_options
        .as_ref()
        .and_then(|o| o.suffix.clone())
        .unwrap_or_else(|| "recv".into());
    let use_cas = task
        .return_options
        .as_ref()
        .and_then(|o| o.target.as_deref())
        .is_some_and(|t| t == "cas");

    let emit = |event| {
        handle.block_on(async {
            let _ = emit_event(
                store,
                loop_ctx,
                task.id,
                task.return_options.as_ref(),
                event,
            );
        });
    };

    match pipeline {
        PipelineData::Empty => {}
        PipelineData::Value(value, _) => {
            if let Some(event) = value_to_event(&value, &suffix, use_cas)? {
                emit(event);
            }
        }
        PipelineData::ListStream(mut stream, _) => {
            while let Some(value) = stream.next_value() {
                if let Some(event) = value_to_event(&value, &suffix, use_cas)? {
                    emit(event);
                }
            }
        }
        PipelineData::ByteStream(stream, _) => {
            if let Some(mut reader) = stream.reader() {
                let mut buf = [0u8; 8192];
                loop {
                    match reader.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            emit(ServiceEventKind::Recv {
                                suffix: suffix.clone(),
                                data: buf[..n].to_vec(),
                            });
                        }
                        Err(_) => break,
                    }
                }
            }
        }
    }
    Ok(())
}

fn value_to_event(
    value: &Value,
    suffix: &str,
    use_cas: bool,
) -> Result<Option<ServiceEventKind>, String> {
    match value {
        Value::Nothing { .. } => Ok(None),
        Value::Record { .. } if !use_cas => Ok(Some(ServiceEventKind::RecvMeta {
            suffix: suffix.to_string(),
            meta: value_to_json(value),
        })),
        _ if use_cas => {
            let data = match value {
                Value::String { val, .. } => val.as_bytes().to_vec(),
                Value::Binary { val, .. } => val.clone(),
                _ => value_to_json(value).to_string().into_bytes(),
            };
            Ok(Some(ServiceEventKind::Recv {
                suffix: suffix.to_string(),
                data,
            }))
        }
        _ => Err(format!(
            "Service output must be a record when target is not \"cas\"; got {}. \
             Set return_options.target to \"cas\" for non-record output.",
            value.get_type()
        )),
    }
}
