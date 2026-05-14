use super::env::{CommandEnv, CommandEnvs, CommandResolvedEnvs};
pub use crate::ffi::OsString as EnvKey;
use crate::ffi::{CString, OsStr, OsString};
use crate::num::NonZero;
use crate::path::Path;
use crate::process::StdioPipes;
use crate::sys::fs::File;
use crate::sys::pal::process as pal_process;
use crate::{fmt, io};

////////////////////////////////////////////////////////////////////////////////
// Command
////////////////////////////////////////////////////////////////////////////////

pub struct Command {
    program: OsString,
    args: Vec<OsString>,
    env: CommandEnv,

    cwd: Option<OsString>,
    stdin: Option<Stdio>,
    stdout: Option<Stdio>,
    stderr: Option<Stdio>,
}

#[derive(Debug)]
pub enum Stdio {
    Inherit,
    Null,
    MakePipe,
    ParentStdout,
    ParentStderr,
    #[allow(dead_code)] // This variant exists only for the Debug impl
    InheritFile(File),
}

impl Command {
    pub fn new(program: &OsStr) -> Command {
        Command {
            program: program.to_owned(),
            args: Vec::new(),
            env: Default::default(),
            cwd: None,
            stdin: None,
            stdout: None,
            stderr: None,
        }
    }

    pub fn arg(&mut self, arg: &OsStr) {
        self.args.push(arg.to_owned());
    }

    pub fn env_mut(&mut self) -> &mut CommandEnv {
        &mut self.env
    }

    pub fn cwd(&mut self, dir: &OsStr) {
        self.cwd = Some(dir.to_owned());
    }

    pub fn stdin(&mut self, stdin: Stdio) {
        self.stdin = Some(stdin);
    }

    pub fn stdout(&mut self, stdout: Stdio) {
        self.stdout = Some(stdout);
    }

    pub fn stderr(&mut self, stderr: Stdio) {
        self.stderr = Some(stderr);
    }

    pub fn get_program(&self) -> &OsStr {
        &self.program
    }

    pub fn get_args(&self) -> CommandArgs<'_> {
        CommandArgs { iter: self.args.iter() }
    }

    pub fn get_envs(&self) -> CommandEnvs<'_> {
        self.env.iter()
    }

    pub fn get_env_clear(&self) -> bool {
        self.env.does_clear()
    }

    pub fn get_resolved_envs(&self) -> CommandResolvedEnvs {
        CommandResolvedEnvs::new(self.env.capture())
    }

    pub fn get_current_dir(&self) -> Option<&Path> {
        self.cwd.as_ref().map(|cwd| Path::new(cwd))
    }

    pub fn spawn(
        &mut self,
        _default: Stdio,
        _needs_stdin: bool,
    ) -> io::Result<(Process, StdioPipes)> {
        self.validate_supported_launch()?;

        let path = cstring_from_os_str(&self.program)?;
        let mut arg_storage = Vec::new();
        let mut argv = Vec::new();

        if !self.args.is_empty() {
            arg_storage.push(cstring_from_os_str(&self.program)?);
            for arg in &self.args {
                arg_storage.push(cstring_from_os_str(arg)?);
            }
            argv.extend(arg_storage.iter().map(|arg| arg.as_ptr().cast_mut()));
        }

        let pid = pal_process::spawn(&path, &mut argv)?;
        let id = pal_process::register_task(pid);

        Ok((
            Process { pid, id, status: None },
            StdioPipes { stdin: None, stdout: None, stderr: None },
        ))
    }

    fn validate_supported_launch(&self) -> io::Result<()> {
        if !self.env.is_unchanged()
            || self.cwd.is_some()
            || self.stdin.is_some()
            || self.stdout.is_some()
            || self.stderr.is_some()
        {
            Err(io::Error::UNSUPPORTED_PLATFORM)
        } else {
            Ok(())
        }
    }
}

pub fn output(_cmd: &mut Command) -> io::Result<(ExitStatus, Vec<u8>, Vec<u8>)> {
    Err(io::Error::UNSUPPORTED_PLATFORM)
}

impl From<ChildPipe> for Stdio {
    fn from(pipe: ChildPipe) -> Stdio {
        pipe.diverge()
    }
}

impl From<io::Stdout> for Stdio {
    fn from(_: io::Stdout) -> Stdio {
        Stdio::ParentStdout
    }
}

