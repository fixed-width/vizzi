# Call destinations, candidates and references

Vizzi shows static source relationships so that you can read related code together.
It does not trace a running program. The confidence of a destination matters separately
from where its bubble appears on the canvas.

## Why a call sometimes opens directly

Some Rust `self.method()` calls have a source-local binding. With a language server
configured, project-aware definition lookup resolves additional Rust, Python and C/C++ calls. A unique supported
destination opens directly; multiple semantic destinations still require a choice.

Without stronger evidence, Vizzi offers declarations matching a written call name. A
sole complete name candidate can open directly, but that does not turn it into a resolved
receiver type. Unconfigured Python uses this name-based navigation. Larger projects can
have many unrelated functions with the same name.

Conditional compilation, aliases, generated code, inheritance and dynamic dispatch can
make a plausible name match wrong. The containing declaration and source path help a
reader choose, but directory proximity does not prove which implementation runs.

## What the arrows mean

A solid current arrow has a supported source binding or language-server definition.
A dashed candidate arrow records the destination chosen from name candidates. Hover
an arrow for its evidence and source location. Neither is a runtime execution trace.

After a save, a relationship may become historical, unverified or removed. Its earlier
context can remain useful, but it must not be read as a proven call in the new source.
Module accents and task-group frames encode other relationships: language membership
and the reader's organisation, respectively.

## Callers and references

A caller invokes the target function. A reference is any use of it, including a call
or using it as a value:

```python
def process():
    pass


def example():
    process()           # a call and a reference
    callback = process  # a reference, not a call
```

**Find callers** filters to supported written call sites and identifies their enclosing
declarations. **Find references** can include other uses when the language provider
supports them. Several calls in one function can produce several result rows.

Without semantic reference support, both commands fall back to written-call name
candidates. They can therefore show identical lists even though reference is a broader
concept. Even semantic results are bounded by available captured source and analysis.
An empty result does not prove a function is unused.

[Language support](../reference/languages.md) lists these differences. The
[call-trail guide](../how-to/follow-a-call-trail.md) covers the controls, and
[configured language servers](../how-to/configure-language-servers.md) enable semantic navigation.
Contributors can consult the [Rust implementation details](../development/call-resolution.md).
