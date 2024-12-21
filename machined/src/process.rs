use std::process::{Command, Stdio};
use tracing::debug;
use miette::{miette, IntoDiagnostic, Result};

pub fn run_capture_stdout<S: AsRef<str>>(
    args: &[S],
    env: Option<&[(S, S)]>,
) -> Result<String> {
    let args: Vec<&str> = args.iter().map(|s| s.as_ref()).collect();
    let env = build_env(env);
    let mut cmd = build_cmd(args.clone(), env);

    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let output = cmd.output().into_diagnostic()?;
    if output.status.success() {
        Ok(String::from_utf8(output.stdout).into_diagnostic()?)
    } else {
        Err(miette!(
            "exec {:?}: failed {:?}",
            &args,
            String::from_utf8(output.stderr).into_diagnostic()?
        ))
    }
}

fn build_env<S: AsRef<str>>(
    env: Option<&[(S, S)]>,
) -> Option<Vec<(&str, &str)>> {
    if let Some(env) = env {
        let env: Vec<(&str, &str)> =
            env.iter().map(|(k, v)| (k.as_ref(), v.as_ref())).collect();
        Some(env)
    } else {
        None
    }
}

fn build_cmd(args: Vec<&str>, env: Option<Vec<(&str, &str)>>) -> Command {
    let mut cmd = Command::new(&args[0]);
    cmd.env_remove("LANG");
    cmd.env_remove("LC_CTYPE");
    cmd.env_remove("LC_NUMERIC");
    cmd.env_remove("LC_TIME");
    cmd.env_remove("LC_COLLATE");
    cmd.env_remove("LC_MONETARY");
    cmd.env_remove("LC_MESSAGES");
    cmd.env_remove("LC_ALL");

    if args.len() > 1 {
        cmd.args(&args[1..]);
    }

    if let Some(env) = env {
        cmd.envs(env.clone());
        debug!(target: "illumos-rs", "exec: {:?} env={:?}", &args, &env);
    } else {
        debug!(target: "illumos-rs", "exec: {:?}", &args);
    }
    cmd
}
