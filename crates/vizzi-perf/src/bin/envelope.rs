//! Observe a Linux child tree, requiring cgroup enforcement unless explicitly disabled.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    {
        let mut arguments = std::env::args().skip(1);
        let first = arguments
            .next()
            .ok_or("use vizzi-envelope [--observe-only] OUTPUT -- COMMAND [ARGS]")?;
        let observe_only = first == "--observe-only";
        let output = if observe_only {
            arguments.next().ok_or("output required")?
        } else {
            first
        };
        if arguments.next().as_deref() != Some("--") {
            return Err("expected -- before command".into());
        }
        if !vizzi_perf::envelope::run(
            std::path::Path::new(&output),
            observe_only,
            &arguments.collect::<Vec<_>>(),
        )? {
            std::process::exit(1);
        }
        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    {
        Err("vizzi-envelope requires Linux procfs and cgroup v2".into())
    }
}