impl From<io::Stderr> for Stdio {
    fn from(_: io::Stderr) -> Stdio {
        Stdio::ParentStderr
    }
}

impl From<File> for Stdio {
    fn from(file: File) -> Stdio {
        Stdio::InheritFile(file)
    }
}

impl fmt::Debug for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug_command = f.debug_struct("Command");
        debug_command.field("program", &self.program).field("args", &self.args);
        if !self.env.is_unchanged() {
            debug_command.field("env", &self.env);
        }
        if self.cwd.is_some() {
            debug_command.field("cwd", &self.cwd);
        }
        if self.stdin.is_some() {
            debug_command.field("stdin", &self.stdin);
        }
        if self.stdout.is_some() {
            debug_command.field("stdout", &self.stdout);
        }
        if self.stderr.is_some() {
            debug_command.field("stderr", &self.stderr);
        }
        debug_command.finish()
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Default)]
pub struct ExitStatus();

impl ExitStatus {
    pub fn exit_ok(&self) -> Result<(), ExitStatusError> {
        Ok(())
    }

    pub fn code(&self) -> Option<i32> {
        Some(0)
    }
}

impl fmt::Display for ExitStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "exit code: 0")
    }
}

pub struct ExitStatusError(!);

impl Clone for ExitStatusError {
    fn clone(&self) -> ExitStatusError {
        self.0
    }
}

impl Copy for ExitStatusError {}

impl PartialEq for ExitStatusError {
    fn eq(&self, _other: &ExitStatusError) -> bool {
        self.0
    }
}

impl Eq for ExitStatusError {}

impl fmt::Debug for ExitStatusError {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0
    }
}

impl Into<ExitStatus> for ExitStatusError {
    fn into(self) -> ExitStatus {
        self.0
    }
}

impl ExitStatusError {
    pub fn code(self) -> Option<NonZero<i32>> {
        self.0
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct ExitCode(u8);

impl ExitCode {
    pub const SUCCESS: ExitCode = ExitCode(0);
    pub const FAILURE: ExitCode = ExitCode(1);

    pub fn as_i32(&self) -> i32 {
        self.0 as i32
    }
}

impl From<u8> for ExitCode {
    fn from(code: u8) -> Self {
        Self(code)
    }
}

pub struct Process {
    pid: pal_process::Pid,
    id: pal_process::TaskId,
    status: Option<ExitStatus>,
}

impl Process {
    pub fn id(&self) -> u32 {
        self.pid as u32
    }

    pub fn kill(&mut self) -> io::Result<()> {
        Err(io::Error::UNSUPPORTED_PLATFORM)
    }

    pub fn wait(&mut self) -> io::Result<ExitStatus> {
        if let Some(status) = self.status {
            return Ok(status);
        }

        loop {
            pal_process::wait_for_task(self.id)?;
            return Ok(self.record_completed());
        }
    }

    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        if let Some(status) = self.status {
            return Ok(Some(status));
        }

        if pal_process::try_wait_for_task(self.id)? {
            return Ok(Some(self.record_completed()));
        }
        Ok(None)
    }

    fn record_completed(&mut self) -> ExitStatus {
        let status = ExitStatus();
        self.status = Some(status);
        status
    }
}

pub struct CommandArgs<'a> {
    iter: crate::slice::Iter<'a, OsString>,
}

impl<'a> Iterator for CommandArgs<'a> {
    type Item = &'a OsStr;
    fn next(&mut self) -> Option<&'a OsStr> {
        self.iter.next().map(|os| &**os)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a> ExactSizeIterator for CommandArgs<'a> {
    fn len(&self) -> usize {
        self.iter.len()
    }
    fn is_empty(&self) -> bool {
        self.iter.is_empty()
    }
}

impl<'a> fmt::Debug for CommandArgs<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter.clone()).finish()
    }
}

pub type ChildPipe = crate::sys::pipe::Pipe;

pub fn read_output(
    out: ChildPipe,
    _stdout: &mut Vec<u8>,
    _err: ChildPipe,
    _stderr: &mut Vec<u8>,
) -> io::Result<()> {
    match out.diverge() {}
}

pub fn getpid() -> u32 {
    pal_process::getpid() as u32
}

fn cstring_from_os_str(value: &OsStr) -> io::Result<CString> {
    CString::new(value.as_encoded_bytes()).map_err(|_| {
        io::const_error!(io::ErrorKind::InvalidInput, "process argument contained NUL")
    })
}
