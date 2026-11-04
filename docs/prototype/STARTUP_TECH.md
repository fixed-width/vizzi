# Task-driven startup: implementation contract

This reference implements [the startup behaviour](STARTUP_PRODUCT.md) using the existing
host, application, comparison and renderer interfaces. It follows commit `b116f4b`.

`Host::prepare` and its explicit-options variant construct the same engine/application
and owned coordinator without live IO. A bounded activation signal gates watcher and
maintenance initialization. `Host::activate` merges a full initial request under the
existing queue lock; duplicate activation cannot supersede in-flight work. An explicit
retry is admitted only after initial failure with no pending attempt. `Host::request`
also activates before accepting a native hint. Drop wakes and joins a dormant worker.

Eager constructors preserve their synchronous initial capture and error propagation.
The shared ongoing coordinator performs the deferred first capture using its existing
foreground path, then installs directory watches and reconciles newly registered paths.
Do not copy the recovery/indexing engine into a new startup module or hold a metadata
lock during watcher setup, source IO or analysis.

Application readiness is derived from publication metadata: snapshot present -> ready;
otherwise pending -> initializing; otherwise error -> failed; otherwise not started.
It is an additive field in status schema 1 and latest-state notifications. Existing
pending/error/source identities retain their meaning. Client validation accepts older
status fixtures without this additive field and validates it when present.

The normal HTTP executable prepares a deferred host. Its host-aware router adds
`POST /api/workspace/start`, requiring JSON `{}`, rejecting unknown fields and bounding
the body. The handler performs only activation/metadata and returns 202 with status,
no-store and nosniff. Existing application-only/eager router constructors remain usable;
they do not acquire hidden native engine dependencies.

Exploration subscribes as before and automatically activates on the first unstarted
status. Retry uses the same operation. Ready application-only hosts need no activation
request. Initial messages distinguish waiting/indexing/failure; no message claims a
previous capture when none exists. Review never calls the live activation operation.
Finite stdio and `--once` keep eager defaults for compatibility.

Controlled blocking/failing language adapters test the interface without timing-based
speed assertions. Test metadata and query availability while the first live parser is
held, scope-only review before activation, duplicate/retry activation, watch recovery,
shutdown and unchanged aggregate worker accounting. Browser and measurement harnesses
must record null initial live measurements as unstarted, not zero work or missing data.
