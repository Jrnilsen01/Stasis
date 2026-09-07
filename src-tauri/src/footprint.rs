use std::sync::Mutex;
use std::time::Instant;

use serde::Serialize;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

/// Live measurement of this process. The whole product claim is "lighter than
/// the incumbent", so these numbers are measured on every poll and never cached
/// as a constant. A value the UI cannot measure is reported as `null`, not zero.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Footprint {
    pub pid: u32,
    /// Resident set size in bytes.
    pub memory_bytes: u64,
    /// Percent of one core. `None` until two samples exist, because a single
    /// sample cannot produce a rate.
    pub cpu_percent: Option<f32>,
    /// Milliseconds since the process started, so the UI can say how long this
    /// footprint has been sustained.
    pub uptime_ms: u128,
}

pub struct FootprintState {
    system: System,
    pid: Pid,
    started: Instant,
    sampled_once: bool,
}

impl FootprintState {
    pub fn new() -> Self {
        Self {
            system: System::new(),
            pid: Pid::from_u32(std::process::id()),
            started: Instant::now(),
            sampled_once: false,
        }
    }
}

#[tauri::command]
pub fn read_footprint(state: tauri::State<'_, Mutex<FootprintState>>) -> Result<Footprint, String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;
    let pid = state.pid;

    state.system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        true,
        ProcessRefreshKind::nothing().with_memory().with_cpu(),
    );

    let process = state
        .system
        .process(pid)
        .ok_or_else(|| format!("process {pid} not visible to the sampler"))?;

    let memory_bytes = process.memory();
    let cpu_percent = process.cpu_usage();

    // The first refresh has no previous sample to diff against, so its CPU
    // figure is meaningless. Report nothing rather than a misleading 0.0.
    let cpu_percent = if state.sampled_once {
        Some(cpu_percent)
    } else {
        state.sampled_once = true;
        None
    };

    Ok(Footprint {
        pid: pid.as_u32(),
        memory_bytes,
        cpu_percent,
        uptime_ms: state.started.elapsed().as_millis(),
    })
}
