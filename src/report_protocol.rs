/// Report Protocol — progress tracking for long operations

use std::time::Instant;

#[derive(Debug, Clone, PartialEq)]
pub enum ReportState {
    Idle,
    Working { task: String, started: Instant },
    Checkpoint { task: String, started: Instant, note: String },
    Done { task: String, started: Instant, finished: Instant },
    Stuck { task: String, started: Instant, reason: String },
}

impl ReportState {
    pub fn is_idle(&self) -> bool {
        matches!(self, ReportState::Idle)
    }

    pub fn is_working(&self) -> bool {
        matches!(self, ReportState::Working { .. })
            || matches!(self, ReportState::Checkpoint { .. })
    }

    pub fn label(&self) -> &str {
        match self {
            ReportState::Idle => "idle",
            ReportState::Working { .. } => "working",
            ReportState::Checkpoint { .. } => "checkpoint",
            ReportState::Done { .. } => "done",
            ReportState::Stuck { .. } => "stuck",
        }
    }

    pub fn elapsed_secs(&self) -> Option<f64> {
        match self {
            ReportState::Idle => None,
            ReportState::Working { started, .. } => Some(started.elapsed().as_secs_f64()),
            ReportState::Checkpoint { started, .. } => Some(started.elapsed().as_secs_f64()),
            ReportState::Done { started, finished, .. } => {
                Some(finished.duration_since(*started).as_secs_f64())
            }
            ReportState::Stuck { started, .. } => Some(started.elapsed().as_secs_f64()),
        }
    }

    pub fn summary(&self) -> String {
        match self {
            ReportState::Idle => "idle".to_string(),
            ReportState::Working { task, .. } => format!("working: {}...", task),
            ReportState::Checkpoint { task, note, .. } => format!("checkpoint: {} @ {}", task, note),
            ReportState::Done { task, started, finished } => {
                let dur = finished.duration_since(*started).as_secs_f64();
                format!("done: {} ({:.1}s)", task, dur)
            }
            ReportState::Stuck { task, reason, .. } => format!("stuck: {} - {}", task, reason),
        }
    }
}

pub struct ReportProtocol {
    state: ReportState,
    history: Vec<ReportState>,
    send_fn: Option<Box<dyn Fn(String) + Send + Sync>>,
}

impl ReportProtocol {
    pub fn new() -> Self {
        Self {
            state: ReportState::Idle,
            history: Vec::with_capacity(32),
            send_fn: None,
        }
    }

    pub fn set_sender<F>(&mut self, f: F)
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        self.send_fn = Some(Box::new(f));
    }

    pub fn start(&mut self, task: &str) {
        let was_idle = self.state.is_idle();
        self.history.push(self.state.clone());
        self.state = ReportState::Working {
            task: task.to_string(),
            started: Instant::now(),
        };
        if was_idle {
            self.emit(format!("start: {}", task));
        }
    }

    pub fn checkpoint(&mut self, note: &str) {
        match &self.state {
            ReportState::Working { task, started } => {
                let task = task.clone();
                let started = *started;
                self.history.push(self.state.clone());
                self.state = ReportState::Checkpoint {
                    task,
                    started,
                    note: note.to_string(),
                };
                self.emit(format!("checkpoint: {}", note));
            }
            ReportState::Checkpoint { task, started, .. } => {
                let task = task.clone();
                let started = *started;
                self.history.push(self.state.clone());
                self.state = ReportState::Checkpoint {
                    task,
                    started,
                    note: note.to_string(),
                };
                self.emit(format!("checkpoint: {}", note));
            }
            _ => {
                self.start(note);
            }
        }
    }

    pub fn done(&mut self) {
        let (task_clone, started_clone) = match &self.state {
            ReportState::Working { task, started } => (task.clone(), *started),
            ReportState::Checkpoint { task, started, .. } => (task.clone(), *started),
            _ => return,
        };
        let finished = Instant::now();
        let dur = finished.duration_since(started_clone).as_secs_f64();
        self.history.push(self.state.clone());
        self.state = ReportState::Done {
            task: task_clone.clone(),
            started: started_clone,
            finished,
        };
        self.emit(format!("done: {} ({:.1}s)", task_clone, dur));
    }

    pub fn stuck(&mut self, reason: &str) {
        let (task_clone, started_clone) = match &self.state {
            ReportState::Working { task, started } => (task.clone(), *started),
            ReportState::Checkpoint { task, started, .. } => (task.clone(), *started),
            _ => {
                self.history.push(self.state.clone());
                self.state = ReportState::Stuck {
                    task: "unknown".to_string(),
                    started: Instant::now(),
                    reason: reason.to_string(),
                };
                self.emit(format!("stuck: {}", reason));
                return;
            }
        };
        self.history.push(self.state.clone());
        self.state = ReportState::Stuck {
            task: task_clone.clone(),
            started: started_clone,
            reason: reason.to_string(),
        };
        self.emit(format!("stuck: {} - {}", task_clone, reason));
    }

    pub fn reset(&mut self) {
        self.history.push(self.state.clone());
        self.state = ReportState::Idle;
    }

    pub fn state(&self) -> &ReportState {
        &self.state
    }

    pub fn recent_history(&self, n: usize) -> Vec<&ReportState> {
        self.history.iter().rev().take(n).collect()
    }

    pub fn status_line(&self) -> String {
        let elapsed = self.state.elapsed_secs()
            .map(|s| format!(" ({:.1}s)", s))
            .unwrap_or_default();
        format!("{}{}", self.state.summary(), elapsed)
    }

    fn emit(&self, msg: String) {
        println!("[report] {}", msg);
        if let Some(ref send) = self.send_fn {
            send(msg);
        }
    }
}
