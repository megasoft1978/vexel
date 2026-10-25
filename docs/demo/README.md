# Demo media

- `docs/media/web_demo.mp4`, `web_demo.webm`: 34 s highlight cut (1280x720, silent, burned-in captions), used on the website and README.
- `docs/media/poster.jpg`, `hero_6s.gif`: poster frame and 6 s README loop.
- `docs/demo/live-session.mp4`: recorded Codex session driving Vexel over MCP.
- The 69 s master (`master.mp4`, 1920x1080), `social_square.mp4` and `whatsapp.mp4` are rendered locally from the Remotion project and are deliberately not committed (size). Re-render them from the Remotion project if you need them.
- Speed figures on the cards come from `cargo run -p vexel-dev -- bench` (walk_home 1450 -> 37 ms per frame; stroke-heavy 1000 shapes at 2048 px 24.8 -> 2.7 s; mixed 5000 shapes 2245 -> 585 ms; exports 1.8-2.9x).
