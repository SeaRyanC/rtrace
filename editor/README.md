# rtrace Scene Editor

The scene editor is an Electron desktop app for editing and previewing rtrace
JSON scenes.

## Start

From the repository root:

```bash
npm install
cargo build --release -p rtrace-cli
npm run editor:start
```

`editor:build` regenerates the scene JSON schema, compiles the Electron
processes, and copies the Monaco browser assets. The app looks for the
renderer at `target/release/rtrace-cli` or `target/release/rtrace-cli.exe`.
Set `RTRACE_CLI` to use a different renderer binary.

## Workflow

- Edit JSON in the left Monaco pane. The editor validates the scene against
  the generated schema and provides completion and hover descriptions for
  cameras, objects, materials, textures, lights, and scene settings.
- The right pane renders automatically after a short editing pause, or use
  **Render** / `F5` for an explicit render.
- Choose **Rasterizer** for a fast preview without ray-traced shadows and
  reflections, or **Raytracer** for the full renderer. `Ctrl+1` and `Ctrl+2`
  switch modes.
- Renders start at a small diagonal resolution and double until the preview
  viewport size is reached. Each completed pass replaces the image in the
  preview pane.
- Press `Esc` or choose **Render > Cancel Render** to terminate the active
  renderer process. Relative STL paths are resolved relative to the active
  scene file.

## File operations

`Ctrl+N`, `Ctrl+O`, `Ctrl+S`, and `Ctrl+Shift+S` create, open, save, and save
scenes as expected. `Shift+Alt+F` formats the current JSON document using
Monaco's built-in formatter.

The editor intentionally keeps the Rust renderer in a separate process. This
keeps the UI responsive and makes cancellation reliable while retaining the
same CLI behavior used outside the editor.
