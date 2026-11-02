//! Local HTTP host and socket-free diagnostic entry point.

use std::{
    io::{BufRead, Write},
    path::PathBuf,
    sync::Arc,
};
use vizzi::syntax_cache::SyntaxCache;
use vizzi::{cache::Cache, engine::Engine, host::Host, source::Directory};
use vizzi_core::{
    analysis::Languages,
    policy::{Assignment, FamilyLanguage, PolicyConfig},
};

#[tokio::main(worker_threads = 2)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut root = None;
    let mut cache = None;
    let mut workers = 4;
    let mut port = 7474;
    let mut stdio = false;
    let mut once = false;
    let mut syntax_cache_mib = 0usize;
    let mut policy = PolicyConfig::default();
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--root" => {
                root = Some(PathBuf::from(
                    arguments.next().ok_or("--root needs a directory")?,
                ))
            }
            "--cache" => {
                cache = Some(PathBuf::from(
                    arguments.next().ok_or("--cache needs a directory")?,
                ))
            }
            "--workers" => workers = arguments.next().ok_or("--workers needs a count")?.parse()?,
            "--port" => {
                port = arguments
                    .next()
                    .ok_or("--port needs a number")?
                    .parse::<u16>()?
            }
            "--stdio" => stdio = true,
            "--once" => once = true,
            "--syntax-cache-mib" => {
                syntax_cache_mib = arguments
                    .next()
                    .ok_or("--syntax-cache-mib needs a weight budget")?
                    .parse()?
            }
            "--c-family-root" => policy.c_family_roots.push(
                arguments
                    .next()
                    .ok_or("--c-family-root needs a relative scope")?,
            ),
            "--exclude-source" => policy.exclusions.push(
                arguments
                    .next()
                    .ok_or("--exclude-source needs a relative scope")?,
            ),
            "--syntax-language" => {
                let rule = arguments
                    .next()
                    .ok_or("--syntax-language needs SCOPE=c|cpp")?;
                let (scope, language) = rule
                    .rsplit_once('=')
                    .ok_or("--syntax-language needs SCOPE=c|cpp")?;
                let language = match language {
                    "c" => FamilyLanguage::C,
                    "cpp" => FamilyLanguage::Cpp,
                    _ => return Err("syntax language must be c or cpp".into()),
                };
                policy.assignments.push(Assignment {
                    scope: scope.into(),
                    language,
                });
            }
            "--help" | "-h" => {
                println!(
                    "vizzi --root DIR --cache DIR [--workers 1..32] [--port 7474] [--stdio | --once]\n  [--syntax-cache-mib N] (0 disables, default 0; weighted retention, not a heap cap)\n  [--c-family-root SCOPE] [--exclude-source SCOPE] [--syntax-language SCOPE=c|cpp] (repeatable)"
                );
                return Ok(());
            }
            _ => return Err(format!("unknown argument: {argument}").into()),
        }
    }
    if once && stdio {
        return Err("choose either --once or --stdio".into());
    }
    let directory = Directory::with_policy(
        &root.ok_or("--root is required")?,
        policy,
        Arc::new(Languages::builtin()?),
    )?;
    let cache =
        cache.ok_or("--cache is required; choose a directory outside the analyzed workspace")?;
    let cache = vizzi::cache::external_path(&cache, directory.root())?;
    let mut engine = Engine::new(directory, Cache::new(cache), workers)?;
    if syntax_cache_mib != 0 {
        let budget = syntax_cache_mib
            .checked_mul(1024 * 1024)
            .ok_or("syntax cache budget overflow")?;
        engine = engine.with_syntax_cache(Arc::new(SyntaxCache::for_budget(budget)?));
    }
    if once {
        println!(
            "{}",
            serde_json::to_string(&vizzi::host::one_shot(&engine)?.status())?
        );
        return Ok(());
    }
    let host = Host::start(engine)?;
    let application = host.application();
    let router = vizzi::server::router(application.clone());
    if stdio {
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout().lock();
        for line in stdin.lock().lines() {
            let line = line?;
            #[derive(serde::Deserialize)]
            struct Request {
                id: serde_json::Value,
                uri: String,
            }
            let response = match serde_json::from_str::<Request>(&line) {
                Ok(request) => {
                    match vizzi::server::diagnostic(router.clone(), request.id, &request.uri).await
                    {
                        Ok(response) => response,
                        Err(error) => vizzi::server::DiagnosticResponse {
                            id: serde_json::Value::Null,
                            status: 400,
                            content_type: "text/plain".into(),
                            body: error.to_string(),
                        },
                    }
                }
                Err(error) => vizzi::server::DiagnosticResponse {
                    id: serde_json::Value::Null,
                    status: 400,
                    content_type: "text/plain".into(),
                    body: error.to_string(),
                },
            };
            writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
            stdout.flush()?;
        }
    } else {
        let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
        eprintln!("Vizzi: http://{}", listener.local_addr()?);
        axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                let _ = tokio::signal::ctrl_c().await;
                application.close_updates();
            })
            .await?;
    }
    Ok(())
}
