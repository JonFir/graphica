# Path web viewer

Status: planned npm library. There is no package manifest, published package name,
renderer, or bundled framework dependency yet.

The viewer renders decision trees, behavior trees, and activity diagrams inside a
host application. It supports zoom, pan, node inspection, and highlighting of
active activations, completed visits, selected transitions, and waiting joins.
Repeated visits to the same node can be inspected separately by activation ID.

## Intended public interface

The draft framework-independent interface is:

```ts
type ViewerInput = { diagram: Diagram; run?: RunView };

declare function createPathViewer(
  container: HTMLElement,
  input: ViewerInput,
  options?: {
    onNodeSelect?: (selection: { nodeId: string; activationId?: string }) => void;
  },
): {
  update(input: ViewerInput): void;
  fit(): void;
  destroy(): void;
};
```

`Diagram` is the parsed model from the [format](../format/README.md). `RunView` is
the public projection in [runs](../docs/runs.md). This excerpt describes an API;
it is not a currently exported TypeScript declaration.

The host parses and validates input through its chosen integration and supplies a
matching diagram/view pair, normally from CLI `run status`. Do not bind to the
CLI's filesystem envelope or use its private interpreter state. For a running
diagram, use the frozen diagram returned with the view, not a newly edited source.

The viewer rejects an incompatible format/run version or mismatched diagram ID
and kind. It does not silently reinterpret unsupported node types. `update`
replaces the displayed data; an invalid update leaves the previous view intact.
`destroy` releases listeners and renderer resources.

Selection is a local inspection event. It never chooses a diagram branch, reports
a leaf status, edits diagram data, or mutates a run. The viewer does not perform
network requests or maintain execution state. Treat all supplied text as text,
without HTML execution. `fit` and viewport changes are presentation-only.

## Acceptance targets

- Render every node type from the shared examples with readable labels and edges.
- Distinguish unvisited nodes, active leaves, active behavior ancestors, repeated
  visits, and branches waiting at a join.
- Replace a run snapshot without advancing it; support diagram-only display.
- Expose keyboard-accessible node inspection and readable status information.
- Mount, update, and destroy without leaking event handlers or changing input data.

The renderer and framework adapters will be selected during implementation.
Editing, execution, storage synchronization, and server hosting are outside this
viewer's initial scope.
