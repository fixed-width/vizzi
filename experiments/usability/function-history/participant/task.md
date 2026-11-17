# Read and review a callback change

Use the supplied lpc-rs source copy in VS Code. Start **Vizzi: Open Workspace** from
the Command Palette. You can refer to `controls.md`, use your usual editor tools,
ask for help, pause or stop. Tell the facilitator if you recognise the change.

Start with `Task::run_entry` in `src/interpreter/task/mod.rs`. Choose your own reading
path. Describe what you are trying to understand and anything that interrupts it.

First, answer this learning question:

> How does a callback entry reach execution through this function? What can cause it
> to stop or choose a different destination? Support your explanation with source
> locations, and say what remains uncertain.

Give that answer before moving on to the review question:

> Review the change to `Task::run_entry` from
> `2a6a23ad1be7e7892ce527fa77aab8eea8203353` to
> `457446c870ca3144c315ebcae9c78b7bc2b76514`, the supplied checkout's HEAD.
> What behaviour changes, and would you accept this part of the change, request changes
> or reserve judgement? Give before/after source evidence and identify any additional
> evidence you would need.

You do not need to review the entire commit. Distinguish the code you inspected from
assumptions about other parts of the system. Please mention when you leave Vizzi for
the editor, what question led you there, and whether earlier context was easy to recover.
