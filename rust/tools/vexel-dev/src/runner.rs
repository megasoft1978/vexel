//! Process execution behind a small trait so tests can use a recording fake.
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    pub program: String,
    pub args: Vec<String>,
    pub envs: Vec<(String, String)>,
    pub cwd: Option<PathBuf>,
}

impl Invocation {
    pub fn new(program: &str) -> Self {
        Self {
            program: program.to_string(),
            args: Vec::new(),
            envs: Vec::new(),
            cwd: None,
        }
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.args
            .extend(args.into_iter().map(|a| a.as_ref().to_string()));
        self
    }

    pub fn arg_str(mut self, arg: &str) -> Self {
        self.args.push(arg.to_string());
        self
    }

    pub fn path_arg(mut self, path: &Path) -> Self {
        self.args.push(path.display().to_string());
        self
    }

    pub fn env(mut self, key: &str, value: impl AsRef<Path>) -> Self {
        self.envs
            .push((key.to_string(), value.as_ref().display().to_string()));
        self
    }

    pub fn in_dir(mut self, dir: &Path) -> Self {
        self.cwd = Some(dir.to_path_buf());
        self
    }

    /// `program arg arg`, for logs and assertions.
    pub fn line(&self) -> String {
        std::iter::once(self.program.as_str())
            .chain(self.args.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Output {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

pub trait Runner {
    fn run(&mut self, invocation: &Invocation) -> io::Result<Output>;
}

/// Runs real processes, echoing the command and its output to stdout so the
/// build log shows exactly what happened.
pub struct SystemRunner;

impl Runner for SystemRunner {
    fn run(&mut self, invocation: &Invocation) -> io::Result<Output> {
        println!("+ {}", invocation.line());
        let mut command = Command::new(&invocation.program);
        command.args(&invocation.args).envs(invocation.envs.clone());
        if let Some(dir) = &invocation.cwd {
            command.current_dir(dir);
        }
        let result = command.output()?;
        let output = Output {
            success: result.status.success(),
            stdout: String::from_utf8_lossy(&result.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&result.stderr).into_owned(),
        };
        for text in [&output.stdout, &output.stderr] {
            if !text.trim().is_empty() {
                println!("{}", text.trim_end());
            }
        }
        Ok(output)
    }
}

/// Runs `invocation` and turns a spawn failure or non-zero exit into an error.
pub fn run_checked(runner: &mut dyn Runner, invocation: &Invocation) -> Result<Output, String> {
    let output = runner
        .run(invocation)
        .map_err(|e| format!("could not start `{}`: {e}", invocation.line()))?;
    if output.success {
        Ok(output)
    } else {
        Err(format!(
            "`{}` failed: {}",
            invocation.line(),
            output.stderr.trim()
        ))
    }
}

#[cfg(test)]
pub mod fake {
    use super::*;

    type Handler = Box<dyn FnMut(&Invocation) -> Option<Output>>;

    /// Records every invocation; `handler` may fake an outcome (and side
    /// effects) per call, otherwise the call succeeds with empty output.
    pub struct RecordingRunner {
        pub calls: Vec<String>,
        pub handler: Handler,
    }

    impl RecordingRunner {
        pub fn new(handler: impl FnMut(&Invocation) -> Option<Output> + 'static) -> Self {
            Self {
                calls: Vec::new(),
                handler: Box::new(handler),
            }
        }
    }

    pub fn ok(stdout: &str) -> Output {
        Output {
            success: true,
            stdout: stdout.to_string(),
            stderr: String::new(),
        }
    }

    pub fn fail(stderr: &str) -> Output {
        Output {
            success: false,
            stdout: String::new(),
            stderr: stderr.to_string(),
        }
    }

    impl Runner for RecordingRunner {
        fn run(&mut self, invocation: &Invocation) -> io::Result<Output> {
            self.calls.push(invocation.line());
            Ok((self.handler)(invocation).unwrap_or_else(|| ok("")))
        }
    }

    #[test]
    fn run_checked_reports_the_failing_command() {
        let mut runner = RecordingRunner::new(|_| Some(fail("boom")));
        let error =
            run_checked(&mut runner, &Invocation::new("tool").args(["a", "b"])).unwrap_err();
        assert_eq!(error, "`tool a b` failed: boom");
        assert_eq!(runner.calls, ["tool a b"]);
    }
}
