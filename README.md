# Pocket Kingdom — Rust + Macroquad

A touch-friendly territory strategy game written in Rust, built for phone browsers using WebAssembly.

## Play the published game
https://danielthrastarson.github.io/Tycoon-game/

**First-time deployment:** Go to [Settings → Pages](https://github.com/Danielthrastarson/Tycoon-game/settings/pages) and set **Build and deployment → Source → GitHub Actions**. Then visit [Actions](https://github.com/Danielthrastarson/Tycoon-game/actions) and use **Build and deploy Pocket Kingdom → Run workflow** if an automatic run did not deploy successfully. The link only works after a successful deploy.

On Android Chrome, open the game and choose **⋮ → Add to Home screen**.

## Edit from your phone
1. Open the repository: https://github.com/Danielthrastarson/Tycoon-game
2. Select **Code → Codespaces → Create codespace on main**.
3. In the terminal run: `python3 dev.py`
4. Open forwarded port **8000**, then play.
5. Edit `src/main.rs`; save to auto-build and refresh the preview.
6. Commit and push changes to `main` to publish them automatically.

## Gameplay
Tap an unowned tile to buy land for 10 gold. Each owned tile earns 5 gold per turn. You begin with 50 gold and one tile.

## Technology
- Rust + Macroquad
- WebAssembly for in-browser play
- GitHub Codespaces for editing on a phone
- GitHub Actions for automatic GitHub Pages deployment

The workflow produces a static website and does not require a server. GitHub account usage limits may apply to Codespaces or Actions.
