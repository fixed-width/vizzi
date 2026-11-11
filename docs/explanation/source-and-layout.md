# Live source, pinned views and saved layouts

A source file and a reading arrangement change for different reasons. Vizzi keeps them
separate so that an editor save can update the code without rearranging the task.

## A bubble refers to captured source

Vizzi reads saved disk files into a source observation. A bubble identifies a declaration
within that observation. It does not continuously read arbitrary bytes from an editor
buffer. This lets the source, its call sites and a displayed relationship refer to a
coherent version while another version is being prepared.

A live bubble can follow the declaration after a save. If its identity or reading context
cannot be recovered confidently, Vizzi retains the earlier view or resets the reading
position explicitly. A similarly named function is not a safe replacement by itself.

## Pinning holds source, not the whole task

A pinned bubble retains its captured version while other bubbles follow updates. The
older text can help explain an earlier branch or comparison, but it does not make an
earlier call current. Historical arrows preserve that distinction.

Retained captures are bounded. Pinning does not promise that the bytes will remain
available after every reload or server restart. Dependency captures have independent
versions and are not watched automatically in the initial implementation.

## Saving a task holds the arrangement

The task shelf saves which views belong together and how they are arranged. That is why
returning to a task can recover useful context without making a permanent source archive.
Saved locations and their notes are another separate record, intended to retain anchors
and a reader's interpretation. Neither mechanism edits the source repository.

## A repository update can affect an unchanged function

Caller/reference results depend on the query's captured repository, not just the target
file. Another file can gain a caller; a configuration change can alter analysis. A result
stack therefore needs refreshing when its repository observation changes, even when the
target function's text is identical.

Periodic checks that find the same observation do not themselves establish a change.
See [live-update controls](../how-to/follow-live-changes.md) and the
[storage reference](../reference/storage.md) for operational details.
