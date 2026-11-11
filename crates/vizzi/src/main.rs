//! Local HTTP host and socket-free diagnostic entry point.

use std::{
    io::{BufRead, Write},
    path::PathBuf,
    sync::Arc,
};
use vizzi::admission::Admission;
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
    let mut analysis_input_mib = 0usize;
    let mut source_spool_mib = 0usize;
    let mut policy = PolicyConfig::default();
    let mut investigation_file = None;
    let mut editor_code = PathBuf::from("code");
    let mut language_servers = vizzi::language_servers::Configuration::default();
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--rust-analyzer" => {
                language_servers.rust_analyzer = Some(PathBuf::from(
                    arguments
                        .next()
                        .ok_or("--rust-analyzer needs an executable")?,
                ))
            }
            "--pyright" => {
                language_servers.pyright = Some(PathBuf::from(
                    arguments.next().ok_or("--pyright needs an executable")?,
                ))
            }
            "--clangd" => {
                language_servers.clangd = Some(PathBuf::from(
                    arguments.next().ok_or("--clangd needs an executable")?,
                ))
            }
            "--python-path" => {
                language_servers.python_path = Some(PathBuf::from(
                    arguments
                        .next()
                        .ok_or("--python-path needs an interpreter")?,
                ))
            }
            "--compile-commands" => {
                language_servers.compilation_database = Some(PathBuf::from(
                    arguments.next().ok_or("--compile-commands needs a file")?,
                ))
            }
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
            "--investigation-file" => {
                investigation_file = Some(PathBuf::from(
                    arguments
                        .next()
                        .ok_or("--investigation-file needs a file")?,
                ))
            }
            "--editor-code" => {
                editor_code = PathBuf::from(
                    arguments
                        .next()
                        .ok_or("--editor-code needs an executable")?,
                )
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
            "--source-spool-mib" => {
                source_spool_mib = arguments
                    .next()
                    .ok_or("--source-spool-mib needs a disk capacity")?
                    .parse()?;
            }
            "--analysis-input-mib" => {
                analysis_input_mib = arguments
                    .next()
                    .ok_or("--analysis-input-mib needs an input-byte budget")?
                    .parse()?;
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
                    "vizzi --root DIR --cache DIR [--workers 1..32] [--port 7474] [--stdio | --once]\n  [--investigation-file FILE] [--editor-code EXECUTABLE] [--rust-analyzer EXECUTABLE]\n  [--pyright EXECUTABLE] [--python-path INTERPRETER]\n  [--clangd EXECUTABLE] [--compile-commands FILE]\n  [--syntax-cache-mib N] (0 disables, default 0; weighted retention, not a heap cap)\n  [--analysis-input-mib N] (0 disables; active input volume, not a heap cap)\n  [--source-spool-mib N] (0 disables; private session disk capacity)\n  [--c-family-root SCOPE] [--exclude-source SCOPE] [--syntax-language SCOPE=c|cpp] (repeatable)"
                );
                return Ok(());
            }
            _ => return Err(format!("unknown argument: {argument}").into()),
        }
    }
    if once && stdio {
        return Err("choose either --once or --stdio".into());
    }
    let mut directory = Directory::with_policy(
        &root.ok_or("--root is required")?,
        policy,
        Arc::new(Languages::builtin()?),
    )?;
    let cache =
        cache.ok_or("--cache is required; choose a directory outside the analyzed workspace")?;
    let cache = vizzi::cache::external_path(&cache, directory.root())?;
    let investigation_file = investigation_file
        .map(|path| vizzi::cache::external_path(&path, directory.root()))
        .transpose()?;
    if investigation_file
        .as_ref()
        .is_some_and(|path| path.starts_with(&cache))
    {
        return Err("Investigation file must be outside the analysis cache".into());
    }
    if source_spool_mib != 0 {
        let capacity = source_spool_mib
            .checked_mul(1024 * 1024)
            .ok_or("source spool capacity overflow")?;
        directory = directory.with_spool(Arc::new(vizzi::source_spool::SourceSpool::new(
            &cache, capacity,
        )?))?;
    }
    let definitions = if language_servers.is_empty() {
        if language_servers.python_path.is_some() || language_servers.compilation_database.is_some()
        {
            return Err("Language-server project options require a configured server".into());
        }
        None
    } else {
        Some(Arc::new(vizzi::language_servers::Servers::new(
            directory.root().to_owned(),
            cache.join("semantic"),
            language_servers,
        )?) as Arc<dyn vizzi::definitions::Provider>)
    };
    let mut engine = Engine::new(directory, Cache::new(cache), workers)?;
    if analysis_input_mib != 0 {
        let bytes = analysis_input_mib
            .checked_mul(1024 * 1024)
            .ok_or("analysis input budget overflow")?;
        engine = engine.with_admission(Arc::new(Admission::for_budget(bytes)?));
    }
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
    let host = Arc::new(if stdio {
        Host::start(engine)?
    } else {
        Host::prepare(engine)?
    });
    let application = host.application();
    let reviews = Arc::new(vizzi::review_application::ReviewApplication::start(
        host.clone(),
        vizzi::review_source::CaptureLimits::default(),
    )?);
    let investigation = Arc::new(vizzi::companion::Companion::new(
        host.clone(),
        investigation_file,
        Arc::new(vizzi::editor::VsCode::new(editor_code)),
    )?);
    let router = vizzi::server::router_with_definitions(
        host.clone(),
        reviews.clone(),
        investigation,
        definitions,
    );
    if stdio {
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout().lock();
        for line in stdin.lock().lines() {
            let line = line?;
            #[derive(serde::Deserialize)]
            struct Request {
                id: serde_json::Value,
                uri: String,
                method: Option<String>,
                body: Option<serde_json::Value>,
            }
            let response = match serde_json::from_str::<Request>(&line) {
                Ok(request) => {
                    match vizzi::server::diagnostic_request(
                        router.clone(),
                        request.id,
                        request.method.as_deref().unwrap_or("GET"),
                        &request.uri,
                        request.body,
                    )
                    .await
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
                reviews.shutdown();
            })
            .await?;
    }
    Ok(())
}
