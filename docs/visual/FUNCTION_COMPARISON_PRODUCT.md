# Compare adjacent function versions

For readers asking what changed in one function without losing their spatial call trail.
Use the existing bubble design and theme; no separate mock is needed.

## Behaviour

1. **Compare with previous version** is available on current repository functions and
   retained historical versions. It compares the displayed declaration with the next
   distinct older version on the existing frozen first-parent history. Saved uncommitted
   code can compare with HEAD. Existing missing/ambiguous declaration and depth boundaries
   remain explicit; failed preparation leaves the working set unchanged.
2. The chosen bubble becomes the pinned **After** view and a new pinned **Before** bubble
   opens beside it. Existing view identities, rectangles, task memberships and other
   bubbles remain intact. The new bubble joins its parent's task group. Reveal the pair
   at the existing zoom, without rearranging earlier bubbles or opening Review changes.
3. Both views identify their exact revision. A working-tree side is labelled as a captured
   saved version. Both captures remain fixed through saved edits, history navigation in
   another view and changes to Git refs. Comparison does not change files or check out code.
4. Highlight removed/replaced lines in Before and added/replacement lines in After. Keep
   original source text, line numbers, styling, reflow and copying. Labels identify each
   side independently of colour. Comparison highlights describe this frozen pair; they
   do not establish a call relationship between the bubbles.
5. Compute changes from complete captured declarations, including text outside the loaded
   section. Ignore unrelated declarations and line movement elsewhere in the file. Open
   at the first changed block. Keep as much preceding context as fits while revealing
   the first changed line or its insertion/deletion anchor. Report the jump once the
   target is visible in **After**. **Previous changed block** and **Next changed block**
   reveal corresponding locations in both visible sides. Insertion and deletion anchors
   remain valid when one side has no changed lines. Each side scrolls through its own
   captured function.
6. Each bubble can close independently. Remaining highlights still refer to the labelled
   frozen pair. **Return to live** resumes that bubble's declaration tracking, retaining
   its actual comparison capture if the current declaration is missing. **Older revision
   (exit diff)** steps back through function history on a committed side and ends its
   comparison presentation. The other bubble keeps its comparison.
7. Calls can follow declarations within the exact captured file on each comparison side;
   see [captured call exploration](CAPTURED_CALLS_PRODUCT.md). Callers, references, Save and
   Editor remain current-source actions. Existing call arrows become historical. Other
   live bubbles update normally.
8. Comparison metadata, reading state and layout survive task saves and reload. Captures
   are session-owned, bounded and may expire; expiry is explicit and never substitutes
   current source. Return to live remains available. Loaded text stays readable if later
   paging fails. The feature does not promise a permanent source archive.
9. Respect the 24-bubble task limit. Loading, invalid replies, quota refusal and cancellation
   cannot leave half a pair. Closing a view, switching tasks, returning live or newer source
   intent invalidates older preparation. Source selection defers source replacement.
10. Controls use ordinary keyboard-focusable buttons and announced status. Minimum-size
    bubbles retain usable controls. An unchanged or limited diff is distinct from missing
    or failed analysis; a resource failure never appears as an empty successful comparison.
